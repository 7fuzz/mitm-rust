use std::sync::Arc;
use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;

use crate::state::{AppState, WebSocketConn, WebSocketConnectionEvent, WebSocketMessage, WebSocketMessageCapturedEvent};

pub async fn bridge_proxied_websocket<C, S>(
    client_io: C,
    server_io: S,
    url: String,
    client_addr: Option<String>,
    subprotocol: Option<String>,
    app_handle: AppHandle,
    state: Arc<AppState>,
) where
    C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let conn_id = format!("ws-prox-{}", Uuid::new_v4());
    let now = chrono::Local::now().timestamp_millis();

    let conn_info = WebSocketConn {
        connection_id: conn_id.clone(),
        url: url.clone(),
        status: "connected".to_string(),
        handshake_time: now,
        closed_at: None,
        protocol: subprotocol,
        client_addr,
        is_client_session: false,
        message_count: 0,
    };

    let _ = crate::db::ws_db::save_ws_connection(&state.db_path, &conn_info);
    let _ = app_handle.emit(
        "websocket_connection_event",
        WebSocketConnectionEvent {
            connection: conn_info.clone(),
            event_type: "opened".to_string(),
        },
    );

    let client_ws = WebSocketStream::from_raw_socket(client_io, Role::Server, None).await;
    let server_ws = WebSocketStream::from_raw_socket(server_io, Role::Client, None).await;

    let (mut client_write, mut client_read) = client_ws.split();
    let (mut server_write, mut server_read) = server_ws.split();

    // Injection channels: (to_server_tx, to_client_tx)
    let (to_server_tx, mut to_server_rx) = mpsc::unbounded_channel::<Message>();
    let (to_client_tx, mut to_client_rx) = mpsc::unbounded_channel::<Message>();

    state.ws_stream_senders.insert(conn_id.clone(), (to_server_tx, to_client_tx));

    let conn_id_c2s = conn_id.clone();
    let db_path_c2s = state.db_path.clone();
    let app_c2s = app_handle.clone();

    // Task 1: Client -> Server (with injected frames to server)
    let c2s_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Injected message from user interface to server
                Some(injected_msg) = to_server_rx.recv() => {
                    let len = injected_msg.len();
                    let (msg_type, payload_str) = extract_message_info(&injected_msg);
                    if let Err(e) = server_write.send(injected_msg).await {
                        eprintln!("[WS Proxy {}] Failed to send injected frame to server: {}", conn_id_c2s, e);
                        break;
                    }

                    let msg_record = WebSocketMessage {
                        id: format!("ws-msg-{}", Uuid::new_v4()),
                        connection_id: conn_id_c2s.clone(),
                        direction: "to_server".to_string(),
                        msg_type: msg_type.to_string(),
                        payload: payload_str,
                        timestamp: chrono::Local::now().timestamp_millis(),
                        length: len,
                        is_injected: true,
                    };
                    let _ = crate::db::ws_db::save_ws_message(&db_path_c2s, &msg_record);
                    let count = crate::db::ws_db::increment_ws_message_count(&db_path_c2s, &conn_id_c2s).unwrap_or(1);
                    let _ = app_c2s.emit("websocket_message_event", WebSocketMessageCapturedEvent { message: msg_record, message_count: count });
                }
                // Regular frame from downstream client to server
                msg_opt = client_read.next() => {
                    match msg_opt {
                        Some(Ok(msg)) => {
                            if msg.is_ping() || msg.is_pong() {
                                let _ = server_write.send(msg).await;
                                continue;
                            }
                            if msg.is_close() {
                                let _ = server_write.send(msg).await;
                                break;
                            }
                            let len = msg.len();
                            let (msg_type, payload_str) = extract_message_info(&msg);

                            let msg_record = WebSocketMessage {
                                id: format!("ws-msg-{}", Uuid::new_v4()),
                                connection_id: conn_id_c2s.clone(),
                                direction: "to_server".to_string(),
                                msg_type: msg_type.to_string(),
                                payload: payload_str,
                                timestamp: chrono::Local::now().timestamp_millis(),
                                length: len,
                                is_injected: false,
                            };
                            let _ = crate::db::ws_db::save_ws_message(&db_path_c2s, &msg_record);
                            let count = crate::db::ws_db::increment_ws_message_count(&db_path_c2s, &conn_id_c2s).unwrap_or(1);
                            let _ = app_c2s.emit("websocket_message_event", WebSocketMessageCapturedEvent { message: msg_record, message_count: count });

                            if let Err(e) = server_write.send(msg).await {
                                eprintln!("[WS Proxy {}] Forward to server failed: {}", conn_id_c2s, e);
                                break;
                            }
                        }
                        _ => break,
                    }
                }
            }
        }
    });

    let conn_id_s2c = conn_id.clone();
    let db_path_s2c = state.db_path.clone();
    let app_s2c = app_handle.clone();

    // Task 2: Server -> Client (with injected frames to client)
    let s2c_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Injected message from user interface to client
                Some(injected_msg) = to_client_rx.recv() => {
                    let len = injected_msg.len();
                    let (msg_type, payload_str) = extract_message_info(&injected_msg);
                    if let Err(e) = client_write.send(injected_msg).await {
                        eprintln!("[WS Proxy {}] Failed to send injected frame to client: {}", conn_id_s2c, e);
                        break;
                    }

                    let msg_record = WebSocketMessage {
                        id: format!("ws-msg-{}", Uuid::new_v4()),
                        connection_id: conn_id_s2c.clone(),
                        direction: "to_client".to_string(),
                        msg_type: msg_type.to_string(),
                        payload: payload_str,
                        timestamp: chrono::Local::now().timestamp_millis(),
                        length: len,
                        is_injected: true,
                    };
                    let _ = crate::db::ws_db::save_ws_message(&db_path_s2c, &msg_record);
                    let count = crate::db::ws_db::increment_ws_message_count(&db_path_s2c, &conn_id_s2c).unwrap_or(1);
                    let _ = app_s2c.emit("websocket_message_event", WebSocketMessageCapturedEvent { message: msg_record, message_count: count });
                }
                // Regular frame from upstream server to client
                msg_opt = server_read.next() => {
                    match msg_opt {
                        Some(Ok(msg)) => {
                            if msg.is_ping() || msg.is_pong() {
                                let _ = client_write.send(msg).await;
                                continue;
                            }
                            if msg.is_close() {
                                let _ = client_write.send(msg).await;
                                break;
                            }
                            let len = msg.len();
                            let (msg_type, payload_str) = extract_message_info(&msg);

                            let msg_record = WebSocketMessage {
                                id: format!("ws-msg-{}", Uuid::new_v4()),
                                connection_id: conn_id_s2c.clone(),
                                direction: "to_client".to_string(),
                                msg_type: msg_type.to_string(),
                                payload: payload_str,
                                timestamp: chrono::Local::now().timestamp_millis(),
                                length: len,
                                is_injected: false,
                            };
                            let _ = crate::db::ws_db::save_ws_message(&db_path_s2c, &msg_record);
                            let count = crate::db::ws_db::increment_ws_message_count(&db_path_s2c, &conn_id_s2c).unwrap_or(1);
                            let _ = app_s2c.emit("websocket_message_event", WebSocketMessageCapturedEvent { message: msg_record, message_count: count });

                            if let Err(e) = client_write.send(msg).await {
                                eprintln!("[WS Proxy {}] Forward to client failed: {}", conn_id_s2c, e);
                                break;
                            }
                        }
                        _ => break,
                    }
                }
            }
        }
    });

    let _ = tokio::join!(c2s_task, s2c_task);

    // Stream finished / disconnected
    let close_time = chrono::Local::now().timestamp_millis();
    let _ = crate::db::ws_db::update_ws_connection_status(&state.db_path, &conn_id, "disconnected", Some(close_time));
    state.ws_stream_senders.remove(&conn_id);

    let mut closed_conn = conn_info;
    closed_conn.status = "disconnected".to_string();
    closed_conn.closed_at = Some(close_time);

    let _ = app_handle.emit(
        "websocket_connection_event",
        WebSocketConnectionEvent {
            connection: closed_conn,
            event_type: "closed".to_string(),
        },
    );
}

fn extract_message_info(msg: &Message) -> (&'static str, String) {
    match msg {
        Message::Text(text) => {
            let is_json = (text.starts_with('{') && text.ends_with('}')) || (text.starts_with('[') && text.ends_with(']'));
            (if is_json { "json" } else { "text" }, text.clone())
        }
        Message::Binary(bin) => {
            ("binary", format!("[Binary Frame: {} bytes] {}", bin.len(), hex::encode(bin)))
        }
        Message::Ping(p) => ("text", format!("[Ping Frame] {}", String::from_utf8_lossy(p))),
        Message::Pong(p) => ("text", format!("[Pong Frame] {}", String::from_utf8_lossy(p))),
        Message::Close(_) => ("text", "[Close Frame]".to_string()),
        Message::Frame(_) => ("text", "[Raw Frame]".to_string()),
    }
}
