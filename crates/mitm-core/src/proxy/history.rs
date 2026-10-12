use serde::{Deserialize, Serialize};

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
