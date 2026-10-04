use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    pub id: i64,
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
    #[serde(default)]
    pub listener_label: String,
    /// When the proxy received the request
    #[serde(default)]
    pub request_at: Option<String>,
    /// When the exchange finished (response delivered, dropped, or failed)
    #[serde(default)]
    pub response_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDetail {
    pub id: i64,
    pub request_headers: Vec<(String, String)>,
    pub response_headers: Vec<(String, String)>,
    pub request_body: String,
    pub response_body: String,
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
    #[serde(default)]
    pub ws_mitm_enabled: bool,
    /// Which listeners' traffic may be intercepted
    #[serde(default)]
    pub intercept_source_scope: SourceScope,
}

/// Restricts a feature to traffic from selected proxy listeners (sources).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceScope {
    /// When true every source matches and `ids` is ignored
    #[serde(default = "default_true")]
    pub all: bool,
    /// Listener ids that match when `all` is false
    #[serde(default)]
    pub ids: Vec<u32>,
}

impl Default for SourceScope {
    fn default() -> Self {
        Self { all: true, ids: Vec::new() }
    }
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
            ws_mitm_enabled: false,
            intercept_source_scope: SourceScope::default(),
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
    pub id: i64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookEndpoint {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub secret_key: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub hit_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookDelivery {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub endpoint_id: String,
    pub endpoint_path: String,
    #[serde(default)]
    pub timestamp: i64,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub payload: String,
    #[serde(default)]
    pub signature_status: String,
    pub computed_hmac: Option<String>,
    pub provided_hmac: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookListenerConfig {
    pub port: u16,
    pub is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSignatureResult {
    pub header_name: String,
    pub header_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookDeliveryCapturedEvent {
    pub delivery: WebhookDelivery,
    pub endpoint_hit_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketConn {
    pub connection_id: i64,
    pub url: String,
    pub status: String,
    pub handshake_time: i64,
    #[serde(default)]
    pub closed_at: Option<i64>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub client_addr: Option<String>,
    #[serde(default)]
    pub is_client_session: bool,
    #[serde(default)]
    pub message_count: u64,
    #[serde(default)]
    pub listener_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub id: i64,
    pub connection_id: i64,
    pub direction: String, // "to_client" | "to_server"
    pub msg_type: String,  // "text" | "json" | "binary"
    pub payload: String,
    pub timestamp: i64,
    #[serde(default)]
    pub length: usize,
    #[serde(default)]
    pub is_injected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketMessageCapturedEvent {
    pub message: WebSocketMessage,
    pub message_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketConnectionEvent {
    pub connection: WebSocketConn,
    pub event_type: String, // "opened" | "closed"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListenerConfig {
    pub id: u32,
    pub label: String,
    pub address: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Listener config plus its runtime status, returned to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListenerStatus {
    #[serde(flatten)]
    pub config: ListenerConfig,
    pub running: bool,
    pub error: Option<String>,
}

pub struct ListenerInstance {
    pub id: u32,
    pub stop_tx: Option<oneshot::Sender<()>>,
}

/// Normalizes "localhost:8080", "8080", "0.0.0.0:8080" etc. into a canonical "ip:port".
pub fn normalize_listener_address(input: &str) -> Result<String, String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err("Listener address cannot be empty".to_string());
    }
    let (host, port_str) = match raw.rfind(':') {
        Some(pos) => (raw[..pos].trim(), raw[pos + 1..].trim()),
        None => ("", raw),
    };
    let port = port_str
        .parse::<u16>()
        .map_err(|_| format!("Invalid port '{}' in address '{}'", port_str, raw))?;
    if port == 0 {
        return Err("Port must be between 1 and 65535".to_string());
    }
    let host = match host {
        "" => "0.0.0.0",
        h if h.eq_ignore_ascii_case("localhost") => "127.0.0.1",
        h => h,
    };
    let candidate = if host.contains(':') && !host.starts_with('[') {
        format!("[{}]:{}", host, port)
    } else {
        format!("{}:{}", host, port)
    };
    candidate
        .parse::<std::net::SocketAddr>()
        .map(|a| a.to_string())
        .map_err(|_| format!("Invalid bind address '{}': host must be an IP address or 'localhost'", raw))
}

/// Loads listener configs from preferences, migrating older formats (non-numeric ids,
/// missing Default listener stored as proxy_host/proxy_port).
fn load_listener_configs(db_path: &PathBuf, legacy_host: &str, legacy_port: u16) -> Vec<ListenerConfig> {
    let raw: Vec<serde_json::Value> = crate::db::get_preference(db_path, "proxy_listeners")
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();

    let mut configs: Vec<ListenerConfig> = Vec::new();
    let mut pending_ids: Vec<usize> = Vec::new();
    for v in raw {
        let label = v.get("label").and_then(|l| l.as_str()).unwrap_or("").trim().to_string();
        let address = v.get("address").and_then(|a| a.as_str()).unwrap_or("");
        let Ok(address) = normalize_listener_address(address) else { continue };
        if label.is_empty() {
            continue;
        }
        let enabled = v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
        let id = v.get("id").and_then(|i| i.as_u64()).map(|i| i as u32);
        match id {
            Some(id) if !configs.iter().any(|c| c.id == id) => {
                configs.push(ListenerConfig { id, label, address, enabled });
            }
            _ => {
                pending_ids.push(configs.len());
                configs.push(ListenerConfig { id: u32::MAX, label, address, enabled });
            }
        }
    }

    if !configs.iter().any(|c| c.id == 0) {
        let address = normalize_listener_address(&format!("{}:{}", legacy_host, legacy_port))
            .unwrap_or_else(|_| format!("0.0.0.0:{}", legacy_port));
        configs.insert(0, ListenerConfig { id: 0, label: "Default".to_string(), address, enabled: true });
        pending_ids.iter_mut().for_each(|i| *i += 1);
    }

    let mut next_id = configs.iter().filter(|c| c.id != u32::MAX).map(|c| c.id).max().unwrap_or(0) + 1;
    for idx in pending_ids {
        configs[idx].id = next_id;
        next_id += 1;
    }

    configs.sort_by_key(|c| c.id);

    // Disable stored listeners that clash with an earlier one (same address, or a port owned by 0.0.0.0)
    let mut taken: Vec<std::net::SocketAddr> = Vec::new();
    for c in configs.iter_mut().filter(|c| c.enabled) {
        let Ok(addr) = c.address.parse::<std::net::SocketAddr>() else { continue };
        let clashes = taken.iter().any(|t| {
            t.port() == addr.port() && (t.ip() == addr.ip() || t.ip().is_unspecified() || addr.ip().is_unspecified())
        });
        if clashes {
            c.enabled = false;
        } else {
            taken.push(addr);
        }
    }
    configs
}

fn load_source_scope(db_path: &PathBuf, key: &str) -> SourceScope {
    crate::db::get_preference(db_path, key)
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    pub history_tx: tokio::sync::mpsc::Sender<HistoryEntry>,
    pub rewrite_tx: tokio::sync::mpsc::Sender<RewriteHistoryEntry>,
    pub proxy_active: Arc<AtomicBool>,
    pub broadcast_tx: broadcast::Sender<TrafficCapturedEvent>,
    pub proxy_config: Arc<RwLock<ProxyConfig>>,
    pub history_settings: Arc<RwLock<HistorySettings>>,
    pub listeners: Arc<RwLock<Vec<ListenerInstance>>>,
    pub listener_errors: Arc<DashMap<u32, String>>,
    pub listener_configs: Arc<RwLock<Vec<ListenerConfig>>>,
    pub pending_flows: Arc<DashMap<String, PendingFlow>>,
    pub rules: Arc<RwLock<Vec<InterceptRule>>>,
    pub rewrite_rules: Arc<RwLock<Vec<RewriteRule>>>,
    pub rewrite_enabled: Arc<AtomicBool>,
    pub rewrite_source_scope: Arc<RwLock<SourceScope>>,
    pub webhook_running: Arc<AtomicBool>,
    pub webhook_port: Arc<RwLock<u16>>,
    pub webhook_stop_signal: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub ws_stream_senders: Arc<DashMap<i64, (tokio::sync::mpsc::UnboundedSender<tungstenite::Message>, tokio::sync::mpsc::UnboundedSender<tungstenite::Message>)>>,
    pub ws_client_senders: Arc<DashMap<i64, tokio::sync::mpsc::UnboundedSender<tungstenite::Message>>>,
    pub ws_client_stops: Arc<DashMap<i64, Arc<Mutex<Option<oneshot::Sender<()>>>>>>,
    pub next_history_id: Arc<AtomicU64>,
    pub next_rewrite_history_id: Arc<AtomicU64>,
    pub next_fuzz_run_id: Arc<AtomicU64>,
    pub fuzz_cancel: Arc<AtomicBool>,
    pub fuzz_running: Arc<AtomicBool>,
    pub fuzz_buffer: Arc<RwLock<Option<crate::fuzzer::execute::FuzzRunBuffer>>>,
}

/// Highest id in `table`, so in-memory id counters resume after it
fn max_id(db_path: &PathBuf, table: &str) -> u64 {
    rusqlite::Connection::open(db_path)
        .and_then(|conn| {
            conn.query_row(&format!("SELECT COALESCE(MAX(id), 0) FROM {}", table), [], |row| row.get::<_, i64>(0))
        })
        .map(|id| id.max(0) as u64)
        .unwrap_or(0)
}

impl AppState {
    pub fn new(
        db_path: PathBuf,
        history_tx: tokio::sync::mpsc::Sender<HistoryEntry>,
        rewrite_tx: tokio::sync::mpsc::Sender<RewriteHistoryEntry>,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(500);

        let initial_next_history_id = max_id(&db_path, "history");
        let initial_next_rewrite_history_id = max_id(&db_path, "rewrite_history");
        let initial_next_fuzz_run_id = max_id(&db_path, "fuzz_runs");

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

        let initial_webhook_port = crate::db::get_preference(&db_path, "webhook_port")
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(9000);

        let limiter_enabled = crate::db::get_preference(&db_path, "history_limiter_enabled")
            .map(|v| v == "true")
            .unwrap_or(true);
        let max_rows = crate::db::get_preference(&db_path, "history_limiter_max_rows")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(500);

        let initial_ws_mitm_enabled = crate::db::get_preference(&db_path, "ws_mitm_enabled")
            .map(|v| v == "true")
            .unwrap_or(false);

        let initial_rules = crate::db::load_intercept_rules(&db_path).unwrap_or_default();
        let initial_rewrite_rules = crate::db::load_rewrite_rules(&db_path).unwrap_or_default();

        let initial_listener_configs = load_listener_configs(&db_path, &initial_host, initial_port);
        // Keep the legacy host/port fields mirroring listener #0
        let (initial_host, initial_port) = initial_listener_configs
            .iter()
            .find(|c| c.id == 0)
            .and_then(|c| c.address.parse::<std::net::SocketAddr>().ok())
            .map(|a| (a.ip().to_string(), a.port()))
            .unwrap_or((initial_host, initial_port));

        let proxy_config = ProxyConfig {
            proxy_enabled: initial_proxy_enabled,
            intercept_enabled: initial_intercept_enabled,
            intercept_mode: initial_intercept_mode,
            proxy_mode: initial_proxy_mode,
            port: initial_port,
            host: initial_host,
            ws_mitm_enabled: initial_ws_mitm_enabled,
            intercept_source_scope: load_source_scope(&db_path, "intercept_source_scope"),
        };

        let initial_rewrite_source_scope = load_source_scope(&db_path, "rewrite_source_scope");

        let history_settings = HistorySettings {
            limiter_enabled,
            max_rows,
        };

        Self {
            db_path,
            history_tx,
            rewrite_tx,
            proxy_active: Arc::new(AtomicBool::new(initial_proxy_enabled)),
            broadcast_tx,
            proxy_config: Arc::new(RwLock::new(proxy_config)),
            history_settings: Arc::new(RwLock::new(history_settings)),
            listeners: Arc::new(RwLock::new(Vec::new())),
            listener_errors: Arc::new(DashMap::new()),
            listener_configs: Arc::new(RwLock::new(initial_listener_configs)),
            pending_flows: Arc::new(DashMap::new()),
            rules: Arc::new(RwLock::new(initial_rules)),
            rewrite_rules: Arc::new(RwLock::new(initial_rewrite_rules)),
            rewrite_enabled: Arc::new(AtomicBool::new(initial_rewrite_enabled)),
            rewrite_source_scope: Arc::new(RwLock::new(initial_rewrite_source_scope)),
            webhook_running: Arc::new(AtomicBool::new(false)),
            webhook_port: Arc::new(RwLock::new(initial_webhook_port)),
            webhook_stop_signal: Arc::new(Mutex::new(None)),
            fuzz_cancel: Arc::new(AtomicBool::new(false)),
            fuzz_running: Arc::new(AtomicBool::new(false)),
            fuzz_buffer: Arc::new(RwLock::new(None)),
            ws_stream_senders: Arc::new(DashMap::new()),
            ws_client_senders: Arc::new(DashMap::new()),
            ws_client_stops: Arc::new(DashMap::new()),
            next_history_id: Arc::new(AtomicU64::new(initial_next_history_id)),
            next_rewrite_history_id: Arc::new(AtomicU64::new(initial_next_rewrite_history_id)),
            next_fuzz_run_id: Arc::new(AtomicU64::new(initial_next_fuzz_run_id)),
        }
    }

    pub fn next_history_id(&self) -> i64 {
        self.next_history_id.fetch_add(1, Ordering::SeqCst) as i64 + 1
    }

    pub fn next_rewrite_history_id(&self) -> i64 {
        self.next_rewrite_history_id.fetch_add(1, Ordering::SeqCst) as i64 + 1
    }

    pub fn next_fuzz_run_id(&self) -> i64 {
        self.next_fuzz_run_id.fetch_add(1, Ordering::SeqCst) as i64 + 1
    }

    pub fn set_proxy_active(&self, active: bool) {
        self.proxy_active.store(active, Ordering::SeqCst);
    }

    pub fn is_proxy_active(&self) -> bool {
        self.proxy_active.load(Ordering::SeqCst)
    }

    /// True when `listener_label` is covered by `scope`. Ids of removed listeners never match.
    pub async fn source_matches(&self, scope: &SourceScope, listener_label: &str) -> bool {
        if scope.all {
            return true;
        }
        self.listener_configs
            .read()
            .await
            .iter()
            .any(|c| scope.ids.contains(&c.id) && c.label == listener_label)
    }

    pub fn set_rewrite_enabled(&self, enabled: bool) {
        self.rewrite_enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn is_rewrite_enabled(&self) -> bool {
        self.rewrite_enabled.load(Ordering::SeqCst)
    }

    pub fn is_webhook_running(&self) -> bool {
        self.webhook_running.load(Ordering::SeqCst)
    }

    pub fn set_webhook_running(&self, running: bool) {
        self.webhook_running.store(running, Ordering::SeqCst);
    }
}
