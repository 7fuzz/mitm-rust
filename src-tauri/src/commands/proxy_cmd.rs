use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::oneshot;
use crate::ca::RootCa;
use crate::proxy::start_proxy_server;
use crate::state::{AppState, ProxyConfig};

#[tauri::command]
pub async fn start_proxy(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    port: Option<u16>,
    host: Option<String>,
) -> Result<ProxyConfig, String> {
    if state.is_proxy_active() {
        let cfg = state.proxy_config.lock().await;
        return Ok(cfg.clone());
    }

    let proxy_port = port.unwrap_or(8080);
    let proxy_host = host.unwrap_or_else(|| "127.0.0.1".to_string());
    let addr_str = format!("{}:{}", proxy_host, proxy_port);

    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let ca_dir = app_data_dir.join("ca");
    let ca = Arc::new(RootCa::load_or_generate(ca_dir)?);

    let (stop_tx, stop_rx) = oneshot::channel::<()>();

    {
        let mut stop_guard = state.stop_signal.lock().await;
        *stop_guard = Some(stop_tx);
    }

    let state_arc = Arc::new(AppState {
        db_path: state.db_path.clone(),
        history_tx: state.history_tx.clone(),
        proxy_active: std::sync::atomic::AtomicBool::new(true),
        broadcast_tx: state.broadcast_tx.clone(),
        proxy_config: Arc::clone(&state.proxy_config),
        stop_signal: Arc::clone(&state.stop_signal),
    });

    state.set_proxy_active(true);

    tauri::async_runtime::spawn(async move {
        let _ = start_proxy_server(app_handle, state_arc, ca, addr_str, stop_rx).await;
    });

    let mut cfg = state.proxy_config.lock().await;
    cfg.port = proxy_port;
    cfg.host = proxy_host;
    cfg.is_running = true;

    Ok(cfg.clone())
}

#[tauri::command]
pub async fn stop_proxy(
    state: State<'_, AppState>,
) -> Result<ProxyConfig, String> {
    if !state.is_proxy_active() {
        let cfg = state.proxy_config.lock().await;
        return Ok(cfg.clone());
    }

    let mut stop_guard = state.stop_signal.lock().await;
    if let Some(tx) = stop_guard.take() {
        let _ = tx.send(());
    }

    state.set_proxy_active(false);

    let mut cfg = state.proxy_config.lock().await;
    cfg.is_running = false;

    Ok(cfg.clone())
}

#[tauri::command]
pub async fn toggle_proxy(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<ProxyConfig, String> {
    if state.is_proxy_active() {
        stop_proxy(state).await
    } else {
        start_proxy(app_handle, state, None, None).await
    }
}

#[tauri::command]
pub async fn get_proxy_status(
    state: State<'_, AppState>,
) -> Result<ProxyConfig, String> {
    let cfg = state.proxy_config.lock().await;
    Ok(cfg.clone())
}
