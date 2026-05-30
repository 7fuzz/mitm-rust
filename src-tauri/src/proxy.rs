use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::rustls::{self, pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer}};
use tokio_rustls::TlsAcceptor;
use hyper::{Request, Response, StatusCode, Method};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use http_body_util::{BodyExt, Full};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;
use rcgen::{CertificateParams, KeyPair, DnType, IsCa, BasicConstraints};

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
}

pub struct ProxyState {
    pub app_handle: AppHandle,
    pub ca: CA,
}

pub async fn start_proxy(app_handle: AppHandle, ca: CA, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = Arc::new(ProxyState { app_handle, ca });
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
    
    let cert_key_pair = KeyPair::generate().unwrap();
    let cert = params.signed_by(&cert_key_pair, &state.ca.cert, &state.ca.key_pair).unwrap();
    
    let cert_der = cert.der().clone();
    let key_der = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(cert_key_pair.serialize_der()));

    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)?;
    
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let tls_stream = acceptor.accept(upgraded_io).await?;
    
    let io = TokioIo::new(tls_stream);
    
    let service = service_fn(move |mut req| {
        let state = Arc::clone(&state);
        let host = host.clone();
        async move {
            let uri = format!("https://{}{}", host, req.uri());
            *req.uri_mut() = uri.parse().unwrap();
            handle_http(req, state).await
        }
    });

    if let Err(_err) = hyper::server::conn::http1::Builder::new()
        .serve_connection(io, service)
        .await 
    {
    }

    Ok(())
}

async fn handle_http(
    req: Request<Incoming>,
    state: Arc<ProxyState>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build(HttpConnector::new());
    
    let method = req.method().clone();
    let url = req.uri().to_string();
    let host = req.uri().host().unwrap_or_default().to_string();
    let request_headers = headers_to_map(req.headers());

    let (parts, body) = req.into_parts();
    let new_req = Request::from_parts(parts, body);
    
    match client.request(new_req).await {
        Ok(res) => {
            let status = res.status().as_u16();
            let response_headers = headers_to_map(res.headers());
            
            let traffic = Traffic {
                id: Uuid::new_v4().to_string(),
                method: method.to_string(),
                url,
                host,
                status_code: status,
                request_headers,
                response_headers,
                request_body: String::new(),
                response_body: String::new(),
                phase: "response".to_string(),
            };
            
            let _ = state.app_handle.emit("traffic_captured", &traffic);

            let (parts, body) = res.into_parts();
            let collected = body.collect().await?.to_bytes();
            Ok(Response::from_parts(parts, Full::new(collected)))
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
