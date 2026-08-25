pub mod mitm;
pub mod intercept;
pub mod rules;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tauri::AppHandle;

use crate::ca::RootCa;
use crate::state::{AppState, HistoryEntry, HistorySummaryItem};

pub async fn start_proxy_server(
    app_handle: AppHandle,
    state: Arc<AppState>,
    ca: Arc<RootCa>,
    addr_str: String,
    stop_rx: oneshot::Receiver<()>,
) -> Result<(), String> {
    let addr: SocketAddr = addr_str
        .parse()
        .map_err(|e| format!("Invalid bind address '{}': {}", addr_str, e))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind proxy listener to {}: {}", addr, e))?;

    tokio::select! {
        _ = async {
            loop {
                match listener.accept().await {
                    Ok((stream, client_addr)) => {
                        let app_handle_clone = app_handle.clone();
                        let state_clone = Arc::clone(&state);
                        let ca_clone = Arc::clone(&ca);
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(app_handle_clone, state_clone, ca_clone, stream, client_addr).await {
                                eprintln!("Error handling proxy connection from {}: {}", client_addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Proxy accept error: {}", e);
                    }
                }
            }
        } => {}
        _ = stop_rx => {
            println!("Proxy server on {} stopped gracefully", addr);
        }
    }

    Ok(())
}

async fn handle_connection(
    _app_handle: AppHandle,
    _state: Arc<AppState>,
    _ca: Arc<RootCa>,
    _stream: TcpStream,
    _client_addr: SocketAddr,
) -> Result<(), String> {
    Ok(())
}

pub fn create_history_entry_payload(
    id: i64,
    uuid: String,
    method: String,
    url: String,
    host: String,
    status_code: u16,
    request_headers: Vec<(String, String)>,
    response_headers: Vec<(String, String)>,
    request_body: String,
    response_body: String,
    duration_ms: Option<u64>,
) -> (HistoryEntry, HistorySummaryItem) {
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    let entry = HistoryEntry {
        id,
        uuid: uuid.clone(),
        method: method.clone(),
        url: url.clone(),
        host: host.clone(),
        status_code,
        request_headers,
        response_headers,
        request_body,
        response_body,
        phase: "response".to_string(),
        duration_ms,
        created_at: now.clone(),
    };

    let summary = HistorySummaryItem {
        id,
        uuid,
        method,
        url,
        host,
        status_code,
        duration_ms,
        created_at: now,
    };

    (entry, summary)
}
