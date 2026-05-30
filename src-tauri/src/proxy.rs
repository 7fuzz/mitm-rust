use hudsucker::{async_trait, HttpHandler, RequestContext, ResponseContext};
use hyper::{Body, Request, Response, http::HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

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

#[derive(Clone)]
pub struct MitmHandler {
    pub app_handle: AppHandle,
}

fn headers_to_map(headers: &hyper::HeaderMap<HeaderValue>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (name, value) in headers.iter() {
        map.insert(
            name.to_string(),
            value.to_str().unwrap_or("").to_string(),
        );
    }
    map
}

#[async_trait]
impl HttpHandler for MitmHandler {
    async fn handle_request(
        &mut self,
        _ctx: &RequestContext,
        req: Request<Body>,
    ) -> Request<Body> {
        req
    }

    async fn handle_response(
        &mut self,
        _ctx: &RequestContext,
        res: Response<Body>,
    ) -> Response<Body> {
        let (parts, body) = res.into_parts();
        
        let traffic = Traffic {
            id: Uuid::new_v4().to_string(),
            method: _ctx.request_parts.method.to_string(),
            url: _ctx.request_parts.uri.to_string(),
            host: _ctx.request_parts.uri.host().unwrap_or("").to_string(),
            status_code: parts.status.as_u16(),
            request_headers: headers_to_map(&_ctx.request_parts.headers),
            response_headers: headers_to_map(&parts.headers),
            request_body: String::new(),
            response_body: String::new(),
            phase: "response".to_string(),
        };

        // Emit to frontend via Tauri
        let _ = self.app_handle.emit("traffic_captured", &traffic);

        Response::from_parts(parts, body)
    }
}
