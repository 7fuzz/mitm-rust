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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfig {
    pub bindings: Vec<String>,
    pub enabled: bool,
}

pub struct ProxyManager {
    pub active_listeners: HashMap<String, tokio::sync::oneshot::Sender<()>>,
    pub config: ProxyConfig,
}

pub struct AppState {
    pub proxy_manager: Arc<Mutex<ProxyManager>>,
    pub intercept_state: Arc<Mutex<proxy::InterceptState>>,
}
