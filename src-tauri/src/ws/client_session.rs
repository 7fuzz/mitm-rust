use std::sync::Arc;
use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::Message;
use url::Url;
use uuid::Uuid;

use crate::state::{AppState, WebSocketConn, WebSocketConnectionEvent, WebSocketMessage, WebSocketMessageCapturedEvent};

pub async fn connect_client_session(
    app_handle: AppHandle,
    state: Arc<AppState>,
    url_str: String,
    custom_headers: Option<Vec<(String, String)>>,
) -> Result<WebSocketConn, String> {
    let parsed_url = Url::parse(&url_str).map_err(|e| format!("Invalid WebSocket URL '{}': {}", url_str, e))?;

    let scheme = parsed_url.scheme();
    if scheme != "ws" && scheme != "wss" {
        return Err(format!("Invalid WebSocket scheme '{}'. Expected 'ws' or 'wss'", scheme));
    }

    let mut request_builder = Request::builder().uri(&url_str);

    let host_str = parsed_url.host_str().unwrap_or("localhost");
    let port_str = parsed_url.port().map(|p| format!(":{}", p)).unwrap_or_default();
    request_builder = request_builder.header("Host", format!("{}{}", host_str, port_str));

    if let Some(headers) = custom_headers {
        for (k, v) in headers {
            if !k.is_empty() && !v.is_empty() {
                request_builder = request_builder.header(k.as_str(), v.as_str());
            }
        }
    }

    let request = request_builder
        .body(())
        .map_err(|e| format!("Failed to build WebSocket handshake request: {}", e))?;

    let (ws_stream, response) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| format!("Failed to connect to WebSocket server at '{}': {}", url_str, e))?;

    let subprotocol = response
        .headers()
        .get("sec-websocket-protocol")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let conn_id = format!("ws-cli-{}", Uuid::new_v4());
    let now = chrono::Local::now().timestamp_millis();

    let conn_info = WebSocketConn {
        connection_id: conn_id.clone(),
        url: url_str.clone(),
        status: "connected".to_string(),
        handshake_time: now,
        closed_at: None,
        protocol: subprotocol,
        client_addr: Some("127.0.0.1 (Studio Client)".to_string()),
        is_client_session: true,
        message_count: 0,
    };

    crate::db::ws_db::save_ws_connection(&state.db_path, &conn_info)?;

    let _ = app_handle.emit(
        "websocket_connection_event",
        WebSocketConnectionEvent {
            connection: conn_info.clone(),
            event_type: "opened".to_string(),
        },
    );

    let (mut write_half, mut read_half) = ws_stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let (stop_tx, mut stop_rx) = oneshot::channel::<()>();

    state.ws_client_senders.insert(conn_id.clone(), tx);
    state.ws_client_stops.insert(conn_id.clone(), Arc::new(Mutex::new(Some(stop_tx))));

    let conn_id_clone = conn_id.clone();
    let db_path_clone = state.db_path.clone();
    let app_handle_clone = app_handle.clone();
    let state_clone = Arc::clone(&state);

    // Writer Task (outgoing to remote server)
    let conn_id_writer = conn_id.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = write_half.send(msg).await {
                eprintln!("[WS Client {}] Send error: {}", conn_id_writer, e);
                break;
            }
        }
    });

    let conn_info_task = conn_info.clone();

    // Reader Task (incoming from remote server)
    tauri::async_runtime::spawn(async move {
        tokio::select! {
            _ = async {
                while let Some(msg_result) = read_half.next().await {
                    match msg_result {
                        Ok(msg) => {
                            let (msg_type, payload_str) = match &msg {
                                Message::Text(text) => ("text", text.to_string()),
                                Message::Binary(bin) => {
                                    ("binary", format!("[Binary Frame: {} bytes] {}", bin.len(), hex::encode(bin)))
                                }
                                Message::Ping(p) => ("text", format!("[Ping Frame] {}", String::from_utf8_lossy(p))),
                                Message::Pong(p) => ("text", format!("[Pong Frame] {}", String::from_utf8_lossy(p))),
                                Message::Close(_) => {
                                    break;
                                }
                                Message::Frame(_) => continue,
                            };

                            let msg_record = WebSocketMessage {
                                id: format!("ws-msg-{}", Uuid::new_v4()),
                                connection_id: conn_id_clone.clone(),
                                direction: "to_client".to_string(),
                                msg_type: msg_type.to_string(),
                                payload: payload_str,
                                timestamp: chrono::Local::now().timestamp_millis(),
                                length: msg.len(),
                                is_injected: false,
                            };

                            let _ = crate::db::ws_db::save_ws_message(&db_path_clone, &msg_record);
                            let count = crate::db::ws_db::increment_ws_message_count(&db_path_clone, &conn_id_clone).unwrap_or(1);

                            let _ = app_handle_clone.emit(
                                "websocket_message_event",
                                WebSocketMessageCapturedEvent {
                                    message: msg_record,
                                    message_count: count,
                                },
                            );
                        }
                        Err(e) => {
                            eprintln!("[WS Client {}] Read error: {}", conn_id_clone, e);
                            break;
                        }
                    }
                }
            } => {}
            _ = &mut stop_rx => {}
        }

        // Connection cleanup
        let close_time = chrono::Local::now().timestamp_millis();
        let _ = crate::db::ws_db::update_ws_connection_status(&db_path_clone, &conn_id_clone, "disconnected", Some(close_time));

        state_clone.ws_client_senders.remove(&conn_id_clone);
        state_clone.ws_client_stops.remove(&conn_id_clone);

        let mut closed_conn = conn_info_task;
        closed_conn.status = "disconnected".to_string();
        closed_conn.closed_at = Some(close_time);

        let _ = app_handle_clone.emit(
            "websocket_connection_event",
            WebSocketConnectionEvent {
                connection: closed_conn,
                event_type: "closed".to_string(),
            },
        );
    });

    Ok(conn_info)
}

pub async fn disconnect_client_session(state: Arc<AppState>, connection_id: &str) -> Result<(), String> {
    if let Some((_, stop_mutex)) = state.ws_client_stops.remove(connection_id) {
        let mut lock = stop_mutex.lock().await;
        if let Some(stop_tx) = lock.take() {
            let _ = stop_tx.send(());
        }
    }
    state.ws_client_senders.remove(connection_id);
    let now = chrono::Local::now().timestamp_millis();
    crate::db::ws_db::update_ws_connection_status(&state.db_path, connection_id, "disconnected", Some(now))?;
    Ok(())
}
