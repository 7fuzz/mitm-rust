use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::{broadcast, oneshot, Mutex, RwLock};
use crate::db::InterceptRule;
use crate::proxy::history::{HistorySettings, TrafficCapturedEvent};
use crate::proxy::intercept::PendingFlow;
use crate::proxy::listener::{normalize_listener_address, ListenerConfig, ListenerInstance, ProxyConfig};
use crate::proxy::rewrite::{RewriteHistoryEntry, RewriteRule};
use crate::proxy::rules::SourceScope;
use crate::proxy::history::HistoryEntry;
use crate::db::Db;

/// Loads listener configs from preferences, assigning ids to entries saved without one
/// and recreating listener #0 when it's missing.
fn load_listener_configs(db: &Db) -> Vec<ListenerConfig> {
    let raw: Vec<serde_json::Value> = crate::db::get_preference(db, "proxy_listeners")
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
        configs.insert(0, ListenerConfig { id: 0, label: "default".to_string(), address: "127.0.0.1:8080".to_string(), enabled: true });
        pending_ids.iter_mut().for_each(|i| *i += 1);
    }

    let first_free = configs.iter().filter(|c| c.id != u32::MAX).map(|c| c.id).max().unwrap_or(0) + 1;
    for (id, idx) in (first_free..).zip(pending_ids) {
        configs[idx].id = id;
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

fn load_source_scope(db: &Db, key: &str) -> SourceScope {
    crate::db::get_preference(db, key)
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

type WsSender = tokio::sync::mpsc::UnboundedSender<tungstenite::Message>;
type StopSignal = Arc<Mutex<Option<oneshot::Sender<()>>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
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
    pub webhook_stop_signal: StopSignal,
    /// Per proxied connection: (to server, to client).
    pub ws_stream_senders: Arc<DashMap<i64, (WsSender, WsSender)>>,
    pub ws_client_senders: Arc<DashMap<i64, WsSender>>,
    pub ws_client_stops: Arc<DashMap<i64, StopSignal>>,
    pub next_history_id: Arc<AtomicU64>,
    pub next_rewrite_history_id: Arc<AtomicU64>,
    pub next_fuzz_run_id: Arc<AtomicU64>,
    pub fuzz: Arc<crate::fuzzer::execute::FuzzControl>,
}

/// Highest id in `table`, so in-memory id counters resume after it
fn max_id(db: &Db, table: &str) -> u64 {
    db.conn()
        .and_then(|conn| {
            conn.query_row(&format!("SELECT COALESCE(MAX(id), 0) FROM {}", table), [], |row| row.get::<_, i64>(0))
        })
        .map(|id| id.max(0) as u64)
        .unwrap_or(0)
}

impl AppState {
    pub fn new(
        db: Db,
        history_tx: tokio::sync::mpsc::Sender<HistoryEntry>,
        rewrite_tx: tokio::sync::mpsc::Sender<RewriteHistoryEntry>,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(500);

        let initial_next_history_id = max_id(&db, "history");
        let initial_next_rewrite_history_id = max_id(&db, "rewrite_history");
        let initial_next_fuzz_run_id = max_id(&db, "fuzz_runs");

        let initial_proxy_mode = crate::db::get_preference(&db, "proxy_mode")
            .unwrap_or_else(|| "on".to_string());

        let initial_proxy_enabled = initial_proxy_mode != "off";
        let initial_intercept_enabled = crate::db::get_preference(&db, "intercept_enabled")
            .map(|v| v == "true")
            .unwrap_or(false);
        let initial_intercept_mode = crate::db::get_preference(&db, "intercept_mode")
            .unwrap_or_else(|| "both".to_string());

        let initial_rewrite_enabled = crate::db::get_preference(&db, "rewrite_enabled")
            .map(|v| v == "true")
            .unwrap_or(true);

        let initial_webhook_port = crate::db::get_preference(&db, "webhook_port")
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(9000);

        let limiter_enabled = crate::db::get_preference(&db, "history_limiter_enabled")
            .map(|v| v == "true")
            .unwrap_or(true);
        let max_rows = crate::db::get_preference(&db, "history_limiter_max_rows")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(500);

        let initial_ws_mitm_enabled = crate::db::get_preference(&db, "ws_mitm_enabled")
            .map(|v| v == "true")
            .unwrap_or(false);

        let initial_rules = crate::db::load_intercept_rules(&db).unwrap_or_default();
        let initial_rewrite_rules = crate::db::load_rewrite_rules(&db).unwrap_or_default();

        let initial_listener_configs = load_listener_configs(&db);

        let proxy_config = ProxyConfig {
            proxy_enabled: initial_proxy_enabled,
            intercept_enabled: initial_intercept_enabled,
            intercept_mode: initial_intercept_mode,
            proxy_mode: initial_proxy_mode,
            ws_mitm_enabled: initial_ws_mitm_enabled,
            intercept_source_scope: load_source_scope(&db, "intercept_source_scope"),
        };

        let initial_rewrite_source_scope = load_source_scope(&db, "rewrite_source_scope");

        let history_settings = HistorySettings {
            limiter_enabled,
            max_rows,
        };

        Self {
            db,
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
            fuzz: Arc::default(),
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

    pub fn reset_history_id(&self) {
        self.next_history_id.store(0, Ordering::SeqCst);
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

#[cfg(test)]
mod tests {
    use crate::proxy::listener::ProxyMode;

    #[test]
    fn proxy_modes_round_trip_through_their_stored_names() {
        for mode in ProxyMode::ALL {
            assert_eq!(ProxyMode::parse(mode.as_str()), Some(mode));
        }
        assert_eq!(ProxyMode::BlockAll.as_str(), "block");
        assert_eq!(ProxyMode::parse("nope"), None);
        assert!(!ProxyMode::Off.is_enabled());
    }
}
