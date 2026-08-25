pub mod mitm;
pub mod intercept;
pub mod rules;

use std::io::Read;
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
use flate2::read::{GzDecoder, ZlibDecoder};
use base64::Engine;

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
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new()).build(https);

    let method = req.method().clone();
    let url = req.uri().to_string();
    let host = req.uri().host().unwrap_or_default().to_string();
    let request_headers = headers_to_vec(req.headers());

    let (_parts, body) = req.into_parts();
    let collected_req_body = body.collect().await?.to_bytes();
    let request_body_bytes = collected_req_body;

    let path = parse_path_from_url(&url);

    let proxy_mode = { state.proxy_config.read().await.proxy_mode.clone() };

    if proxy_mode == "block" {
        log_and_emit_history(
            &app_handle,
            &state,
            &method.to_string(),
            &url,
            &host,
            &path,
            502,
            request_headers,
            vec![("Content-Type".to_string(), "text/plain".to_string())],
            request_body_bytes.to_vec(),
            "[MITM] Request blocked by proxy (Block mode)".to_string().into_bytes(),
            0,
        ).await;

        return Ok(Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .header("Content-Type", "text/plain")
            .body(Full::new(Bytes::from("Request blocked by MITM proxy (Proxy mode: Block)")))
            .unwrap());
    }

    // 1. Request Intercept Hook
    let (final_req_headers, final_req_body) = match handle_intercept_hook(
        &app_handle,
        &state,
        InterceptPhase::Request,
        &method.to_string(),
        &url,
        &host,
        request_headers.clone(),
        request_body_bytes.to_vec(),
    ).await {
        Some(InterceptAction::Drop) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from("[MITM] Request dropped by Interceptor")))
                .unwrap());
        }
        Some(InterceptAction::Forward { modified_headers, modified_body }) => (
            modified_headers.unwrap_or(request_headers),
            modified_body.unwrap_or(request_body_bytes.to_vec()),
        ),
        None => (request_headers, request_body_bytes.to_vec()),
    };

    let mut new_req = Request::builder()
        .method(method.clone())
        .uri(url.clone());

    let mut headers_cleaned = final_req_headers.clone();
    headers_cleaned.retain(|(k, _)| !k.eq_ignore_ascii_case("content-length") && !k.eq_ignore_ascii_case("transfer-encoding"));

    for (k, v) in headers_cleaned.iter() {
        new_req = new_req.header(k, v);
    }

    let new_req = new_req.body(Full::new(Bytes::from(final_req_body.clone()))).unwrap();

    let start_time = Instant::now();
    match client.request(new_req).await {
        Ok(res) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let status = res.status().as_u16();
            let response_headers = headers_to_vec(res.headers());
            let (_parts, body) = res.into_parts();
            let collected_res_body = body.collect().await?.to_bytes();
            let response_body_bytes = collected_res_body;

            // 2. Response Intercept Hook
            let (final_res_headers, final_res_body) = match handle_intercept_hook(
                &app_handle,
                &state,
                InterceptPhase::Response,
                &method.to_string(),
                &url,
                &host,
                response_headers.clone(),
                response_body_bytes.to_vec(),
            ).await {
                Some(InterceptAction::Drop) => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(Full::new(Bytes::from("[MITM] Response dropped by Interceptor")))
                        .unwrap());
                }
                Some(InterceptAction::Forward { modified_headers, modified_body }) => (
                    modified_headers.unwrap_or(response_headers),
                    modified_body.unwrap_or(response_body_bytes.to_vec()),
                ),
                None => (response_headers, response_body_bytes.to_vec()),
            };

            log_and_emit_history(
                &app_handle,
                &state,
                &method.to_string(),
                &url,
                &host,
                &path,
                status,
                final_req_headers,
                final_res_headers.clone(),
                final_req_body,
                final_res_body.clone(),
                duration_ms,
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

            let mut builder = Response::builder().status(status);
            for (k, v) in response_headers_cleaned.iter() {
                builder = builder.header(k, v);
            }
            Ok(builder.body(Full::new(Bytes::from(final_res_body))).unwrap())
        }
        Err(e) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let err_msg = format!("Proxy error: {}", e);
            log_and_emit_history(
                &app_handle,
                &state,
                &method.to_string(),
                &url,
                &host,
                &path,
                502,
                final_req_headers,
                vec![("Content-Type".to_string(), "text/plain".to_string())],
                final_req_body,
                err_msg.as_bytes().to_vec(),
                duration_ms,
            ).await;

            Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(err_msg)))
                .unwrap())
        }
    }
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
    let content_encoding = res_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-encoding"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");

    let decompressed_res_body = if !content_encoding.is_empty() {
        decompress_body(&res_body, content_encoding).unwrap_or_else(|| res_body.clone())
    } else {
        res_body.clone()
    };

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

    let req_body_str = encode_body_for_ui(&req_body, req_content_type);
    let res_body_str = encode_body_for_ui(&decompressed_res_body, &content_type);
    let response_size = decompressed_res_body.len() as u64;

    let entry_id = Uuid::new_v4().to_string();
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    let history_entry = HistoryEntry {
        id: entry_id,
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

fn decompress_body(body: &[u8], encoding: &str) -> Option<Vec<u8>> {
    let encodings: Vec<&str> = encoding.split(',').map(|s| s.trim()).collect();
    let mut current_body = body.to_vec();
    let mut decompressed = false;

    for enc in encodings.iter().rev() {
        let enc = enc.to_lowercase();
        match enc.as_str() {
            "gzip" | "x-gzip" => {
                let mut decoder = GzDecoder::new(&current_body[..]);
                let mut decoded = Vec::new();
                if decoder.read_to_end(&mut decoded).is_ok() {
                    current_body = decoded;
                    decompressed = true;
                } else {
                    return None;
                }
            }
            "deflate" => {
                let mut decoder = ZlibDecoder::new(&current_body[..]);
                let mut decoded = Vec::new();
                if decoder.read_to_end(&mut decoded).is_ok() {
                    current_body = decoded;
                    decompressed = true;
                } else {
                    return None;
                }
            }
            "br" => {
                let mut decoded = Vec::new();
                if brotli::Decompressor::new(&current_body[..], 4096).read_to_end(&mut decoded).is_ok() {
                    current_body = decoded;
                    decompressed = true;
                } else {
                    return None;
                }
            }
            "zstd" => {
                if let Ok(decoded) = zstd::decode_all(&current_body[..]) {
                    current_body = decoded;
                    decompressed = true;
                } else {
                    return None;
                }
            }
            "identity" | "" => {}
            _ => {
                return if decompressed { Some(current_body) } else { None };
            }
        }
    }

    if decompressed {
        Some(current_body)
    } else {
        None
    }
}

fn encode_body_for_ui(body: &[u8], content_type: &str) -> String {
    let is_binary = !content_type.is_empty() && (
        content_type.contains("image/") ||
        content_type.contains("video/") ||
        content_type.contains("audio/") ||
        content_type.contains("application/octet-stream") ||
        content_type.contains("application/pdf") ||
        content_type.contains("application/zip") ||
        content_type.contains("application/gzip") ||
        content_type.contains("font/")
    );

    if is_binary {
        let encoded = base64::engine::general_purpose::STANDARD.encode(body);
        format!("base64:{}", encoded)
    } else {
        String::from_utf8(body.to_vec())
            .unwrap_or_else(|_| String::from_utf8_lossy(body).to_string())
    }
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
