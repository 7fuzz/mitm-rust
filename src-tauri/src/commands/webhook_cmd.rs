use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::oneshot;

use crate::state::{AppState, WebhookDelivery, WebhookEndpoint, WebhookListenerConfig, WebhookSignatureResult};
use crate::webhook::hmac::calculate_signature;
use crate::webhook::replay::{replay_delivery, WebhookReplayResult};
use crate::webhook::start_webhook_listener_server;

#[tauri::command]
pub async fn get_webhook_endpoints(state: State<'_, AppState>) -> Result<Vec<WebhookEndpoint>, String> {
    crate::db::webhook_db::load_webhook_endpoints(&state.db_path)
}

#[tauri::command]
pub async fn create_webhook_endpoint(
    state: State<'_, AppState>,
    endpoint: WebhookEndpoint,
) -> Result<WebhookEndpoint, String> {
    let mut ep = endpoint;
    if ep.id.is_empty() {
        ep.id = format!("wh-ep-{}", uuid::Uuid::new_v4());
    }
    if ep.created_at == 0 {
        ep.created_at = chrono::Local::now().timestamp_millis();
    }
    crate::db::webhook_db::save_webhook_endpoint(&state.db_path, &ep)?;
    Ok(ep)
}

#[tauri::command]
pub async fn delete_webhook_endpoint(state: State<'_, AppState>, id: String) -> Result<(), String> {
    crate::db::webhook_db::delete_webhook_endpoint(&state.db_path, &id)
}

#[tauri::command]
pub async fn get_webhook_deliveries(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<WebhookDelivery>, String> {
    crate::db::webhook_db::load_webhook_deliveries(&state.db_path, limit)
}

#[tauri::command]
pub async fn clear_webhook_deliveries(state: State<'_, AppState>) -> Result<(), String> {
    crate::db::webhook_db::clear_webhook_deliveries(&state.db_path)
}

#[tauri::command]
pub async fn get_webhook_listener_status(state: State<'_, AppState>) -> Result<WebhookListenerConfig, String> {
    let port = *state.webhook_port.read().await;
    let is_running = state.is_webhook_running();
    Ok(WebhookListenerConfig { port, is_running })
}

#[tauri::command]
pub async fn start_webhook_listener(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    port: u16,
) -> Result<(), String> {
    if state.is_webhook_running() {
        stop_webhook_listener(state.clone()).await?;
    }

    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    {
        let mut sig = state.webhook_stop_signal.lock().await;
        *sig = Some(stop_tx);
    }

    *state.webhook_port.write().await = port;
    let _ = crate::db::set_preference(&state.db_path, "webhook_port", &port.to_string());

    let state_arc = Arc::new((*state).clone());

    tauri::async_runtime::spawn(async move {
        if let Err(e) = start_webhook_listener_server(app_handle, state_arc, port, stop_rx).await {
            eprintln!("[Webhook Server] Error: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_webhook_listener(state: State<'_, AppState>) -> Result<(), String> {
    let mut sig = state.webhook_stop_signal.lock().await;
    if let Some(tx) = sig.take() {
        let _ = tx.send(());
    }
    state.set_webhook_running(false);
    Ok(())
}

#[tauri::command]
pub async fn calculate_webhook_signature(
    secret: String,
    body: String,
    provider: String,
) -> Result<WebhookSignatureResult, String> {
    let (header_name, header_value) = calculate_signature(&secret, &body, &provider);
    Ok(WebhookSignatureResult {
        header_name,
        header_value,
    })
}

#[tauri::command]
pub async fn replay_webhook_delivery(
    state: State<'_, AppState>,
    id: String,
    target_url: String,
) -> Result<WebhookReplayResult, String> {
    let deliveries = crate::db::webhook_db::load_webhook_deliveries(&state.db_path, Some(500))?;
    let delivery = deliveries
        .into_iter()
        .find(|d| d.id == id)
        .ok_or_else(|| format!("Webhook delivery with id {} not found", id))?;

    replay_delivery(delivery.headers, delivery.payload, &target_url).await
}
