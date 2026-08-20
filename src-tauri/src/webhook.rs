use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use hyper::{Request, Response, StatusCode};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use http_body_util::{BodyExt, Full};
use bytes::Bytes;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};
use ring::hmac;
use base64::Engine;

use crate::models::{WebhookEndpoint, WebhookDelivery, WebhookTriggerRequest, WebhookForwardResult};
use crate::db;

/// Calculate HMAC signatures for webhook providers
pub fn calculate_hmac_signature(secret: &str, body: &str, provider: &str) -> (String, String) {
    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    
    match provider.to_lowercase().as_str() {
        "github" => {
            let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
            let tag = hmac::sign(&key, body.as_bytes());
            let hex_sig = hex::encode(tag.as_ref());
            ("X-Hub-Signature-256".to_string(), format!("sha256={}", hex_sig))
        }
        "stripe" => {
            let payload_to_sign = format!("{}.{}", now_ts, body);
            let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
            let tag = hmac::sign(&key, payload_to_sign.as_bytes());
            let hex_sig = hex::encode(tag.as_ref());
            ("Stripe-Signature".to_string(), format!("t={},v1={}", now_ts, hex_sig))
        }
        "shopify" => {
            let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
            let tag = hmac::sign(&key, body.as_bytes());
            let b64_sig = base64::engine::general_purpose::STANDARD.encode(tag.as_ref());
            ("X-Shopify-Hmac-Sha256".to_string(), b64_sig)
        }
        "slack" => {
            let payload_to_sign = format!("v0:{}:{}", now_ts, body);
            let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
            let tag = hmac::sign(&key, payload_to_sign.as_bytes());
            let hex_sig = hex::encode(tag.as_ref());
            ("X-Slack-Signature".to_string(), format!("v0={}", hex_sig))
        }
        _ => {
            // Default HMAC SHA256
            let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
            let tag = hmac::sign(&key, body.as_bytes());
            let hex_sig = hex::encode(tag.as_ref());
            ("X-Signature-256".to_string(), hex_sig)
        }
    }
}

pub async fn start_webhook_server(
    app_handle: AppHandle,
    port: u16,
) -> Result<tokio::sync::oneshot::Sender<()>, String> {
    let addr: SocketAddr = format!("0.0.0.0:{}", port)
        .parse()
        .map_err(|e: std::net::AddrParseError| format!("Invalid address: {}", e))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind webhook server to {}: {}", addr, e))?;

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    tauri::async_runtime::spawn(async move {
        tokio::select! {
            _ = async {
                loop {
                    match listener.accept().await {
                        Ok((stream, client_addr)) => {
                            let io = TokioIo::new(stream);
                            let app_handle_clone = app_handle.clone();
                            tokio::task::spawn(async move {
                                let service = service_fn(move |req: Request<Incoming>| {
                                    let app = app_handle_clone.clone();
                                    async move {
                                        handle_incoming_webhook(app, req, client_addr.ip().to_string()).await
                                    }
                                });
                                if let Err(err) = hyper::server::conn::http1::Builder::new()
                                    .serve_connection(io, service)
                                    .await
                                {
                                    eprintln!("Error serving webhook connection: {:?}", err);
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("Failed to accept webhook connection: {:?}", e);
                        }
                    }
                }
            } => {}
            _ = rx => {
                println!("Stopping webhook server on port {}", port);
            }
        }
    });

    Ok(tx)
}

async fn handle_incoming_webhook(
    app_handle: AppHandle,
    req: Request<Incoming>,
    client_ip: String,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let method = req.method().to_string();
    let uri = req.uri();
    let path = uri.path().to_string();
    let query_params = uri.query().unwrap_or("").to_string();

    // Extract headers into HashMap & JSON
    let mut headers_map = HashMap::new();
    for (k, v) in req.headers() {
        if let Ok(val_str) = v.to_str() {
            headers_map.insert(k.as_str().to_string(), val_str.to_string());
        }
    }
    let headers_json = serde_json::to_string(&headers_map).unwrap_or_else(|_| "{}".to_string());

    // Resolve real client IP: CF-Connecting-IP > X-Real-IP > X-Forwarded-For > socket IP
    let resolved_ip = headers_map.get("cf-connecting-ip")
        .or_else(|| headers_map.get("x-real-ip"))
        .or_else(|| headers_map.get("x-forwarded-for"))
        .map(|v| {
            // X-Forwarded-For may be a comma-separated list; take the first (leftmost = original client)
            v.split(',').next().unwrap_or(v.trim()).trim().to_string()
        })
        .unwrap_or_else(|| client_ip.clone());

    // Read body bytes
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => Bytes::new(),
    };
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

    // Match path with endpoint in DB
    let db_path = db::get_db_path(&app_handle);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    let delivery_id = Uuid::new_v4().to_string();

    let mut matched_endpoint: Option<WebhookEndpoint> = None;

    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
        let mut stmt = conn.prepare(
            "SELECT id, name, path_slug, mock_status, mock_headers, mock_body, auto_forward_url, is_active, created_at FROM webhook_endpoints WHERE is_active = 1"
        ).ok();

        if let Some(ref mut stmt) = stmt {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok(WebhookEndpoint {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path_slug: row.get(2)?,
                    mock_status: row.get(3)?,
                    mock_headers: row.get(4)?,
                    mock_body: row.get(5)?,
                    auto_forward_url: row.get(6)?,
                    is_active: row.get::<_, i32>(7)? != 0,
                    created_at: row.get(8)?,
                })
            }) {
                for ep in rows.flatten() {
                    let slug_normalized = if ep.path_slug.starts_with('/') {
                        ep.path_slug.clone()
                    } else {
                        format!("/{}", ep.path_slug)
                    };
                    if path == slug_normalized || path.starts_with(&format!("{}/", slug_normalized)) {
                        matched_endpoint = Some(ep);
                        break;
                    }
                }
            }
        }
    }

    let endpoint_id = matched_endpoint.as_ref().map(|e| e.id.clone());

    // Insert delivery into database
    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
        let _ = conn.execute(
            "INSERT INTO webhook_deliveries (id, endpoint_id, method, path, headers, query_params, body, client_ip, forwarded, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?)",
            rusqlite::params![
                delivery_id,
                endpoint_id,
                method,
                path,
                headers_json,
                query_params,
                body_str,
                resolved_ip,
                now
            ]
        );
    }

    let delivery = WebhookDelivery {
        id: delivery_id.clone(),
        endpoint_id: endpoint_id.clone(),
        method: method.clone(),
        path: path.clone(),
        headers: headers_json.clone(),
        query_params: query_params.clone(),
        body: body_str.clone(),
        client_ip: Some(resolved_ip),
        forwarded: false,
        forward_status: None,
        forward_response_body: None,
        timestamp: now as i64,
    };

    // Emit event to frontend
    let _ = app_handle.emit("webhook_captured", &delivery);

    // Auto-forward if configured
    if let Some(ref ep) = matched_endpoint {
        if let Some(ref forward_url) = ep.auto_forward_url {
            if !forward_url.trim().is_empty() {
                let app_handle_clone = app_handle.clone();
                let delivery_id_clone = delivery_id.clone();
                let target_url = forward_url.clone();
                let method_clone = method.clone();
                let headers_map_clone = headers_map.clone();
                let body_str_clone = body_str.clone();

                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    let mut req_builder = match method_clone.to_uppercase().as_str() {
                        "GET" => client.get(&target_url),
                        "PUT" => client.put(&target_url),
                        "DELETE" => client.delete(&target_url),
                        "PATCH" => client.patch(&target_url),
                        _ => client.post(&target_url),
                    };

                    for (k, v) in headers_map_clone {
                        if k.to_lowercase() != "host" && k.to_lowercase() != "content-length" {
                            req_builder = req_builder.header(&k, &v);
                        }
                    }
                    req_builder = req_builder.body(body_str_clone);

                    match req_builder.send().await {
                        Ok(resp) => {
                            let status = resp.status().as_u16() as i32;
                            let res_body = resp.text().await.unwrap_or_default();
                            let db_p = db::get_db_path(&app_handle_clone);
                            if let Ok(conn) = rusqlite::Connection::open(db_p) {
                                let _ = conn.execute(
                                    "UPDATE webhook_deliveries SET forwarded = 1, forward_status = ?, forward_response_body = ? WHERE id = ?",
                                    rusqlite::params![status, res_body, delivery_id_clone]
                                );
                            }
                        }
                        Err(e) => {
                            let db_p = db::get_db_path(&app_handle_clone);
                            if let Ok(conn) = rusqlite::Connection::open(db_p) {
                                let _ = conn.execute(
                                    "UPDATE webhook_deliveries SET forwarded = 1, forward_status = 500, forward_response_body = ? WHERE id = ?",
                                    rusqlite::params![e.to_string(), delivery_id_clone]
                                );
                            }
                        }
                    }
                });
            }
        }
    }

    // Build response
    let (status_code, resp_body, resp_headers) = if let Some(ref ep) = matched_endpoint {
        (
            StatusCode::from_u16(ep.mock_status as u16).unwrap_or(StatusCode::OK),
            ep.mock_body.clone(),
            ep.mock_headers.clone(),
        )
    } else {
        (
            StatusCode::OK,
            format!(r#"{{"status":"captured","delivery_id":"{}"}}"#, delivery_id),
            r#"{"Content-Type":"application/json"}"#.to_string(),
        )
    };

    let mut response_builder = Response::builder().status(status_code);

    if let Ok(parsed_headers) = serde_json::from_str::<HashMap<String, String>>(&resp_headers) {
        for (k, v) in parsed_headers {
            response_builder = response_builder.header(k, v);
        }
    } else {
        response_builder = response_builder.header("Content-Type", "application/json");
    }

    let response = response_builder
        .body(Full::new(Bytes::from(resp_body)))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("OK"))));

    Ok(response)
}

// Database & Tauri Command Helpers

pub fn get_endpoints_db(app_handle: &AppHandle) -> Result<Vec<WebhookEndpoint>, String> {
    let db_path = db::get_db_path(app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, name, path_slug, mock_status, mock_headers, mock_body, auto_forward_url, is_active, created_at FROM webhook_endpoints ORDER BY created_at DESC"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        Ok(WebhookEndpoint {
            id: row.get(0)?,
            name: row.get(1)?,
            path_slug: row.get(2)?,
            mock_status: row.get(3)?,
            mock_headers: row.get(4)?,
            mock_body: row.get(5)?,
            auto_forward_url: row.get(6)?,
            is_active: row.get::<_, i32>(7)? != 0,
            created_at: row.get(8)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows.flatten() {
        list.push(r);
    }
    Ok(list)
}

pub fn create_endpoint_db(app_handle: &AppHandle, ep: WebhookEndpoint) -> Result<WebhookEndpoint, String> {
    let db_path = db::get_db_path(app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    let id = if ep.id.trim().is_empty() { Uuid::new_v4().to_string() } else { ep.id };

    conn.execute(
        "INSERT INTO webhook_endpoints (id, name, path_slug, mock_status, mock_headers, mock_body, auto_forward_url, is_active, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            id,
            ep.name,
            ep.path_slug,
            ep.mock_status,
            ep.mock_headers,
            ep.mock_body,
            ep.auto_forward_url,
            if ep.is_active { 1 } else { 0 },
            now
        ]
    ).map_err(|e| e.to_string())?;

    Ok(WebhookEndpoint {
        id,
        created_at: now,
        ..ep
    })
}

pub fn update_endpoint_db(app_handle: &AppHandle, ep: WebhookEndpoint) -> Result<(), String> {
    let db_path = db::get_db_path(app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE webhook_endpoints SET name = ?, path_slug = ?, mock_status = ?, mock_headers = ?, mock_body = ?, auto_forward_url = ?, is_active = ? WHERE id = ?",
        rusqlite::params![
            ep.name,
            ep.path_slug,
            ep.mock_status,
            ep.mock_headers,
            ep.mock_body,
            ep.auto_forward_url,
            if ep.is_active { 1 } else { 0 },
            ep.id
        ]
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_endpoint_db(app_handle: &AppHandle, id: String) -> Result<(), String> {
    let db_path = db::get_db_path(app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM webhook_endpoints WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_deliveries_db(app_handle: &AppHandle, limit: i32) -> Result<Vec<WebhookDelivery>, String> {
    let db_path = db::get_db_path(app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, endpoint_id, method, path, headers, query_params, body, client_ip, forwarded, forward_status, forward_response_body, timestamp FROM webhook_deliveries ORDER BY timestamp DESC LIMIT ?"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([limit], |row| {
        let f_val: i32 = row.get(8)?;
        Ok(WebhookDelivery {
            id: row.get(0)?,
            endpoint_id: row.get(1)?,
            method: row.get(2)?,
            path: row.get(3)?,
            headers: row.get(4)?,
            query_params: row.get(5)?,
            body: row.get(6)?,
            client_ip: row.get(7)?,
            forwarded: f_val != 0,
            forward_status: row.get(9)?,
            forward_response_body: row.get(10)?,
            timestamp: row.get(11)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows.flatten() {
        list.push(r);
    }
    Ok(list)
}

pub fn clear_deliveries_db(app_handle: &AppHandle) -> Result<(), String> {
    let db_path = db::get_db_path(app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM webhook_deliveries", []).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_delivery_db(app_handle: &AppHandle, id: String) -> Result<(), String> {
    let db_path = db::get_db_path(app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM webhook_deliveries WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn forward_delivery_http(
    app_handle: &AppHandle,
    delivery_id: String,
    target_url: String,
) -> Result<WebhookForwardResult, String> {
    let db_path = db::get_db_path(app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;

    let delivery: WebhookDelivery = conn.query_row(
        "SELECT id, endpoint_id, method, path, headers, query_params, body, client_ip, forwarded, forward_status, forward_response_body, timestamp FROM webhook_deliveries WHERE id = ?",
        [&delivery_id],
        |row| {
            let f_val: i32 = row.get(8)?;
            Ok(WebhookDelivery {
                id: row.get(0)?,
                endpoint_id: row.get(1)?,
                method: row.get(2)?,
                path: row.get(3)?,
                headers: row.get(4)?,
                query_params: row.get(5)?,
                body: row.get(6)?,
                client_ip: row.get(7)?,
                forwarded: f_val != 0,
                forward_status: row.get(9)?,
                forward_response_body: row.get(10)?,
                timestamp: row.get(11)?,
            })
        }
    ).map_err(|e| format!("Delivery not found: {}", e))?;

    let client = reqwest::Client::new();
    let mut req_builder = match delivery.method.to_uppercase().as_str() {
        "GET" => client.get(&target_url),
        "PUT" => client.put(&target_url),
        "DELETE" => client.delete(&target_url),
        "PATCH" => client.patch(&target_url),
        _ => client.post(&target_url),
    };

    if let Ok(headers_map) = serde_json::from_str::<HashMap<String, String>>(&delivery.headers) {
        for (k, v) in headers_map {
            if k.to_lowercase() != "host" && k.to_lowercase() != "content-length" {
                req_builder = req_builder.header(&k, &v);
            }
        }
    }
    req_builder = req_builder.body(delivery.body);

    match req_builder.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16() as i32;
            let res_body = resp.text().await.unwrap_or_default();
            let _ = conn.execute(
                "UPDATE webhook_deliveries SET forwarded = 1, forward_status = ?, forward_response_body = ? WHERE id = ?",
                rusqlite::params![status, res_body, delivery_id]
            );
            Ok(WebhookForwardResult {
                success: true,
                status_code: Some(status),
                response_body: Some(res_body),
                error: None,
            })
        }
        Err(e) => {
            let err_msg = e.to_string();
            let _ = conn.execute(
                "UPDATE webhook_deliveries SET forwarded = 1, forward_status = 500, forward_response_body = ? WHERE id = ?",
                rusqlite::params![err_msg, delivery_id]
            );
            Ok(WebhookForwardResult {
                success: false,
                status_code: Some(500),
                response_body: None,
                error: Some(err_msg),
            })
        }
    }
}

pub async fn trigger_webhook_request(req: WebhookTriggerRequest) -> Result<WebhookForwardResult, String> {
    let client = reqwest::Client::new();
    let mut req_builder = match req.method.to_uppercase().as_str() {
        "GET" => client.get(&req.url),
        "PUT" => client.put(&req.url),
        "DELETE" => client.delete(&req.url),
        "PATCH" => client.patch(&req.url),
        _ => client.post(&req.url),
    };

    let mut headers = req.headers.clone();

    // Auto-calculate HMAC signature if secret & provider preset are supplied
    if let Some(secret) = req.signature_secret {
        if !secret.trim().is_empty() {
            let provider = req.provider_preset.as_deref().unwrap_or("custom");
            let (header_name, header_val) = calculate_hmac_signature(&secret, &req.body, provider);
            headers.insert(header_name, header_val);
        }
    }

    for (k, v) in headers {
        req_builder = req_builder.header(&k, &v);
    }
    req_builder = req_builder.body(req.body);

    match req_builder.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16() as i32;
            let body = resp.text().await.unwrap_or_default();
            Ok(WebhookForwardResult {
                success: true,
                status_code: Some(status),
                response_body: Some(body),
                error: None,
            })
        }
        Err(e) => Ok(WebhookForwardResult {
            success: false,
            status_code: None,
            response_body: None,
            error: Some(e.to_string()),
        }),
    }
}
