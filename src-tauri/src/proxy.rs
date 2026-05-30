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
use tauri::{AppHandle, Emitter};
use uuid::Uuid;
use rcgen::{CertificateParams, KeyPair, DnType};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ca::CA;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Traffic {
    pub id: String,
    pub method: String,
    pub url: String,
    pub host: String,
    pub status_code: u16,
    pub request_headers: HashMap<String, String>,
    pub response_headers: HashMap<String, String>,
    pub request_body: String,
    pub response_body: String,
    pub phase: String,
    pub is_intercepted: bool,
    pub intercepted_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptConfig {
    pub enabled: bool,
    pub mode: String, // "both", "request", "response"
    pub ignored_methods: Vec<String>,
    pub url_filter: String,
}

impl Default for InterceptConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "both".to_string(),
            ignored_methods: vec!["OPTIONS".to_string()],
            url_filter: "".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ResumeAction {
    pub drop: Option<bool>,
    pub method: Option<String>,
    pub url: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub status_code: Option<u16>,
    pub variables: Option<HashMap<String, String>>,
}

pub struct InterceptState {
    pub config: InterceptConfig,
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
    let mut request_headers = headers_to_map(req.headers());

    let (_parts, body) = req.into_parts();
    let collected_req_body = body.collect().await?.to_bytes();
    let mut request_body = String::from_utf8_lossy(&collected_req_body).to_string();

    // 1. Check Request Interception
    let intercept_config = {
        let intercept = state.intercept.lock().await;
        intercept.config.clone()
    };

    let traffic_id = Uuid::new_v4().to_string();

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
            response_headers: HashMap::new(),
            request_body: request_body.clone(),
            response_body: String::new(),
            phase: "request".to_string(),
            is_intercepted: true,
            intercepted_at: Some(now),
        };

        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut intercept = state.intercept.lock().await;
            intercept.pending.insert(traffic_id.clone(), tx);
        }

        let _ = state.app_handle.emit("traffic_captured", &traffic);

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
            if let Some(b) = action.body { request_body = b; }
        }
    }

    let mut new_req = Request::builder()
        .method(method.clone())
        .uri(url.clone());
    
    for (k, v) in request_headers.iter() {
        new_req = new_req.header(k, v);
    }
    
    let new_req = new_req.body(Full::new(Bytes::from(request_body.clone()))).unwrap();
    
    match client.request(new_req).await {
        Ok(res) => {
            let mut status = res.status().as_u16();
            let mut response_headers = headers_to_map(res.headers());
            let (_parts, body) = res.into_parts();
            let collected_res_body = body.collect().await?.to_bytes();
            let mut response_body = String::from_utf8_lossy(&collected_res_body).to_string();

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
                    request_body: request_body.clone(),
                    response_body: response_body.clone(),
                    phase: "response".to_string(),
                    is_intercepted: true,
                    intercepted_at: Some(now),
                };

                let (tx, rx) = tokio::sync::oneshot::channel();
                {
                    let mut intercept = state.intercept.lock().await;
                    intercept.pending.insert(traffic_id.clone(), tx);
                }

                let _ = state.app_handle.emit("traffic_captured", &traffic);

                if let Ok(action) = rx.await {
                    if action.drop.unwrap_or(false) {
                        return Ok(Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .body(Full::new(Bytes::from("Response dropped by proxy")))
                            .unwrap());
                    }

                    if let Some(s) = action.status_code { status = s; }
                    if let Some(h) = action.headers { response_headers = h; }
                    if let Some(b) = action.body { response_body = b; }
                }
            }
            
            let traffic = Traffic {
                id: traffic_id,
                method: method.to_string(),
                url,
                host,
                status_code: status,
                request_headers,
                response_headers: response_headers.clone(),
                request_body,
                response_body: response_body.clone(),
                phase: "response".to_string(),
                is_intercepted: false,
                intercepted_at: None,
            };
            
            let _ = state.app_handle.emit("traffic_captured", &traffic);

            let mut builder = Response::builder().status(status);
            for (k, v) in response_headers.iter() {
                builder = builder.header(k, v);
            }
            Ok(builder.body(Full::new(Bytes::from(response_body))).unwrap())
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

fn headers_to_map(headers: &hyper::HeaderMap) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (name, value) in headers.iter() {
        map.insert(
            name.to_string(),
            value.to_str().unwrap_or("").to_string(),
        );
    }
    map
}
