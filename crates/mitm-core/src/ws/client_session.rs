use std::sync::Arc;
use futures_util::{SinkExt, StreamExt};
use crate::events::EventBus;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

use crate::state::{AppState, WebSocketConn, WebSocketMessage};

pub async fn connect_client_session(
    event_bus: Option<EventBus>,
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

    let now = chrono::Local::now().timestamp_millis();

    let mut conn_info = WebSocketConn {
        connection_id: 0,
        url: url_str.clone(),
        status: "connected".to_string(),
        handshake_time: now,
        closed_at: None,
        protocol: subprotocol,
        client_addr: Some("127.0.0.1 (Studio Client)".to_string()),
        is_client_session: true,
        message_count: 0,
        listener_label: String::new(),
    };

    conn_info.connection_id = crate::db::ws_db::insert_ws_connection(&state.db_path, &conn_info)?;
    let conn_id = conn_info.connection_id;

    let (mut write_half, mut read_half) = ws_stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let (stop_tx, mut stop_rx) = oneshot::channel::<()>();

    state.ws_client_senders.insert(conn_id, tx);
    state.ws_client_stops.insert(conn_id, Arc::new(Mutex::new(Some(stop_tx))));

    let db_path_clone = state.db_path.clone();
    let event_bus_clone = event_bus.clone();
    let state_clone = Arc::clone(&state);

    // Writer Task (outgoing to remote server)
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = write_half.send(msg).await {
                eprintln!("[WS Client {}] Send error: {}", conn_id, e);
                break;
            }
        }
    });

    let conn_info_task = conn_info.clone();

    // Reader Task (incoming from remote server)
    tokio::spawn(async move {
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

                            let mut msg_record = WebSocketMessage {
                                id: 0,
                                connection_id: conn_id,
                                direction: "to_client".to_string(),
                                msg_type: msg_type.to_string(),
                                payload: payload_str,
                                timestamp: chrono::Local::now().timestamp_millis(),
                                length: msg.len(),
                                is_injected: false,
                            };

                            let _ = crate::db::ws_db::save_ws_message(&db_path_clone, &mut msg_record);
                            let _count = crate::db::ws_db::increment_ws_message_count(&db_path_clone, conn_id).unwrap_or(1);

                            if let Some(ref bus) = event_bus_clone {
                                if let Ok(val) = serde_json::to_value(&msg_record) {
                                    bus.emit(crate::events::CoreEvent::WsFrameReceived {
                                        session_id: conn_id.to_string(),
                                        frame: val,
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[WS Client {}] Read error: {}", conn_id, e);
                            break;
                        }
                    }
                }
            } => {}
            _ = &mut stop_rx => {}
        }

        // Connection cleanup
        let close_time = chrono::Local::now().timestamp_millis();
        let _ = crate::db::ws_db::update_ws_connection_status(&db_path_clone, conn_id, "disconnected", Some(close_time));

        state_clone.ws_client_senders.remove(&conn_id);
        state_clone.ws_client_stops.remove(&conn_id);

        let mut closed_conn = conn_info_task;
        closed_conn.status = "disconnected".to_string();
        closed_conn.closed_at = Some(close_time);
    });


    Ok(conn_info)
}

/// Sends a frame on a live session. Client sessions always send upstream; a
/// proxied session injects toward the server or the client per `to_server`. The
/// frame is saved and emitted as injected, since it never returns through a read
/// loop.
pub async fn send_frame(
    event_bus: Option<EventBus>,
    state: Arc<AppState>,
    connection_id: i64,
    msg_type: &str,
    payload: String,
    to_server: bool,
) -> Result<(), String> {
    let (message, stored_payload) = match msg_type {
        "binary" => {
            let bytes = hex::decode(payload.split_whitespace().collect::<String>())
                .map_err(|e| format!("Invalid hex payload: {e}"))?;
            let stored = format!("[Binary Frame: {} bytes] {}", bytes.len(), hex::encode(&bytes));
            (Message::Binary(bytes.into()), stored)
        }
        _ => (Message::Text(payload.clone().into()), payload),
    };

    let direction = if let Some(tx) = state.ws_client_senders.get(&connection_id) {
        tx.send(message).map_err(|_| "WebSocket client is closed".to_string())?;
        "to_server"
    } else if let Some(entry) = state.ws_stream_senders.get(&connection_id) {
        let (to_server_tx, to_client_tx) = entry.value();
        let tx = if to_server { to_server_tx } else { to_client_tx };
        tx.send(message).map_err(|_| "WebSocket session is closed".to_string())?;
        if to_server { "to_server" } else { "to_client" }
    } else {
        return Err("WebSocket session is not connected".to_string());
    };

    let mut record = WebSocketMessage {
        id: 0,
        connection_id,
        direction: direction.to_string(),
        msg_type: if msg_type == "json" { "json" } else if msg_type == "binary" { "binary" } else { "text" }.to_string(),
        payload: stored_payload,
        timestamp: chrono::Local::now().timestamp_millis(),
        length: 0,
        is_injected: true,
    };
    record.length = record.payload.len();
    crate::db::ws_db::save_ws_message(&state.db_path, &mut record)?;
    let _ = crate::db::ws_db::increment_ws_message_count(&state.db_path, connection_id);
    if let Some(bus) = event_bus {
        if let Ok(val) = serde_json::to_value(&record) {
            bus.emit(crate::events::CoreEvent::WsFrameReceived {
                session_id: connection_id.to_string(),
                frame: val,
            });
        }
    }
    Ok(())
}

pub async fn disconnect_client_session(state: Arc<AppState>, connection_id: i64) -> Result<(), String> {
    if let Some((_, stop_mutex)) = state.ws_client_stops.remove(&connection_id) {
        let mut lock = stop_mutex.lock().await;
        if let Some(stop_tx) = lock.take() {
            let _ = stop_tx.send(());
        }
    }
    state.ws_client_senders.remove(&connection_id);
    let now = chrono::Local::now().timestamp_millis();
    crate::db::ws_db::update_ws_connection_status(&state.db_path, connection_id, "disconnected", Some(now))?;
    Ok(())
}
