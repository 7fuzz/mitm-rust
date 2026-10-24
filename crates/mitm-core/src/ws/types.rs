use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub container_label: String,
    #[serde(default)]
    pub run_id: i64,
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
