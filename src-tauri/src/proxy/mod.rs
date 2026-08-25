pub mod mitm;
pub mod intercept;
pub mod rules;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::ca::RootCa;
use crate::proxy::intercept::handle_intercept_hook;
use crate::state::{AppState, HistoryDetailItem, HistoryEntry, HistorySummaryItem, InterceptAction, InterceptPhase, TrafficCapturedEvent};

pub async fn start_proxy_server(
    app_handle: AppHandle,
    state: Arc<AppState>,
    ca: Arc<RootCa>,
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
                            if let Err(e) = handle_connection(app_handle_clone, state_clone, ca_clone, stream, client_addr).await {
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
    app_handle: AppHandle,
    state: Arc<AppState>,
    ca: Arc<RootCa>,
    mut stream: TcpStream,
    _client_addr: SocketAddr,
) -> Result<(), String> {
    let mut buffer = [0u8; 8192];
    let n = stream.peek(&mut buffer).await.map_err(|e| e.to_string())?;
    if n == 0 {
        return Ok(());
    }

    let req_str = String::from_utf8_lossy(&buffer[..n]);

    if req_str.starts_with("CONNECT ") {
        let line = req_str.lines().next().unwrap_or_default();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err("Invalid CONNECT request".to_string());
        }

        let host_port = parts[1];
        let domain = host_port.split(':').next().unwrap_or(host_port);

        let mut conn_buf = vec![0u8; n];
        let _ = stream.read(&mut conn_buf).await;

        stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await.map_err(|e| e.to_string())?;

        let mitm_engine = crate::proxy::mitm::MitmEngine::new(ca);
        if let Ok(acceptor) = mitm_engine.create_tls_acceptor(domain).await {
            if let Ok(mut tls_stream) = acceptor.accept(stream).await {
                let mut tls_buf = [0u8; 8192];
                if let Ok(read_bytes) = tls_stream.read(&mut tls_buf).await {
                    if read_bytes > 0 {
                        process_http_request(app_handle, state, &mut tls_stream, &tls_buf[..read_bytes], domain, true).await?;
                    }
                }
            }
        }
    } else {
        let mut req_buf = vec![0u8; n];
        let _ = stream.read(&mut req_buf).await;
        let host = parse_host_from_headers(&req_str).unwrap_or_else(|| "127.0.0.1".to_string());
        process_http_request(app_handle, state, &mut stream, &req_buf, &host, false).await?;
    }

    Ok(())
}

async fn process_http_request<S>(
    app_handle: AppHandle,
    state: Arc<AppState>,
    stream: &mut S,
    raw_req_bytes: &[u8],
    default_host: &str,
    is_tls: bool,
) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let proxy_mode = { state.proxy_config.read().await.proxy_mode.clone() };

    let req_str = String::from_utf8_lossy(raw_req_bytes);
    let mut lines = req_str.lines();
    let first_line = lines.next().unwrap_or_default();
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    let method = parts.get(0).unwrap_or(&"GET").to_string();
    let raw_url = parts.get(1).unwrap_or(&"/").to_string();
    let raw_host = parse_host_from_headers(&req_str).unwrap_or_else(|| default_host.to_string());

    let scheme = if is_tls { "https" } else { "http" };
    let full_host = if raw_host.starts_with("http://") || raw_host.starts_with("https://") {
        raw_host.clone()
    } else {
        format!("{}://{}", scheme, raw_host)
    };

    let full_url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
        raw_url.clone()
    } else {
        format!("{}://{}{}", scheme, raw_host, raw_url)
    };

    let path = parse_path_from_url(&raw_url);

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }

    let body_start = if let Some(pos) = req_str.find("\r\n\r\n") {
        pos + 4
    } else if let Some(pos) = req_str.find("\n\n") {
        pos + 2
    } else {
        raw_req_bytes.len()
    };

    let req_body_bytes = if body_start < raw_req_bytes.len() {
        raw_req_bytes[body_start..].to_vec()
    } else {
        Vec::new()
    };


    // Mode: "block" -> Never send to server, never send to client
    if proxy_mode == "block" {
        log_and_emit_history(
            &app_handle,
            &state,
            &method,
            &full_url,
            &full_host,
            &path,
            502,
            headers,
            vec![("Content-Type".to_string(), "text/plain".to_string())],
            req_body_bytes,
            "[MITM] Request blocked by proxy (Block mode)".to_string().into_bytes(),
            0,
        ).await;

        let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\n\r\n[MITM] Blocked (Proxy mode: Block)").await;
        return Ok(());
    }

    // 1. Request Intercept Hook
    let (final_req_headers, final_req_body) = match handle_intercept_hook(
        &app_handle,
        &state,
        InterceptPhase::Request,
        &method,
        &full_url,
        &full_host,
        headers.clone(),
        req_body_bytes.clone(),
    ).await {
        Some(InterceptAction::Drop) => {
            let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\n\r\n[MITM] Request dropped by Interceptor").await;
            return Ok(());
        }
        Some(InterceptAction::Forward { modified_headers, modified_body }) => (
            modified_headers.unwrap_or(headers),
            modified_body.unwrap_or(req_body_bytes),
        ),
        None => (headers, req_body_bytes),
    };

    // 2. Forward request to target server using reqwest
    let start_time = Instant::now();
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut req_builder = match method.to_uppercase().as_str() {
        "POST" => client.post(&full_url),
        "PUT" => client.put(&full_url),
        "DELETE" => client.delete(&full_url),
        "PATCH" => client.patch(&full_url),
        "HEAD" => client.head(&full_url),
        "OPTIONS" => client.request(reqwest::Method::OPTIONS, &full_url),
        _ => client.get(&full_url),
    };

    for (k, v) in &final_req_headers {
        if k.eq_ignore_ascii_case("host") || k.eq_ignore_ascii_case("content-length") {
            continue;
        }
        req_builder = req_builder.header(k, v);
    }

    if !final_req_body.is_empty() {
        req_builder = req_builder.body(final_req_body.clone());
    }

    let res = req_builder.send().await;
    let duration_ms = start_time.elapsed().as_millis() as u64;

    let (status_code, res_headers, res_body_bytes) = match res {
        Ok(response) => {
            let status = response.status().as_u16();
            let mut headers_vec = Vec::new();
            for (k, v) in response.headers().iter() {
                headers_vec.push((k.as_str().to_string(), v.to_str().unwrap_or_default().to_string()));
            }
            let body_bytes = response.bytes().await.map(|b| b.to_vec()).unwrap_or_default();
            (status, headers_vec, body_bytes)
        }
        Err(e) => {
            (502, vec![("Content-Type".to_string(), "text/plain".to_string())], format!("Proxy Error: {}", e).into_bytes())
        }
    };

    // 3. Response Intercept Hook
    let (final_res_headers, final_res_body) = match handle_intercept_hook(
        &app_handle,
        &state,
        InterceptPhase::Response,
        &method,
        &full_url,
        &full_host,
        res_headers.clone(),
        res_body_bytes.clone(),
    ).await {
        Some(InterceptAction::Drop) => {
            let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\n\r\n[MITM] Response dropped by Interceptor").await;
            return Ok(());
        }
        Some(InterceptAction::Forward { modified_headers, modified_body }) => (
            modified_headers.unwrap_or(res_headers),
            modified_body.unwrap_or(res_body_bytes),
        ),
        None => (res_headers, res_body_bytes),
    };

    // 4. Log History & Broadcast Event
    log_and_emit_history(
        &app_handle,
        &state,
        &method,
        &full_url,
        &full_host,
        &path,
        status_code,
        final_req_headers,
        final_res_headers.clone(),
        final_req_body,
        final_res_body.clone(),
        duration_ms,
    ).await;

    // Mode: "block_client" -> Sent to server and logged, but client does NOT get response
    if proxy_mode == "block_client" {
        let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\n\r\n[MITM] Response blocked from client (Block Client mode)").await;
        return Ok(());
    }

    // Mode: "on" -> Send Response Back to Client Stream
    let mut response_head = format!("HTTP/1.1 {} OK\r\n", status_code);
    for (k, v) in &final_res_headers {
        if k.eq_ignore_ascii_case("transfer-encoding") || k.eq_ignore_ascii_case("content-length") {
            continue;
        }
        response_head.push_str(&format!("{}: {}\r\n", k, v));
    }
    response_head.push_str(&format!("Content-Length: {}\r\n", final_res_body.len()));
    response_head.push_str("\r\n");

    stream.write_all(response_head.as_bytes()).await.map_err(|e| e.to_string())?;
    stream.write_all(&final_res_body).await.map_err(|e| e.to_string())?;

    Ok(())
}

async fn log_and_emit_history(
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
) {
    let req_body_str = String::from_utf8_lossy(&req_body).to_string();
    let res_body_str = String::from_utf8_lossy(&res_body).to_string();

    let content_type = res_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "-".to_string());

    let response_size = res_body.len() as u64;

    let entry_uuid = Uuid::new_v4().to_string();
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    let history_entry = HistoryEntry {
        id: 0,
        uuid: entry_uuid.clone(),
        method: method.to_string(),
        url: full_url.to_string(),
        host: host.to_string(),
        status_code,
        request_headers: req_headers.clone(),
        response_headers: res_headers.clone(),
        request_body: req_body_str.clone(),
        response_body: res_body_str.clone(),
        phase: "response".to_string(),
        duration_ms: Some(duration_ms),
        created_at: now.clone(),
    };

    let summary_item = HistorySummaryItem {
        id: 0,
        uuid: entry_uuid.clone(),
        method: method.to_string(),
        url: full_url.to_string(),
        host: host.to_string(),
        path: path.to_string(),
        content_type,
        response_size,
        status_code,
        duration_ms: Some(duration_ms),
        created_at: now.clone(),
    };

    let detail_item = HistoryDetailItem {
        id: 0,
        uuid: entry_uuid,
        method: method.to_string(),
        url: full_url.to_string(),
        host: host.to_string(),
        status_code,
        request_headers: req_headers,
        response_headers: res_headers,
        request_body: req_body_str,
        response_body: res_body_str,
        request_body_hex: None,
        response_body_hex: None,
        phase: "response".to_string(),
        duration_ms: Some(duration_ms),
        created_at: now,
    };

    let _ = state.history_tx.send(history_entry).await;
    let _ = app_handle.emit("traffic_captured", &TrafficCapturedEvent { entry: summary_item, detail: detail_item });
}

fn parse_host_from_headers(raw: &str) -> Option<String> {
    for line in raw.lines() {
        if line.to_lowercase().starts_with("host:") {
            return Some(line[5..].trim().to_string());
        }
    }
    None
}

fn parse_path_from_url(url_str: &str) -> String {
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
