use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_rustls::rustls::{self, pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer}};
use tokio_rustls::TlsAcceptor;
use hyper::{Request, Response, StatusCode, Method};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use hyper_util::client::legacy::Client;
use hyper_rustls;
use http_body_util::{BodyExt, Full};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;
use rcgen::{CertificateParams, KeyPair, DnType};
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Read;
use flate2::read::{GzDecoder, ZlibDecoder};
use base64::Engine;

use crate::ca::CA;
use crate::repeater_execute::reconstruct_multipart_if_needed;

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
                match zstd::decode_all(&current_body[..]) {
                    Ok(decoded) => {
                        current_body = decoded;
                        decompressed = true;
                    }
                    Err(_) => return None,
                }
            }
            "identity" | "" => {}
            _ => {
                // Unknown encoding, stop decompressing
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

/// Encode a body as string, using base64 for binary content
fn encode_body_for_ui(body: &[u8], content_type: &str) -> String {
    // Check if content is binary based on content-type
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
        // Base64 encode binary content with prefix
        let encoded = base64::engine::general_purpose::STANDARD.encode(body);
        format!("base64:{}", encoded)
    } else {
        // Try to parse as UTF-8, fall back to lossy conversion
        String::from_utf8(body.to_vec())
            .unwrap_or_else(|_| String::from_utf8_lossy(body).to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Traffic {
    pub id: String,
    pub method: String,
    pub url: String,
    pub host: String,
    pub status_code: u16,
    pub request_headers: Vec<(String, String)>,
    pub response_headers: Vec<(String, String)>,
    pub request_body: String,
    pub response_body: String,
    pub phase: String,
    pub is_intercepted: bool,
    pub intercepted_at: Option<u64>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptConfig {
    pub enabled: bool,
    pub mode: String, // "both", "request", "response"
    pub ignored_methods: Vec<String>,
    pub url_filter: String,
    pub auto_focus: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    pub id: String,
    pub is_active: bool,
    pub field: String,     // "url", "method", "status_code"
    pub rule_type: String, // "contains", "starts_with", "ends_with", "exact"
    pub mode: String,      // "whitelist", "blacklist"
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub rules: Vec<FilterRule>,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
        }
    }
}

impl FilterConfig {
    pub fn should_process(&self, url: &str, method: &str, status_code: u16) -> bool {
        let active_rules: Vec<&FilterRule> = self.rules.iter().filter(|r| r.is_active).collect();
        
        if active_rules.is_empty() {
            return true;
        }

        let whitelists: Vec<&FilterRule> = active_rules.iter().filter(|r| r.mode == "whitelist").cloned().collect();
        let blacklists: Vec<&FilterRule> = active_rules.iter().filter(|r| r.mode == "blacklist").cloned().collect();

        let matches = |rules: &[&FilterRule]| -> bool {
            rules.iter().any(|rule| {
                let value_to_check = match rule.field.as_str() {
                    "method" => Some(method.to_string()),
                    "status_code" => {
                        if status_code == 0 {
                            None // Can't match status code yet
                        } else {
                            Some(status_code.to_string())
                        }
                    },
                    _ => Some(url.to_string()), // default to url
                };

                if let Some(val) = value_to_check {
                    match rule.rule_type.as_str() {
                        "contains" => val.contains(&rule.pattern),
                        "starts_with" => val.starts_with(&rule.pattern),
                        "ends_with" => val.ends_with(&rule.pattern),
                        "exact" => val == rule.pattern,
                        _ => false,
                    }
                } else {
                    false
                }
            })
        };

        if whitelists.is_empty() {
            // Only blacklists: Allow everything EXCEPT matches
            !matches(&blacklists)
        } else if blacklists.is_empty() {
            // Only whitelists: Allow ONLY matches
            // If status_code is 0, we allow it to proceed so we can check the status code later,
            // UNLESS there are no whitelists for URL or Method.
            // Actually, simpler: if it matches ANY whitelist, it's allowed.
            // If we are at status 0, and there's a status_code whitelist, we don't know yet.
            matches(&whitelists) || (status_code == 0 && whitelists.iter().any(|r| r.field == "status_code"))
        } else {
            // Both: Allow if matches any whitelist AND matches no blacklist
            let is_whitelisted = matches(&whitelists) || (status_code == 0 && whitelists.iter().any(|r| r.field == "status_code"));
            let is_blacklisted = matches(&blacklists);
            is_whitelisted && !is_blacklisted
        }
    }
}

impl Default for InterceptConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "both".to_string(),
            ignored_methods: vec!["OPTIONS".to_string()],
            url_filter: String::new(),
            auto_focus: true,
        }
    }
}


#[derive(Debug, Deserialize)]
pub struct ResumeAction {
    pub drop: Option<bool>,
    pub method: Option<String>,
    pub url: Option<String>,
    pub headers: Option<Vec<(String, String)>>,
    pub body: Option<String>,
    pub status_code: Option<u16>,
    pub variables: Option<HashMap<String, String>>,
}

pub struct InterceptState {
    pub config: InterceptConfig,
    pub filter_config: FilterConfig,
    pub pending: HashMap<String, tokio::sync::oneshot::Sender<ResumeAction>>,
}

pub struct ProxyState {
    pub app_handle: AppHandle,
    pub ca: CA,
    pub intercept: Arc<Mutex<InterceptState>>,
}

pub async fn start_proxy(
    app_handle: AppHandle, 
    ca: CA, 
    intercept: Arc<Mutex<InterceptState>>,
    addr: SocketAddr
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = Arc::new(ProxyState { app_handle, ca, intercept });
    let listener = TcpListener::bind(addr).await?;
    println!("Proxy listening on {}", addr);

    loop {
        let (stream, client_addr) = listener.accept().await?;
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, state, client_addr).await {
                eprintln!("Error handling connection: {}", err);
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    state: Arc<ProxyState>,
    _client_addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let io = TokioIo::new(stream);

    let state_for_service = Arc::clone(&state);
    let service = service_fn(move |req| {
        let state = Arc::clone(&state_for_service);
        proxy_service(req, state)
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

async fn proxy_service(
    req: Request<Incoming>,
    state: Arc<ProxyState>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    if req.method() == Method::CONNECT {
        let host = req.uri().host().unwrap_or_default().to_string();
        let port = req.uri().port_u16().unwrap_or(443);
        
        tokio::spawn(async move {
            if let Err(e) = handle_connect(req, state, host, port).await {
                eprintln!("Error in CONNECT: {}", e);
            }
        });
        
        Ok(Response::new(Full::new(Bytes::new())))
    } else {
        handle_http(req, state).await
    }
}

async fn handle_connect(
    req: Request<Incoming>,
    state: Arc<ProxyState>,
    host: String,
    _port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let upgraded = hyper::upgrade::on(req).await?;
    let upgraded_io = TokioIo::new(upgraded);
    
    // Generate cert for this host
    let mut params = CertificateParams::new(vec![host.clone()]).unwrap();
    params.distinguished_name.push(DnType::CommonName, host.clone());
    params.key_usages.push(rcgen::KeyUsagePurpose::DigitalSignature);
    params.key_usages.push(rcgen::KeyUsagePurpose::KeyEncipherment);
    params.extended_key_usages.push(rcgen::ExtendedKeyUsagePurpose::ServerAuth);
    
    // Adjust dates for clock skew
    let now = SystemTime::now();
    params.not_before = time::OffsetDateTime::from(now - std::time::Duration::from_secs(86400));
    params.not_after = time::OffsetDateTime::from(now + std::time::Duration::from_secs(86400 * 365));

    let cert_key_pair = KeyPair::generate().unwrap();
    let cert = params.signed_by(&cert_key_pair, &state.ca.cert, &state.ca.key_pair).unwrap();
    
    let cert_der = cert.der().clone();
    let key_der = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(cert_key_pair.serialize_der()));

    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)?;
    
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
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
        let host = host_for_service.clone();
        async move {
            let uri = format!("https://{}{}", host, req.uri());
            *req.uri_mut() = uri.parse().unwrap();
            handle_http(req, state).await
        }
    });

    if let Err(err) = hyper::server::conn::http1::Builder::new()
        .serve_connection(io, service)
        .await 
    {
        eprintln!("Error serving TLS connection for {}: {}", host, err);
    }

    Ok(())
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

async fn handle_http(
    req: Request<Incoming>,
    state: Arc<ProxyState>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_or_http()
        .enable_http1()
        .build();
    let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build(https);
    
    let mut method = req.method().clone();
    let mut url = req.uri().to_string();
    let host = req.uri().host().unwrap_or_default().to_string();
    let mut request_headers = headers_to_vec(req.headers());

    let (_parts, body) = req.into_parts();
    let collected_req_body = body.collect().await?.to_bytes();
    let mut request_body_bytes = collected_req_body;

    // 0. Check Traffic Filter
    let filter_config = {
        let intercept = state.intercept.lock().await;
        intercept.filter_config.clone()
    };

    if !filter_config.should_process(&url, &method.to_string(), 0) {
        let mut new_req = Request::builder()
            .method(method.clone())
            .uri(url.clone());
        
        // Remove content-length/transfer-encoding to let hyper recalculate
        let mut headers_cleaned = request_headers.clone();
        headers_cleaned.retain(|(k, _)| k != "content-length" && k != "transfer-encoding");

        for (k, v) in headers_cleaned.iter() {
            new_req = new_req.header(k, v);
        }
        
        let new_req = new_req.body(Full::new(request_body_bytes.clone())).unwrap();
        
        match client.request(new_req).await {
            Ok(res) => {
                let status = res.status().as_u16();
                let mut response_headers = headers_to_vec(res.headers());
                let (_parts, body) = res.into_parts();
                let collected_res_body = body.collect().await?.to_bytes();
                let response_body_bytes = collected_res_body;

                response_headers.retain(|(k, _)| k != "content-length" && k != "transfer-encoding");

                let mut builder = Response::builder().status(status);
                for (k, v) in response_headers.iter() {
                    builder = builder.header(k, v);
                }
                return Ok(builder.body(Full::new(response_body_bytes)).unwrap());
            }
            Err(e) => {
                eprintln!("Outbound filtered request error: {}", e);
                return Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Full::new(Bytes::from(format!("Proxy error: {}", e))))
                    .unwrap());
            }
        }
    }

    // 1. Check Request Interception
    let intercept_config = {
        let intercept = state.intercept.lock().await;
        intercept.config.clone()
    };

    let traffic_id = Uuid::new_v4().to_string();

    let req_encoding = request_headers.iter()
        .find(|(k, _)| k == "content-encoding")
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    
    let req_content_type = request_headers.iter()
        .find(|(k, _)| k == "content-type")
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    
    let decompressed_req_body = decompress_body(&request_body_bytes, &req_encoding);
    let mut req_body_for_ui = match &decompressed_req_body {
        Some(b) => encode_body_for_ui(b, &req_content_type),
        None => encode_body_for_ui(&request_body_bytes, &req_content_type),
    };

    let proxy_mode = {
        let app_state = state.app_handle.state::<crate::models::AppState>();
        let proxy_manager = app_state.proxy_manager.lock().await;
        proxy_manager.config.proxy_mode.clone()
    };

    if proxy_mode == "halt_all" {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        let traffic = Traffic {
            id: traffic_id.clone(),
            method: method.to_string(),
            url: url.clone(),
            host: host.clone(),
            status_code: 502,
            request_headers: request_headers.clone(),
            response_headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
            request_body: req_body_for_ui.clone(),
            response_body: "Request halted by MITM proxy (no forwarding to server)".to_string(),
            phase: "response".to_string(),
            is_intercepted: false,
            intercepted_at: Some(now),
            duration_ms: Some(0),
        };
        let _ = state.app_handle.emit("traffic_captured", &traffic);
        
        return Ok(Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .header("Content-Type", "text/plain")
            .body(Full::new(Bytes::from("Request halted by MITM proxy (no forwarding to server)")))
            .unwrap());
    }

    if intercept_config.enabled && 
       (intercept_config.mode == "both" || intercept_config.mode == "request") &&
       !intercept_config.ignored_methods.contains(&method.to_string()) &&
       (intercept_config.url_filter.is_empty() || url.contains(&intercept_config.url_filter)) 
    {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        let traffic = Traffic {
            id: traffic_id.clone(),
            method: method.to_string(),
            url: url.clone(),
            host: host.clone(),
            status_code: 0,
            request_headers: request_headers.clone(),
            response_headers: Vec::new(),
            request_body: req_body_for_ui.clone(),
            response_body: String::new(),
            phase: "request".to_string(),
            is_intercepted: true,
            intercepted_at: Some(now),
            duration_ms: None,
        };


        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut intercept = state.intercept.lock().await;
            intercept.pending.insert(traffic_id.clone(), tx);
        }

        let _ = state.app_handle.emit("traffic_captured", &traffic);

        if intercept_config.auto_focus {
            if let Some(window) = state.app_handle.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }

        // Wait for resume
        if let Ok(action) = rx.await {
            if action.drop.unwrap_or(false) {
                return Ok(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Full::new(Bytes::from("Request dropped by proxy")))
                    .unwrap());
            }

            if let Some(m) = action.method { method = m.parse().unwrap_or(method); }
            if let Some(u) = action.url { url = u; }
            if let Some(h) = action.headers { request_headers = h; }
            if let Some(b) = action.body { 
                let mut body_bytes = b.as_bytes().to_vec();
                if let Some(reconstructed) = reconstruct_multipart_if_needed(&b, &mut request_headers) {
                    body_bytes = reconstructed;
                }
                request_body_bytes = Bytes::from(body_bytes);
                // Update UI representation of request body for the next phase
                req_body_for_ui = b;
                // If body was modified or even just resumed from UI, it's now decompressed
                request_headers.retain(|(k, _)| k != "content-encoding");
            }
        }
    }

    let mut new_req = Request::builder()
        .method(method.clone())
        .uri(url.clone());
    
    // Remove content-length/transfer-encoding to let hyper recalculate
    request_headers.retain(|(k, _)| k != "content-length" && k != "transfer-encoding");

    for (k, v) in request_headers.iter() {
        new_req = new_req.header(k, v);
    }
    
    let new_req = new_req.body(Full::new(request_body_bytes.clone())).unwrap();
    
    let start_time = SystemTime::now();
    match client.request(new_req).await {
        Ok(res) => {
            let duration = start_time.elapsed().map(|d| d.as_millis() as u64).unwrap_or(0);
            let mut status = res.status().as_u16();
            let mut response_headers = headers_to_vec(res.headers());
            let (_parts, body) = res.into_parts();
            let collected_res_body = body.collect().await?.to_bytes();
            let mut response_body_bytes = collected_res_body;

            let res_encoding = response_headers.iter()
                .find(|(k, _)| k == "content-encoding")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            
            let res_content_type = response_headers.iter()
                .find(|(k, _)| k == "content-type")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
                
            let decompressed_res_body = decompress_body(&response_body_bytes, &res_encoding);
            let res_body_for_ui = match &decompressed_res_body {
                Some(b) => encode_body_for_ui(b, &res_content_type),
                None => encode_body_for_ui(&response_body_bytes, &res_content_type),
            };

            if proxy_mode == "halt_client" {
                let traffic = Traffic {
                    id: traffic_id.clone(),
                    method: method.to_string(),
                    url: url.clone(),
                    host: host.clone(),
                    status_code: status,
                    request_headers: request_headers.clone(),
                    response_headers: response_headers.clone(),
                    request_body: req_body_for_ui.clone(),
                    response_body: res_body_for_ui.clone(),
                    phase: "response".to_string(),
                    is_intercepted: false,
                    intercepted_at: None,
                    duration_ms: Some(duration),
                };
                let _ = state.app_handle.emit("traffic_captured", &traffic);
                
                return Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("Content-Type", "text/plain")
                    .body(Full::new(Bytes::from("Response halted by MITM proxy (forwarded to server but blocked to client)")))
                    .unwrap());
            }

            // 2. Check Response Interception
            if intercept_config.enabled && 
               (intercept_config.mode == "both" || intercept_config.mode == "response") &&
               !intercept_config.ignored_methods.contains(&method.to_string()) &&
               (intercept_config.url_filter.is_empty() || url.contains(&intercept_config.url_filter)) 
            {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                let traffic = Traffic {
                    id: traffic_id.clone(),
                    method: method.to_string(),
                    url: url.clone(),
                    host: host.clone(),
                    status_code: status,
                    request_headers: request_headers.clone(),
                    response_headers: response_headers.clone(),
                    request_body: req_body_for_ui.clone(),
                    response_body: res_body_for_ui.clone(),
                    phase: "response".to_string(),
                    is_intercepted: true,
                    intercepted_at: Some(now),
                    duration_ms: Some(duration),
                };


                let (tx, rx) = tokio::sync::oneshot::channel();
                {
                    let mut intercept = state.intercept.lock().await;
                    intercept.pending.insert(traffic_id.clone(), tx);
                }

                let _ = state.app_handle.emit("traffic_captured", &traffic);

        if intercept_config.auto_focus {
            if let Some(window) = state.app_handle.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }

                if let Ok(action) = rx.await {
                    if action.drop.unwrap_or(false) {
                        return Ok(Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .body(Full::new(Bytes::from("Response dropped by proxy")))
                            .unwrap());
                    }

                    if let Some(s) = action.status_code { status = s; }
                    if let Some(h) = action.headers { response_headers = h; }
                    if let Some(b) = action.body { 
                        response_body_bytes = Bytes::from(b);
                        response_headers.retain(|(k, _)| k != "content-encoding");
                    }
                }
            }
            
            let traffic = Traffic {
                id: traffic_id,
                method: method.to_string(),
                url: url.clone(),
                host,
                status_code: status,
                request_headers,
                response_headers: response_headers.clone(),
                request_body: req_body_for_ui,
                response_body: res_body_for_ui,
                phase: "response".to_string(),
                is_intercepted: false,
                intercepted_at: None,
                duration_ms: Some(duration),
            };
            
            if filter_config.should_process(&url, &method.to_string(), status) {
                let _ = state.app_handle.emit("traffic_captured", &traffic);
            }

            // Remove content-length/transfer-encoding to let hyper recalculate
            response_headers.retain(|(k, _)| k != "content-length" && k != "transfer-encoding");

            let mut builder = Response::builder().status(status);
            for (k, v) in response_headers.iter() {
                builder = builder.header(k, v);
            }
            Ok(builder.body(Full::new(response_body_bytes)).unwrap())
        }
        Err(e) => {
            eprintln!("Outbound request error: {}", e);
            Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(format!("Proxy error: {}", e))))
                .unwrap())
        }
    }
}
