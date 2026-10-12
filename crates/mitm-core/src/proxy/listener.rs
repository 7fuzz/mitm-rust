use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use super::rules::SourceScope;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub proxy_enabled: bool,
    pub intercept_enabled: bool,
    pub intercept_mode: String, // 'request', 'response', 'both'
    pub proxy_mode: String,     // 'on', 'off', 'block_client', 'block'
    #[serde(default)]
    pub ws_mitm_enabled: bool,
    /// Which listeners' traffic may be intercepted
    #[serde(default)]
    pub intercept_source_scope: SourceScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyMode {
    On,
    Off,
    /// Requests reach the server but responses never reach the client
    BlockClient,
    /// Requests are answered by the proxy and never leave
    BlockAll,
}

impl ProxyMode {
    pub const ALL: [ProxyMode; 4] = [ProxyMode::On, ProxyMode::Off, ProxyMode::BlockClient, ProxyMode::BlockAll];

    pub fn as_str(self) -> &'static str {
        match self {
            ProxyMode::On => "on",
            ProxyMode::Off => "off",
            ProxyMode::BlockClient => "block_client",
            ProxyMode::BlockAll => "block",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.as_str() == value)
    }

    pub fn is_enabled(self) -> bool {
        self != ProxyMode::Off
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            proxy_enabled: true,
            intercept_enabled: false,
            intercept_mode: "request".to_string(),
            proxy_mode: "on".to_string(),
            ws_mitm_enabled: false,
            intercept_source_scope: SourceScope::default(),
        }
    }
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

pub(crate) fn default_true() -> bool {
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
