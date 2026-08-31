pub mod mitm;
pub mod intercept;
pub mod rules;
pub mod rewrite;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rcgen::{CertificateParams, DnType, KeyPair};
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use tauri::{AppHandle, Emitter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::ca::CA;
use crate::proxy::intercept::handle_intercept_hook;
use crate::state::{AppState, HistoryEntry, InterceptAction, InterceptPhase, TrafficCapturedEvent};

pub async fn start_proxy_server(
    app_handle: AppHandle,
    state: Arc<AppState>,
    ca: Arc<CA>,
    addr_str: String,
    stop_rx: oneshot::Receiver<()>,
) -> Result<(), String> {
    let addr: SocketAddr = addr_str
        .parse()
        .map_err(|e| format!("Invalid bind address '{}': {}", addr_str, e))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind proxy listener to {}: {}", addr, e))?;

    println!("Proxy listener active on {}", addr);

    tokio::select! {
        _ = async {
            loop {
                match listener.accept().await {
                    Ok((stream, client_addr)) => {
                        let app_handle_clone = app_handle.clone();
                        let state_clone = Arc::clone(&state);
                        let ca_clone = Arc::clone(&ca);
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = handle_connection(stream, state_clone, ca_clone, app_handle_clone, client_addr).await {
                                eprintln!("Error handling proxy connection from {}: {}", client_addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Proxy accept error: {}", e);
                    }
                }
            }
        } => {}
        _ = stop_rx => {
            println!("Proxy server on {} stopped gracefully", addr);
        }
    }

    Ok(())
}

async fn handle_connection(
    stream: TcpStream,
    state: Arc<AppState>,
    ca: Arc<CA>,
    app_handle: AppHandle,
    _client_addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let io = TokioIo::new(stream);

    let state_clone = Arc::clone(&state);
    let ca_clone = Arc::clone(&ca);
    let app_handle_clone = app_handle.clone();

    let service = service_fn(move |req| {
        let state = Arc::clone(&state_clone);
        let ca = Arc::clone(&ca_clone);
        let app_handle = app_handle_clone.clone();
        async move {
            if req.method() == Method::CONNECT {
                let host = req.uri().host().unwrap_or_default().to_string();
                let port = req.uri().port_u16().unwrap_or(443);

                tokio::spawn(async move {
                    if let Err(e) = handle_connect(req, state, ca, app_handle, host, port).await {
                        eprintln!("Error in CONNECT: {}", e);
                    }
                });

                Ok(Response::new(Full::new(Bytes::new())))
            } else {
                handle_http(req, state, app_handle).await
            }
        }
    });

    if let Err(_err) = hyper::server::conn::http1::Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .serve_connection(io, service)
        .with_upgrades()
        .await
    {
    }

    Ok(())
}

async fn handle_connect(
    req: Request<Incoming>,
    state: Arc<AppState>,
    ca: Arc<CA>,
    app_handle: AppHandle,
    host: String,
    _port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let upgraded = hyper::upgrade::on(req).await?;
    let upgraded_io = TokioIo::new(upgraded);

    let mut params = CertificateParams::new(vec![host.clone()]).map_err(|e| format!("{}", e))?;
    params.distinguished_name.push(DnType::CommonName, host.clone());
    params.key_usages.push(rcgen::KeyUsagePurpose::DigitalSignature);
    params.key_usages.push(rcgen::KeyUsagePurpose::KeyEncipherment);
    params.extended_key_usages.push(rcgen::ExtendedKeyUsagePurpose::ServerAuth);

    let now = SystemTime::now();
    params.not_before = time::OffsetDateTime::from(now - std::time::Duration::from_secs(86400));
    params.not_after = time::OffsetDateTime::from(now + std::time::Duration::from_secs(86400 * 365));

    let cert_key_pair = KeyPair::generate().map_err(|e| format!("{}", e))?;
    let cert = params.signed_by(&cert_key_pair, &ca.cert, &ca.key_pair).map_err(|e| format!("{}", e))?;

    let cert_der = cert.der().clone();
    let key_der = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(cert_key_pair.serialize_der()));

    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)?;

    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
    let tls_stream = match acceptor.accept(upgraded_io).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("TLS accept error for {}: {}", host, e);
            return Err(e.into());
        }
    };

    let io = TokioIo::new(tls_stream);

    let host_for_service = host.clone();
    let service = service_fn(move |mut req| {
        let state = Arc::clone(&state);
        let app_handle = app_handle.clone();
        let host = host_for_service.clone();
        async move {
            let uri = format!("https://{}{}", host, req.uri());
            *req.uri_mut() = uri.parse().unwrap();
            handle_http(req, state, app_handle).await
        }
    });

    if let Err(err) = hyper::server::conn::http1::Builder::new()
        .serve_connection(io, service)
        .with_upgrades()
        .await
    {
        eprintln!("Error serving TLS connection for {}: {}", host, err);
    }

    Ok(())
}

pub fn is_websocket_upgrade(headers: &hyper::HeaderMap) -> bool {
    let connection_upgrade = headers.get(hyper::header::CONNECTION)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_lowercase().contains("upgrade"))
        .unwrap_or(false);
    let upgrade_websocket = headers.get(hyper::header::UPGRADE)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_lowercase().contains("websocket"))
        .unwrap_or(false);
    connection_upgrade && upgrade_websocket
}

async fn handle_http(
    req: Request<Incoming>,
    state: Arc<AppState>,
    app_handle: AppHandle,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_or_http()
        .enable_http1()
        .build();
    let ws_mitm_enabled = { state.proxy_config.read().await.ws_mitm_enabled };

    if is_websocket_upgrade(req.headers()) {
        let method = req.method().clone();
        let url = req.uri().to_string();
        let host = req.uri().host().unwrap_or_default().to_string();
        let path = parse_path_from_url(&url);
        let request_headers = headers_to_vec(req.headers());
        let subprotocol = req.headers().get("sec-websocket-protocol").and_then(|h| h.to_str().ok()).map(|s| s.to_string());
        let entry_id = Uuid::new_v4().to_string();

        let mut new_req = Request::builder()
            .method(method.clone())
            .uri(url.clone());

        let mut headers_cleaned = request_headers.clone();
        headers_cleaned.retain(|(k, _)| k != "content-length" && k != "transfer-encoding");
        for (k, v) in headers_cleaned.iter() {
            new_req = new_req.header(k, v);
        }
        let new_req = new_req.body(Full::new(Bytes::new())).unwrap();

        match client.request(new_req).await {
            Ok(res) => {
                let status = res.status().as_u16();
                if status == 101 {
                    let response_headers = headers_to_vec(res.headers());
                    let mut client_res_builder = Response::builder().status(StatusCode::SWITCHING_PROTOCOLS);
                    for (k, v) in response_headers.iter() {
                        client_res_builder = client_res_builder.header(k, v);
                    }
                    let client_res = client_res_builder.body(Full::new(Bytes::new())).unwrap();

                    log_and_emit_history(
                        &entry_id,
                        &app_handle,
                        &state,
                        method.as_str(),
                        &url,
                        &host,
                        &path,
                        101,
                        request_headers,
                        response_headers,
                        vec![],
                        "WebSocket Connection Established".to_string().into_bytes(),
                        0,
                        false,
                        false,
                        false,
                    ).await;

                    let state_clone = Arc::clone(&state);
                    let app_handle_clone = app_handle.clone();
                    let url_clone = url.clone();
                    let subprotocol_clone = subprotocol.clone();

                    tokio::spawn(async move {
                        let client_upgraded = match hyper::upgrade::on(req).await {
                            Ok(up) => up,
                            Err(e) => {
                                eprintln!("[WS Proxy] Error upgrading client connection {}: {}", url_clone, e);
                                return;
                            }
                        };
                        let server_upgraded = match hyper::upgrade::on(res).await {
                            Ok(up) => up,
                            Err(e) => {
                                eprintln!("[WS Proxy] Error upgrading server connection {}: {}", url_clone, e);
                                return;
                            }
                        };

                        if ws_mitm_enabled {
                            crate::ws::proxy_pipe::bridge_proxied_websocket(
                                TokioIo::new(client_upgraded),
                                TokioIo::new(server_upgraded),
                                url_clone,
                                None,
                                subprotocol_clone,
                                app_handle_clone,
                                state_clone,
                            ).await;
                        } else {
                            let mut c_io = TokioIo::new(client_upgraded);
                            let mut s_io = TokioIo::new(server_upgraded);
                            let _ = tokio::io::copy_bidirectional(&mut c_io, &mut s_io).await;
                        }
                    });

                    return Ok(client_res);
                } else {
                    let mut client_res_builder = Response::builder().status(status);
                    for (k, v) in headers_to_vec(res.headers()).iter() {
                        client_res_builder = client_res_builder.header(k, v);
                    }
                    let collected = res.into_body().collect().await?.to_bytes();
                    return Ok(client_res_builder.body(Full::new(collected)).unwrap());
                }
            }
            Err(e) => {
                return Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Full::new(Bytes::from(format!("WebSocket handshake failed: {}", e))))
                    .unwrap());
            }
        }
    }

    let method = req.method().clone();
    let url = req.uri().to_string();
    let host = req.uri().host().unwrap_or_default().to_string();
    let request_headers = headers_to_vec(req.headers());

    let (_parts, body) = req.into_parts();
    let collected_req_body = body.collect().await?.to_bytes();
    let request_body_bytes = collected_req_body;

    let path = parse_path_from_url(&url);

    let entry_id = Uuid::new_v4().to_string();

    let proxy_mode = { state.proxy_config.read().await.proxy_mode.clone() };

    if proxy_mode == "block" {
        log_and_emit_history(
            &entry_id,
            &app_handle,
            &state,
            method.as_str(),
            &url,
            &host,
            &path,
            0,
            request_headers,
            vec![("Content-Type".to_string(), "text/plain".to_string())],
            request_body_bytes.to_vec(),
            "[MITM] Request blocked by proxy (Block mode)".to_string().into_bytes(),
            0,
            false,
            false,
            true,
        ).await;

        return Ok(Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .header("Content-Type", "text/plain")
            .body(Full::new(Bytes::from("Request blocked by MITM proxy (Proxy mode: Block)")))
            .unwrap());
    }

    let mut was_rewritten = false;
    let mut was_intercepted = false;

    // 1. Request Rewrite Engine (Auto background transformation & Mocking)
    let mut req_method_str = method.to_string();
    let mut req_url_str = url.clone();
    let mut req_host_str = host.clone();
    let mut req_headers = request_headers.clone();
    let mut req_body_bytes = request_body_bytes.to_vec();

    if let Some(mock_res) = rewrite::apply_request_rewrite_pipeline(
        &app_handle,
        &state,
        &mut req_method_str,
        &mut req_url_str,
        &mut req_host_str,
        &mut req_headers,
        &mut req_body_bytes,
    ).await {
        let final_path = parse_path_from_url(&req_url_str);
        log_and_emit_history(
            &entry_id,
            &app_handle,
            &state,
            &req_method_str,
            &req_url_str,
            &req_host_str,
            &final_path,
            mock_res.status,
            req_headers,
            mock_res.headers.clone(),
            req_body_bytes,
            mock_res.body.clone(),
            0,
            false,
            true,
            false,
        ).await;

        let mut builder = Response::builder().status(mock_res.status);
        for (k, v) in mock_res.headers.iter() {
            builder = builder.header(k, v);
        }
        return Ok(builder.body(Full::new(Bytes::from(mock_res.body))).unwrap());
    }

    if req_method_str != method.to_string() || req_url_str != url || req_headers != request_headers || req_body_bytes != request_body_bytes {
        was_rewritten = true;
    }

    // Emit initial in-flight request to history list immediately
    {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        let initial_entry = HistoryEntry {
            id: entry_id.clone(),
            method: req_method_str.clone(),
            url: req_url_str.clone(),
            host: req_host_str.clone(),
            path: parse_path_from_url(&req_url_str),
            content_type: "-".to_string(),
            response_size: 0,
            status_code: 0,
            request_headers: req_headers.clone(),
            response_headers: vec![],
            request_body: encode_body_for_ui(&req_body_bytes, "", ""),
            response_body: "".to_string(),
            phase: "request".to_string(),
            duration_ms: None,
            created_at: now,
            is_intercepted: false,
            is_rewritten: was_rewritten,
            is_failed: false,
        };
        let _ = app_handle.emit("traffic_captured", &TrafficCapturedEvent { entry: initial_entry });
    }

    // 2. Request Intercept Hook (Manual pause)
    let (final_req_url, final_req_method, final_req_headers, final_req_body) = match handle_intercept_hook(
        Some(&entry_id),
        &app_handle,
        &state,
        InterceptPhase::Request,
        &req_method_str,
        &req_url_str,
        &req_host_str,
        req_headers.clone(),
        req_body_bytes.clone(),
    ).await {
        Some(InterceptAction::Drop) => {
            let final_path = parse_path_from_url(&req_url_str);
            log_and_emit_history(
                &entry_id,
                &app_handle,
                &state,
                &req_method_str,
                &req_url_str,
                &req_host_str,
                &final_path,
                0,
                req_headers,
                vec![],
                req_body_bytes,
                "[MITM] Request dropped by Interceptor".as_bytes().to_vec(),
                0,
                true,
                was_rewritten,
                true,
            ).await;

            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from("[MITM] Request dropped by Interceptor")))
                .unwrap());
        }
        Some(InterceptAction::Forward { modified_url, modified_method, modified_headers, modified_body }) => {
            was_intercepted = true;
            let u = modified_url.unwrap_or(req_url_str);
            let m = modified_method
                .and_then(|s| s.parse::<Method>().ok())
                .unwrap_or_else(|| Method::from_bytes(req_method_str.as_bytes()).unwrap_or(Method::GET));
            let h = modified_headers.unwrap_or(req_headers);
            let b = modified_body.unwrap_or(req_body_bytes);

            // Re-emit in-flight request as waiting for upstream response
            let now = time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default();
            let in_flight_entry = HistoryEntry {
                id: entry_id.clone(),
                method: m.to_string(),
                url: u.clone(),
                host: u.parse::<hyper::Uri>().ok().and_then(|uri| uri.host().map(|h| h.to_string())).unwrap_or_else(|| host.clone()),
                path: parse_path_from_url(&u),
                content_type: "-".to_string(),
                response_size: 0,
                status_code: 0,
                request_headers: h.clone(),
                response_headers: vec![],
                request_body: encode_body_for_ui(&b, "", ""),
                response_body: "".to_string(),
                phase: "request".to_string(),
                duration_ms: None,
                created_at: now,
                is_intercepted: true,
                is_rewritten: was_rewritten,
                is_failed: false,
            };
            let _ = app_handle.emit("traffic_captured", &TrafficCapturedEvent { entry: in_flight_entry });

            (u, m, h, b)
        }
        None => (req_url_str, Method::from_bytes(req_method_str.as_bytes()).unwrap_or(Method::GET), req_headers, req_body_bytes),
    };

    let final_host = match final_req_url.parse::<hyper::Uri>() {
        Ok(u) => u.host().unwrap_or(&host).to_string(),
        Err(_) => host.clone(),
    };
    let final_path = parse_path_from_url(&final_req_url);

    let mut new_req = Request::builder()
        .method(final_req_method.clone())
        .uri(final_req_url.clone());

    let mut headers_cleaned = final_req_headers.clone();
    headers_cleaned.retain(|(k, _)| !k.eq_ignore_ascii_case("content-length") && !k.eq_ignore_ascii_case("transfer-encoding"));

    for (k, v) in headers_cleaned.iter() {
        new_req = new_req.header(k, v);
    }

    let new_req = match new_req.body(Full::new(Bytes::from(final_req_body.clone()))) {
        Ok(r) => r,
        Err(e) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!("Invalid modified request: {}", e))))
                .unwrap());
        }
    };

    let start_time = Instant::now();
    match client.request(new_req).await {
        Ok(res) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let status = res.status().as_u16();
            let response_headers = headers_to_vec(res.headers());
            let (_parts, body) = res.into_parts();
            let collected_res_body = body.collect().await?.to_bytes();
            let response_body_bytes = collected_res_body;

            // 3. Response Rewrite Engine
            let mut res_status = status;
            let mut res_headers = response_headers;
            let mut res_body = response_body_bytes.to_vec();

            if rewrite::apply_response_rewrite_pipeline(
                &app_handle,
                &state,
                &final_req_method.to_string(),
                &final_req_url,
                &final_host,
                &mut res_status,
                &mut res_headers,
                &mut res_body,
            ).await {
                was_rewritten = true;
            }

            // 4. Response Intercept Hook (Manual pause)
            let (final_res_headers, final_res_body) = match handle_intercept_hook(
                Some(&entry_id),
                &app_handle,
                &state,
                InterceptPhase::Response,
                &final_req_method.to_string(),
                &final_req_url,
                &final_host,
                res_headers.clone(),
                res_body.clone(),
            ).await {
                Some(InterceptAction::Drop) => {
                    log_and_emit_history(
                        &entry_id,
                        &app_handle,
                        &state,
                        &final_req_method.to_string(),
                        &final_req_url,
                        &final_host,
                        &final_path,
                        0,
                        final_req_headers,
                        vec![],
                        final_req_body,
                        "[MITM] Response dropped by Interceptor".as_bytes().to_vec(),
                        duration_ms,
                        true,
                        was_rewritten,
                        true,
                    ).await;

                    return Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(Full::new(Bytes::from("[MITM] Response dropped by Interceptor")))
                        .unwrap());
                }
                Some(InterceptAction::Forward { modified_headers, modified_body, .. }) => {
                    was_intercepted = true;
                    (
                        modified_headers.unwrap_or(res_headers),
                        modified_body.unwrap_or(res_body),
                    )
                }
                None => (res_headers, res_body),
            };

            log_and_emit_history(
                &entry_id,
                &app_handle,
                &state,
                &final_req_method.to_string(),
                &final_req_url,
                &final_host,
                &final_path,
                res_status,
                final_req_headers,
                final_res_headers.clone(),
                final_req_body,
                final_res_body.clone(),
                duration_ms,
                was_intercepted,
                was_rewritten,
                false,
            ).await;

            if proxy_mode == "block_client" {
                return Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("Content-Type", "text/plain")
                    .body(Full::new(Bytes::from("[MITM] Response blocked from client (Block Client mode)")))
                    .unwrap());
            }

            let mut response_headers_cleaned = final_res_headers.clone();
            response_headers_cleaned.retain(|(k, _)| !k.eq_ignore_ascii_case("content-length") && !k.eq_ignore_ascii_case("transfer-encoding"));

            let mut builder = Response::builder().status(res_status);
            for (k, v) in response_headers_cleaned.iter() {
                builder = builder.header(k, v);
            }
            Ok(builder.body(Full::new(Bytes::from(final_res_body))).unwrap())
        }
        Err(e) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let err_msg = format!("Proxy error: {}", e);
            log_and_emit_history(
                &entry_id,
                &app_handle,
                &state,
                &final_req_method.to_string(),
                &final_req_url,
                &final_host,
                &final_path,
                0,
                final_req_headers,
                vec![("Content-Type".to_string(), "text/plain".to_string())],
                final_req_body,
                err_msg.as_bytes().to_vec(),
                duration_ms,
                was_intercepted,
                was_rewritten,
                true,
            ).await;

            Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(err_msg)))
                .unwrap())
        }
    }
}

async fn log_and_emit_history(
    entry_id: &str,
    app_handle: &AppHandle,
    state: &Arc<AppState>,
    method: &str,
    full_url: &str,
    host: &str,
    path: &str,
    status_code: u16,
    req_headers: Vec<(String, String)>,
    res_headers: Vec<(String, String)>,
    req_body: Vec<u8>,
    res_body: Vec<u8>,
    duration_ms: u64,
    is_intercepted: bool,
    is_rewritten: bool,
    is_failed: bool,
) {
    let content_encoding = res_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-encoding"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");

    let decompressed_res_body = decompress_body(&res_body, content_encoding);

    let content_type = res_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "-".to_string());

    let req_content_type = req_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");

    let req_body_str = encode_body_for_ui(&req_body, req_content_type, "");
    let res_body_str = encode_body_for_ui(&decompressed_res_body, &content_type, content_encoding);
    let response_size = decompressed_res_body.len() as u64;

    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    let history_entry = HistoryEntry {
        id: entry_id.to_string(),
        method: method.to_string(),
        url: full_url.to_string(),
        host: host.to_string(),
        path: path.to_string(),
        content_type,
        response_size,
        status_code,
        request_headers: req_headers,
        response_headers: res_headers,
        request_body: req_body_str,
        response_body: res_body_str,
        phase: "response".to_string(),
        duration_ms: Some(duration_ms),
        created_at: now,
        is_intercepted,
        is_rewritten,
        is_failed,
    };

    let _ = state.history_tx.send(history_entry.clone()).await;
    let _ = app_handle.emit("traffic_captured", &TrafficCapturedEvent { entry: history_entry });
}

fn headers_to_vec(headers: &hyper::HeaderMap) -> Vec<(String, String)> {
    let mut vec = Vec::new();
    for (name, value) in headers.iter() {
        vec.push((
            name.to_string().to_lowercase(),
            value.to_str().unwrap_or("").to_string(),
        ));
    }
    vec
}

use crate::encoding::{decompress_body, format_body_for_ui as encode_body_for_ui};

pub(crate) fn parse_path_from_url(url_str: &str) -> String {
    if let Some(pos) = url_str.find("://") {
        let rest = &url_str[pos + 3..];
        if let Some(slash_pos) = rest.find('/') {
            return rest[slash_pos..].to_string();
        }
        return "/".to_string();
    }
    if url_str.starts_with('/') {
        url_str.to_string()
    } else {
        "/".to_string()
    }
}
