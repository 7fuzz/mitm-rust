mod view;

use std::path::PathBuf;
use mitm_core::state::WebhookEndpoint;

pub use view::show;

#[derive(Clone, PartialEq)]
pub enum EndpointFilter {
    All,
    Unmatched,
    Endpoint(String),
}

#[derive(Default)]
pub struct EndpointDraft {
    pub name: String,
    pub path: String,
    pub secret: String,
}

pub struct WebhooksState {
    pub running: bool,
    pub port: u16,
    pub endpoints: Vec<WebhookEndpoint>,
    pub filter: EndpointFilter,
    pub draft: Option<EndpointDraft>,
}

impl Default for WebhooksState {
    fn default() -> Self {
        Self {
            running: false,
            port: 9000,
            endpoints: Vec::new(),
            filter: EndpointFilter::All,
            draft: None,
        }
    }
}

impl WebhooksState {
    pub fn load(&mut self, db_path: &PathBuf) {
        if let Ok(endpoints) = mitm_core::db::webhook_db::load_webhook_endpoints(db_path) {
            self.endpoints = endpoints;
        }
    }

    pub fn base_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    pub fn endpoint_url(&self, path: &str) -> String {
        format!("{}{}", self.base_url(), path)
    }

    pub fn created(&mut self, endpoint: WebhookEndpoint) {
        self.endpoints.push(endpoint);
        self.draft = None;
    }

    pub fn removed(&mut self, id: &str) {
        self.endpoints.retain(|e| e.id != id);
        if self.filter == EndpointFilter::Endpoint(id.to_string()) {
            self.filter = EndpointFilter::All;
        }
    }
}
