use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use crate::state::{AppState, WebSocketConn, WebSocketMessage, WebSocketMessageCapturedEvent};
use crate::ws::{connect_client_session, disconnect_client_session};

#[tauri::command]
pub async fn get_ws_connections(state: State<'_, AppState>) -> Result<Vec<WebSocketConn>, String> {
    crate::db::ws_db::load_ws_connections(&state.db_path)
}

#[tauri::command]
pub async fn get_ws_messages(
    state: State<'_, AppState>,
    connection_id: String,
    limit: Option<u32>,
) -> Result<Vec<WebSocketMessage>, String> {
    crate::db::ws_db::load_ws_messages(&state.db_path, &connection_id, limit)
}

#[tauri::command]
pub async fn connect_ws_client(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    url: String,
    headers: Option<Vec<(String, String)>>,
) -> Result<WebSocketConn, String> {
    connect_client_session(app_handle, Arc::new((*state).clone()), url, headers).await
}

#[tauri::command]
pub async fn disconnect_ws_client(
    state: State<'_, AppState>,
    connection_id: String,
) -> Result<(), String> {
    disconnect_client_session(Arc::new((*state).clone()), &connection_id).await
}

#[tauri::command]
pub async fn send_ws_message(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    connection_id: String,
    direction: String,
    msg_type: String,
    payload: String,
) -> Result<WebSocketMessage, String> {
    let msg = match msg_type.as_str() {
        "binary" => {
            let bytes = hex::decode(&payload).unwrap_or_else(|_| payload.as_bytes().to_vec());
            Message::Binary(bytes.into())
        }
        _ => Message::Text(payload.clone().into()),
    };

    let len = msg.len();

    // 1. If it's a client session, send through ws_client_senders
    if let Some(sender) = state.ws_client_senders.get(&connection_id) {
        sender.send(msg).map_err(|e| format!("Failed to send client message: {}", e))?;
    }
    // 2. If it's a proxied stream, send to specified direction
    else if let Some(senders) = state.ws_stream_senders.get(&connection_id) {
        let (to_server_tx, to_client_tx) = senders.value();
        if direction == "to_server" {
            to_server_tx.send(msg).map_err(|e| format!("Failed to inject to server: {}", e))?;
        } else {
            to_client_tx.send(msg).map_err(|e| format!("Failed to inject to client: {}", e))?;
        }
    } else {
        return Err(format!("Connection '{}' is not currently active or connected", connection_id));
    }

    let msg_record = WebSocketMessage {
        id: format!("ws-msg-{}", Uuid::new_v4()),
        connection_id: connection_id.clone(),
        direction,
        msg_type,
        payload,
        timestamp: chrono::Local::now().timestamp_millis(),
        length: len,
        is_injected: true,
    };

    crate::db::ws_db::save_ws_message(&state.db_path, &msg_record)?;
    let count = crate::db::ws_db::increment_ws_message_count(&state.db_path, &connection_id).unwrap_or(1);

    let _ = app_handle.emit(
        "websocket_message_event",
        WebSocketMessageCapturedEvent {
            message: msg_record.clone(),
            message_count: count,
        },
    );

    Ok(msg_record)
}

#[tauri::command]
pub async fn clear_ws_messages(
    state: State<'_, AppState>,
    connection_id: Option<String>,
) -> Result<(), String> {
    crate::db::ws_db::clear_ws_messages(&state.db_path, connection_id.as_deref())
}

#[tauri::command]
pub async fn delete_ws_connection(
    state: State<'_, AppState>,
    connection_id: String,
) -> Result<(), String> {
    let _ = disconnect_client_session(Arc::new((*state).clone()), &connection_id).await;
    crate::db::ws_db::delete_ws_connection(&state.db_path, &connection_id)
}
