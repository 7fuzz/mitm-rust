use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub uuid: String,
    pub method: String,
    pub url: String,
    pub host: String,
    pub status_code: u16,
    pub request_headers: Vec<(String, String)>,
    pub response_headers: Vec<(String, String)>,
    pub request_body: String,
    pub response_body: String,
    pub phase: String,
    pub duration_ms: Option<u64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySummaryItem {
    pub id: i64,
    pub uuid: String,
    pub method: String,
    pub url: String,
    pub host: String,
    pub status_code: u16,
    pub duration_ms: Option<u64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDetailItem {
    pub id: i64,
    pub uuid: String,
    pub method: String,
    pub url: String,
    pub host: String,
    pub status_code: u16,
    pub request_headers: Vec<(String, String)>,
    pub response_headers: Vec<(String, String)>,
    pub request_body: String,
    pub response_body: String,
    pub request_body_hex: Option<String>,
    pub response_body_hex: Option<String>,
    pub phase: String,
    pub duration_ms: Option<u64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficCapturedEvent {
    pub entry: HistorySummaryItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub port: u16,
    pub host: String,
    pub is_running: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "127.0.0.1".to_string(),
            is_running: false,
        }
    }
}

pub struct AppState {
    pub db_path: PathBuf,
    pub history_tx: mpsc::Sender<HistoryEntry>,
    pub proxy_active: AtomicBool,
    pub broadcast_tx: broadcast::Sender<TrafficCapturedEvent>,
    pub proxy_config: Arc<Mutex<ProxyConfig>>,
    pub stop_signal: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl AppState {
    pub fn new(
        db_path: PathBuf,
        history_tx: mpsc::Sender<HistoryEntry>,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(500);
        Self {
            db_path,
            history_tx,
            proxy_active: AtomicBool::new(false),
            broadcast_tx,
            proxy_config: Arc::new(Mutex::new(ProxyConfig::default())),
            stop_signal: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_proxy_active(&self, active: bool) {
        self.proxy_active.store(active, Ordering::SeqCst);
    }

    pub fn is_proxy_active(&self) -> bool {
        self.proxy_active.load(Ordering::SeqCst)
    }
}
