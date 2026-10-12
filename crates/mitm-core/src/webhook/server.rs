use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use bytes::Bytes;
use hyper_util::rt::TokioIo;
use crate::events::{CoreEvent, EventBus};

use crate::state::AppState;
use crate::webhook::types::{WebhookDelivery, WebhookDeliveryCapturedEvent};
use crate::webhook::hmac::verify_signature;

/// Binds the port up front so the caller can report a busy or forbidden port.
pub async fn bind_webhook_listener(port: u16) -> Result<TcpListener, String> {
    TcpListener::bind(("0.0.0.0", port))
        .await
        .map_err(|e| format!("Could not listen on port {}: {}", port, e))
}

pub async fn start_webhook_listener_server(
    event_bus: Option<EventBus>,
    state: Arc<AppState>,
    listener: TcpListener,
    port: u16,
    mut stop_rx: oneshot::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("0.0.0.0:{}", port);
    eprintln!("[Webhook Server] Listening on http://{}", addr);

    state.set_webhook_running(true);
    *state.webhook_port.write().await = port;

    loop {
        tokio::select! {
            _ = &mut stop_rx => {
                eprintln!("[Webhook Server] Stopping listener on {}", addr);
                break;
            }
            accept_result = listener.accept() => {
                let (stream, _remote_addr) = match accept_result {
                    Ok(res) => res,
                    Err(e) => {
                        eprintln!("[Webhook Server] Accept error: {}", e);
                        continue;
                    }
                };

                let io = TokioIo::new(stream);
                let state_clone = Arc::clone(&state);
                let event_bus_clone = event_bus.clone();

                tokio::spawn(async move {
                    let service = service_fn(move |req| {
                        let state = Arc::clone(&state_clone);
                        let bus = event_bus_clone.clone();
                        async move {
                            handle_webhook_request(req, state, bus).await
                        }
                    });

                    if let Err(err) = http1::Builder::new()
                        .serve_connection(io, service)
                        .await
                    {
                        eprintln!("[Webhook Server] Error serving connection: {}", err);
                    }
                });
            }
        }
    }

    state.set_webhook_running(false);
    Ok(())
}

async fn handle_webhook_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    event_bus: Option<EventBus>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let _method = req.method().to_string();
    let uri = req.uri();
    let path = uri.path().to_string();

    let mut headers = Vec::new();
    for (k, v) in req.headers().iter() {
        headers.push((
            k.as_str().to_lowercase(),
            v.to_str().unwrap_or("").to_string(),
        ));
    }

    let (_parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes().to_vec();
    let payload_str = String::from_utf8(body_bytes.clone())
        .unwrap_or_else(|_| format!("<binary data {} bytes>", body_bytes.len()));

    let matched_endpoint = crate::db::webhook_db::get_webhook_endpoint_by_path(&state.db_path, &path).ok().flatten();

    let (endpoint_id, endpoint_path, secret_key, is_matched) = match &matched_endpoint {
        Some(ep) => (ep.id.clone(), ep.path.clone(), ep.secret_key.clone(), true),
        None => ("unmatched".to_string(), path.clone(), "".to_string(), false),
    };

    let (signature_status, computed_hmac, provided_hmac) = verify_signature(&headers, &body_bytes, &secret_key);

    let now = chrono::Local::now().timestamp_millis();

    let mut delivery = WebhookDelivery {
        id: 0,
        endpoint_id: endpoint_id.clone(),
        endpoint_path: endpoint_path.clone(),
        timestamp: now,
        headers,
        payload: payload_str,
        signature_status,
        computed_hmac,
        provided_hmac,
    };

    match crate::db::webhook_db::save_webhook_delivery(&state.db_path, &delivery) {
        Ok(id) => delivery.id = id,
        Err(e) => eprintln!("[Webhook] Failed to save delivery: {}", e),
    }

    let endpoint_hit_count = if is_matched {
        crate::db::webhook_db::increment_webhook_endpoint_hits(&state.db_path, &endpoint_id).unwrap_or(0)
    } else {
        0
    };

    if let Some(bus) = event_bus {
        bus.emit(CoreEvent::WebhookDelivery(WebhookDeliveryCapturedEvent {
            delivery: delivery.clone(),
            endpoint_hit_count,
        }));
    }

    let response_body = serde_json::json!({
        "status": "received",
        "deliveryId": delivery.id,
        "matched": is_matched,
        "endpoint": endpoint_path,
    });

    let res = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Server", "MITM-Developer-Studio-Webhook-Receiver")
        .body(Full::new(Bytes::from(response_body.to_string())))
        .unwrap();

    Ok(res)
}
