mod view;

use std::path::PathBuf;
use mitm_core::state::{WebhookDelivery, WebhookDeliveryCapturedEvent, WebhookEndpoint};
use serde::{Deserialize, Serialize};

pub use view::show;

/// Cap on deliveries held in memory, so a busy receiver can't grow RAM without bound.
const MAX_DELIVERIES: usize = 1000;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebhooksPrefs {
    pub list_split: f32,
}

impl Default for WebhooksPrefs {
    fn default() -> Self {
        Self { list_split: 0.3 }
    }
}

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
    pub prefs: WebhooksPrefs,
    pub running: bool,
    pub port: u16,
    pub endpoints: Vec<WebhookEndpoint>,
    pub deliveries: Vec<WebhookDelivery>,
    pub filter: EndpointFilter,
    pub search: String,
    pub draft: Option<EndpointDraft>,
}

impl Default for WebhooksState {
    fn default() -> Self {
        Self {
            prefs: WebhooksPrefs::default(),
            running: false,
            port: 9000,
            endpoints: Vec::new(),
            deliveries: Vec::new(),
            filter: EndpointFilter::All,
            search: String::new(),
            draft: None,
        }
    }
}

impl WebhooksState {
    pub fn load(&mut self, db_path: &PathBuf) {
        if let Ok(endpoints) = mitm_core::db::webhook_db::load_webhook_endpoints(db_path) {
            self.endpoints = endpoints;
        }
        if let Ok(deliveries) = mitm_core::db::webhook_db::load_webhook_deliveries(db_path, None) {
            self.deliveries = deliveries;
        }
    }

    /// A live delivery from the receiver: newest first, capped, with its endpoint's hit count bumped.
    pub fn apply_delivery(&mut self, event: WebhookDeliveryCapturedEvent) {
        if let Some(endpoint) = self.endpoints.iter_mut().find(|e| e.id == event.delivery.endpoint_id) {
            endpoint.hit_count = event.endpoint_hit_count;
        }
        self.deliveries.insert(0, event.delivery);
        self.deliveries.truncate(MAX_DELIVERIES);
    }

    pub fn deliveries_cleared(&mut self) {
        self.deliveries.clear();
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
        self.deliveries.retain(|d| d.endpoint_id != id);
        if self.filter == EndpointFilter::Endpoint(id.to_string()) {
            self.filter = EndpointFilter::All;
        }
    }
}
