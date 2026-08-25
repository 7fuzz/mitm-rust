use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use crate::proxy;
use crate::repeater::{RepeaterGroup, RepeaterRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableValue {
    pub id: String,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalVariable {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub active_index: i32,
    pub order_index: i32,
    pub values: Vec<VariableValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Replacement {
    pub id: String,
    pub r_type: String, 
    pub pattern: String,
    pub replacement: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub order_index: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncData {
    pub history: Vec<proxy::Traffic>,
    pub repeater_groups: Vec<RepeaterGroup>,
    pub repeater_requests: Vec<RepeaterRequest>,
    pub environments: Vec<Environment>,
    pub variables: Vec<GlobalVariable>,
    pub replacements: Vec<Replacement>,
    pub prefs: serde_json::Value,
    pub ui_layout: serde_json::Value,
    pub toolkit_json: String,
    pub history_limits: serde_json::Value,
    pub active_group_id: Option<String>,
    pub active_env_id: Option<String>,
    pub filter_config: proxy::FilterConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfig {
    pub bindings: Vec<String>,
    pub proxy_mode: String, // "off", "normal", "halt_client", "halt_all"
}

pub struct ProxyManager {
    pub active_listeners: HashMap<String, tokio::sync::oneshot::Sender<()>>,
    pub config: ProxyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsMessage {
    pub id: String,
    pub connection_id: String,
    pub direction: String,
    pub msg_type: String,
    pub payload: String,
    pub timestamp: u64,
    pub is_intercepted: bool,
}

#[derive(Debug, Deserialize)]
pub struct WsResumeAction {
    pub drop: Option<bool>,
    pub payload: Option<String>,
}

pub struct ActiveWsConnection {
    pub to_client_tx: tokio::sync::mpsc::UnboundedSender<tokio_tungstenite::tungstenite::Message>,
    pub to_server_tx: tokio::sync::mpsc::UnboundedSender<tokio_tungstenite::tungstenite::Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookEndpoint {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub path_slug: String,
    pub mock_status: i32,
    pub mock_headers: String,
    pub mock_body: String,
    pub auto_forward_url: Option<String>,
    pub is_active: bool,
    #[serde(default)]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookDelivery {
    pub id: String,
    pub endpoint_id: Option<String>,
    pub method: String,
    pub path: String,
    pub headers: String,
    pub query_params: String,
    pub body: String,
    pub client_ip: Option<String>,
    pub forwarded: bool,
    pub forward_status: Option<i32>,
    pub forward_response_body: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookTriggerRequest {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub signature_secret: Option<String>,
    pub provider_preset: Option<String>, // "stripe", "github", "shopify", "slack", "custom"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookForwardResult {
    pub success: bool,
    pub status_code: Option<i32>,
    pub response_body: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookListenerConfig {
    pub port: u16,
    pub is_running: bool,
}

pub struct WebhookManager {
    pub active_listener: Option<tokio::sync::oneshot::Sender<()>>,
    pub config: WebhookListenerConfig,
}

pub struct AppState {
    pub proxy_manager: Arc<Mutex<ProxyManager>>,
    pub intercept_state: Arc<Mutex<proxy::InterceptState>>,
    pub active_websockets: Arc<Mutex<HashMap<String, ActiveWsConnection>>>,
    pub pending_ws: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<WsResumeAction>>>>,
    pub webhook_manager: Arc<Mutex<WebhookManager>>,
}
