use serde::{Deserialize, Serialize};

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
