use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use hyper_util::rt::TokioIo;
use tokio::sync::Mutex;
use futures::{StreamExt, SinkExt};

use crate::models::{AppState, ActiveWsConnection, WsMessage, WsResumeAction};

pub fn is_websocket_upgrade(headers: &hyper::HeaderMap) -> bool {
    let connection_upgrade = headers.get(hyper::header::CONNECTION)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_lowercase().contains("upgrade"))
        .unwrap_or(false);
    let upgrade_websocket = headers.get(hyper::header::UPGRADE)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_lowercase() == "websocket")
        .unwrap_or(false);
    connection_upgrade && upgrade_websocket
}

pub async fn run_ws_proxy(
    app_handle: AppHandle,
    intercept_state: Arc<Mutex<crate::proxy::InterceptState>>,
    connection_id: String,
    client_upgraded: hyper::upgrade::Upgraded,
    server_upgraded: hyper::upgrade::Upgraded,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client_ws = tokio_tungstenite::WebSocketStream::from_raw_socket(
        TokioIo::new(client_upgraded),
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None
    ).await;
    
    let server_ws = tokio_tungstenite::WebSocketStream::from_raw_socket(
        TokioIo::new(server_upgraded),
        tokio_tungstenite::tungstenite::protocol::Role::Client,
        None
    ).await;

    let (client_tx, mut client_rx) = client_ws.split();
    let (server_tx, mut server_rx) = server_ws.split();
    
    let client_tx = Arc::new(Mutex::new(client_tx));
    let server_tx = Arc::new(Mutex::new(server_tx));

    let (to_client_tx, mut to_client_rx) = tokio::sync::mpsc::unbounded_channel::<tokio_tungstenite::tungstenite::Message>();
    let (to_server_tx, mut to_server_rx) = tokio::sync::mpsc::unbounded_channel::<tokio_tungstenite::tungstenite::Message>();

    let active_ws = ActiveWsConnection {
        to_client_tx,
        to_server_tx,
    };
    {
        let app_state = app_handle.state::<AppState>();
        let mut active = app_state.active_websockets.lock().await;
        active.insert(connection_id.clone(), active_ws);
    }

    let app_handle_c1 = app_handle.clone();
    let intercept_c1 = Arc::clone(&intercept_state);
    let conn_id_c1 = connection_id.clone();
    let server_tx_c1 = Arc::clone(&server_tx);
    
    // Task 1: Client to Server
    let t1 = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg_opt = client_rx.next() => {
                    let msg = match msg_opt {
                        Some(Ok(m)) => m,
                        _ => break,
                    };
                    
                    if msg.is_ping() || msg.is_pong() {
                        let mut tx = server_tx_c1.lock().await;
                        let _ = tx.send(msg).await;
                        continue;
                    }
                    if msg.is_close() {
                        let mut tx = server_tx_c1.lock().await;
                        let _ = tx.send(msg).await;
                        break;
                    }
                    
                    let payload = msg.to_text().unwrap_or_default().to_string();
                    let msg_type = if msg.is_text() { "text" } else { "binary" };
                    
                    let mut tx = server_tx_c1.lock().await;
                    if let Err(e) = handle_ws_message(
                        &app_handle_c1,
                        &intercept_c1,
                        &conn_id_c1,
                        "client_to_server",
                        msg_type,
                        payload,
                        &mut *tx
                    ).await {
                        eprintln!("Error in WS client to server: {}", e);
                        break;
                    }
                }
                injected_opt = to_server_rx.recv() => {
                    if let Some(msg) = injected_opt {
                        let mut tx = server_tx_c1.lock().await;
                        let _ = tx.send(msg).await;
                    } else {
                        break;
                    }
                }
            }
        }
    });

    let app_handle_c2 = app_handle.clone();
    let intercept_c2 = Arc::clone(&intercept_state);
    let conn_id_c2 = connection_id.clone();
    let client_tx_c2 = Arc::clone(&client_tx);
    
    // Task 2: Server to Client
    let t2 = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg_opt = server_rx.next() => {
                    let msg = match msg_opt {
                        Some(Ok(m)) => m,
                        _ => break,
                    };
                    
                    if msg.is_ping() || msg.is_pong() {
                        let mut tx = client_tx_c2.lock().await;
                        let _ = tx.send(msg).await;
                        continue;
                    }
                    if msg.is_close() {
                        let mut tx = client_tx_c2.lock().await;
                        let _ = tx.send(msg).await;
                        break;
                    }
                    
                    let payload = msg.to_text().unwrap_or_default().to_string();
                    let msg_type = if msg.is_text() { "text" } else { "binary" };
                    
                    let mut tx = client_tx_c2.lock().await;
                    if let Err(e) = handle_ws_message(
                        &app_handle_c2,
                        &intercept_c2,
                        &conn_id_c2,
                        "server_to_client",
                        msg_type,
                        payload,
                        &mut *tx
                    ).await {
                        eprintln!("Error in WS server to client: {}", e);
                        break;
                    }
                }
                injected_opt = to_client_rx.recv() => {
                    if let Some(msg) = injected_opt {
                        let mut tx = client_tx_c2.lock().await;
                        let _ = tx.send(msg).await;
                    } else {
                        break;
                    }
                }
            }
        }
    });

    let _ = tokio::select! {
        _ = t1 => {},
        _ = t2 => {},
    };

    {
        let app_state = app_handle.state::<AppState>();
        let mut active = app_state.active_websockets.lock().await;
        active.remove(&connection_id);
        let _ = app_handle.emit("ws_connection_closed", &connection_id);
    }

    Ok(())
}

async fn handle_ws_message<S>(
    app_handle: &AppHandle,
    intercept_state: &Arc<Mutex<crate::proxy::InterceptState>>,
    connection_id: &str,
    direction: &str,
    msg_type: &str,
    payload: String,
    sink: &mut S
) -> Result<(), String> 
where
    S: futures::sink::Sink<tokio_tungstenite::tungstenite::Message> + Unpin,
    <S as futures::sink::Sink<tokio_tungstenite::tungstenite::Message>>::Error: std::fmt::Display
{
    let message_id = uuid::Uuid::new_v4().to_string();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;

    let db_path = crate::db::get_db_path(app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    conn.execute(
        "INSERT INTO websocket_messages (id, connection_id, direction, msg_type, payload, timestamp, is_intercepted) VALUES (?, ?, ?, ?, ?, ?, 0)",
        rusqlite::params![
            message_id,
            connection_id,
            direction,
            msg_type,
            payload,
            now as i64
        ]
    ).map_err(|e| e.to_string())?;

    let ws_message = WsMessage {
        id: message_id.clone(),
        connection_id: connection_id.to_string(),
        direction: direction.to_string(),
        msg_type: msg_type.to_string(),
        payload: payload.clone(),
        timestamp: now,
        is_intercepted: false,
    };
    let _ = app_handle.emit("ws_message_captured", &ws_message);

    let intercept_config = {
        let intercept = intercept_state.lock().await;
        intercept.config.clone()
    };

    if intercept_config.enabled && 
       (intercept_config.mode == "both" || 
        (intercept_config.mode == "request" && direction == "client_to_server") ||
        (intercept_config.mode == "response" && direction == "server_to_client"))
    {
        let _ = conn.execute(
            "UPDATE websocket_messages SET is_intercepted = 1 WHERE id = ?",
            [message_id.clone()]
        );

        let mut intercept_msg = ws_message.clone();
        intercept_msg.is_intercepted = true;
        let _ = app_handle.emit("ws_message_captured", &intercept_msg);

        let (tx, rx) = tokio::sync::oneshot::channel::<WsResumeAction>();
        {
            let app_state = app_handle.state::<AppState>();
            let mut pending = app_state.pending_ws.lock().await;
            pending.insert(message_id.clone(), tx);
        }

        let mut final_payload = payload;
        if let Ok(action) = rx.await {
            if action.drop.unwrap_or(false) {
                let _ = conn.execute(
                    "UPDATE websocket_messages SET is_intercepted = 0, payload = '[DROPPED]' WHERE id = ?",
                    [message_id.clone()]
                );
                return Ok(());
            }
            if let Some(new_payload) = action.payload {
                final_payload = new_payload;
                let _ = conn.execute(
                    "UPDATE websocket_messages SET is_intercepted = 0, payload = ? WHERE id = ?",
                    rusqlite::params![final_payload, message_id.clone()]
                );
            } else {
                let _ = conn.execute(
                    "UPDATE websocket_messages SET is_intercepted = 0 WHERE id = ?",
                    [message_id.clone()]
                );
            }
        } else {
            let _ = conn.execute(
                "UPDATE websocket_messages SET is_intercepted = 0 WHERE id = ?",
                [message_id.clone()]
            );
        }

        let mut updated_msg = ws_message;
        updated_msg.payload = final_payload.clone();
        updated_msg.is_intercepted = false;
        let _ = app_handle.emit("ws_message_captured", &updated_msg);

        let msg = tokio_tungstenite::tungstenite::Message::Text(final_payload);
        sink.send(msg).await.map_err(|e| e.to_string())?;
    } else {
        let msg = tokio_tungstenite::tungstenite::Message::Text(payload);
        sink.send(msg).await.map_err(|e| e.to_string())?;
    }

    Ok(())
}
