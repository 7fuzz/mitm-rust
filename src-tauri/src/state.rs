use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::{broadcast, oneshot, Mutex, RwLock};
use serde::{Deserialize, Serialize};
use crate::db::InterceptRule;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InterceptPhase {
    Request,
    Response,
}

#[derive(Debug)]
pub enum InterceptAction {
    Forward {
        modified_url: Option<String>,
        modified_method: Option<String>,
        modified_headers: Option<Vec<(String, String)>>,
        modified_body: Option<Vec<u8>>,
    },
    Drop,
}

pub struct PendingFlow {
    pub flow_id: String,
    pub phase: InterceptPhase,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub tx: oneshot::Sender<InterceptAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingFlowPayload {
    pub flow_id: String,
    pub phase: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub body_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub method: String,
    pub url: String,
    pub host: String,
    pub path: String,
    pub content_type: String,
    pub response_size: u64,
    pub status_code: u16,
    pub request_headers: Vec<(String, String)>,
    pub response_headers: Vec<(String, String)>,
    pub request_body: String,
    pub response_body: String,
    pub phase: String,
    pub duration_ms: Option<u64>,
    pub created_at: String,
    pub is_intercepted: bool,
    pub is_rewritten: bool,
    pub is_failed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySettings {
    pub limiter_enabled: bool,
    pub max_rows: u32,
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self {
            limiter_enabled: true,
            max_rows: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficCapturedEvent {
    pub entry: HistoryEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub proxy_enabled: bool,
    pub intercept_enabled: bool,
    pub intercept_mode: String, // 'request', 'response', 'both'
    pub proxy_mode: String,     // 'on', 'off', 'block_client', 'block'
    pub port: u16,
    pub host: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            proxy_enabled: true,
            intercept_enabled: false,
            intercept_mode: "request".to_string(),
            proxy_mode: "on".to_string(),
            port: 8080,
            host: "0.0.0.0".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub action_type: String, // "partial_request" | "full_request" | "partial_response" | "full_response"
    pub match_field: String, // "all" | "url" | "host" | "path" | "method" | "header"
    pub match_operator: String, // "contains" | "equals" | "regex" | "starts_with"
    pub match_value: String,
    pub target_part: String, // "url" | "query" | "header" | "body" | "method" | "status"
    pub target_header: Option<String>,
    pub match_pattern: String,
    pub replacement_value: String,
    pub is_regex: bool,
    pub mock_status_code: Option<u16>,
    pub mock_headers_json: Option<String>,
    pub mock_body: Option<String>,
    pub order_index: i32,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteHistoryEntry {
    pub id: String,
    pub rule_id: Option<String>,
    pub rule_name: String,
    pub action_type: String,
    pub method: String,
    pub original_url: String,
    pub rewritten_url: String,
    pub original_headers: Vec<(String, String)>,
    pub rewritten_headers: Vec<(String, String)>,
    pub original_body: String,
    pub rewritten_body: String,
    pub status_code: Option<u16>,
    pub duration_ms: Option<u64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteCapturedEvent {
    pub entry: RewriteHistoryEntry,
}

pub struct AppState {
    pub db_path: PathBuf,
    pub history_tx: tokio::sync::mpsc::Sender<HistoryEntry>,
    pub rewrite_tx: tokio::sync::mpsc::Sender<RewriteHistoryEntry>,
    pub proxy_active: AtomicBool,
    pub broadcast_tx: broadcast::Sender<TrafficCapturedEvent>,
    pub proxy_config: Arc<RwLock<ProxyConfig>>,
    pub history_settings: Arc<RwLock<HistorySettings>>,
    pub stop_signal: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub pending_flows: Arc<DashMap<String, PendingFlow>>,
    pub rules: Arc<RwLock<Vec<InterceptRule>>>,
    pub rewrite_rules: Arc<RwLock<Vec<RewriteRule>>>,
    pub rewrite_enabled: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(
        db_path: PathBuf,
        history_tx: tokio::sync::mpsc::Sender<HistoryEntry>,
        rewrite_tx: tokio::sync::mpsc::Sender<RewriteHistoryEntry>,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(500);

        let initial_proxy_mode = crate::db::get_preference(&db_path, "proxy_mode")
            .unwrap_or_else(|| "on".to_string());

        let initial_proxy_enabled = initial_proxy_mode != "off";
        let initial_intercept_enabled = crate::db::get_preference(&db_path, "intercept_enabled")
            .map(|v| v == "true")
            .unwrap_or(false);
        let initial_intercept_mode = crate::db::get_preference(&db_path, "intercept_mode")
            .unwrap_or_else(|| "both".to_string());

        let initial_rewrite_enabled = crate::db::get_preference(&db_path, "rewrite_enabled")
            .map(|v| v == "true")
            .unwrap_or(true);

        let initial_host = crate::db::get_preference(&db_path, "proxy_host")
            .unwrap_or_else(|| "0.0.0.0".to_string());
        let initial_port = crate::db::get_preference(&db_path, "proxy_port")
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8080);

        let limiter_enabled = crate::db::get_preference(&db_path, "history_limiter_enabled")
            .map(|v| v == "true")
            .unwrap_or(true);
        let max_rows = crate::db::get_preference(&db_path, "history_limiter_max_rows")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(500);

        let initial_rules = crate::db::load_intercept_rules(&db_path).unwrap_or_default();
        let initial_rewrite_rules = crate::db::load_rewrite_rules(&db_path).unwrap_or_default();

        let proxy_config = ProxyConfig {
            proxy_enabled: initial_proxy_enabled,
            intercept_enabled: initial_intercept_enabled,
            intercept_mode: initial_intercept_mode,
            proxy_mode: initial_proxy_mode,
            port: initial_port,
            host: initial_host,
        };

        let history_settings = HistorySettings {
            limiter_enabled,
            max_rows,
        };

        Self {
            db_path,
            history_tx,
            rewrite_tx,
            proxy_active: AtomicBool::new(initial_proxy_enabled),
            broadcast_tx,
            proxy_config: Arc::new(RwLock::new(proxy_config)),
            history_settings: Arc::new(RwLock::new(history_settings)),
            stop_signal: Arc::new(Mutex::new(None)),
            pending_flows: Arc::new(DashMap::new()),
            rules: Arc::new(RwLock::new(initial_rules)),
            rewrite_rules: Arc::new(RwLock::new(initial_rewrite_rules)),
            rewrite_enabled: Arc::new(AtomicBool::new(initial_rewrite_enabled)),
        }
    }

    pub fn set_proxy_active(&self, active: bool) {
        self.proxy_active.store(active, Ordering::SeqCst);
    }

    pub fn is_proxy_active(&self) -> bool {
        self.proxy_active.load(Ordering::SeqCst)
    }

    pub fn set_rewrite_enabled(&self, enabled: bool) {
        self.rewrite_enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn is_rewrite_enabled(&self) -> bool {
        self.rewrite_enabled.load(Ordering::SeqCst)
    }
}
