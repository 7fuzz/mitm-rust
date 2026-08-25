use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::oneshot;
use crate::db::set_preference;
use crate::proxy::start_proxy_server;
use crate::state::{AppState, ProxyConfig};

#[tauri::command]
pub async fn get_proxy_state(
    state: State<'_, AppState>,
) -> Result<ProxyConfig, String> {
    let cfg = state.proxy_config.read().await;
    Ok(cfg.clone())
}

#[tauri::command]
pub async fn set_proxy_mode(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    mode: String,
) -> Result<ProxyConfig, String> {
    let valid_modes = ["on", "off", "block_client", "block"];
    if !valid_modes.contains(&mode.as_str()) {
        return Err(format!("Invalid proxy mode '{}'. Expected one of: {:?}", mode, valid_modes));
    }

    let is_enabled = mode != "off";

    {
        let mut cfg = state.proxy_config.write().await;
        cfg.proxy_mode = mode.clone();
        cfg.proxy_enabled = is_enabled;
    }

    let _ = set_preference(&state.db_path, "proxy_mode", &mode);
    let _ = set_preference(&state.db_path, "proxy_enabled", if is_enabled { "true" } else { "false" });

    if is_enabled {
        if !state.is_proxy_active() {
            let proxy_port = { state.proxy_config.read().await.port };
            let proxy_host = { state.proxy_config.read().await.host.clone() };
            let addr_str = format!("{}:{}", proxy_host, proxy_port);

            let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
            let ca_dir = app_data_dir.join("ca");
            let ca = Arc::new(crate::ca::get_ca(ca_dir));

            let (stop_tx, stop_rx) = oneshot::channel::<()>();

            {
                let mut stop_guard = state.stop_signal.lock().await;
                *stop_guard = Some(stop_tx);
            }

            state.set_proxy_active(true);

            let state_arc = Arc::new(AppState {
                db_path: state.db_path.clone(),
                history_tx: state.history_tx.clone(),
                proxy_active: std::sync::atomic::AtomicBool::new(true),
                broadcast_tx: state.broadcast_tx.clone(),
                proxy_config: Arc::clone(&state.proxy_config),
                history_settings: Arc::clone(&state.history_settings),
                stop_signal: Arc::clone(&state.stop_signal),

                pending_flows: Arc::clone(&state.pending_flows),
                rules: Arc::clone(&state.rules),
            });

            tauri::async_runtime::spawn(async move {
                let _ = start_proxy_server(app_handle, state_arc, ca, addr_str, stop_rx).await;
            });
        }
    } else {
        if state.is_proxy_active() {
            let mut stop_guard = state.stop_signal.lock().await;
            if let Some(tx) = stop_guard.take() {
                let _ = tx.send(());
            }
            state.set_proxy_active(false);
        }
    }

    let cfg = state.proxy_config.read().await;
    Ok(cfg.clone())
}

#[tauri::command]
pub async fn toggle_proxy(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<ProxyConfig, String> {
    let mode = if enabled { "on" } else { "off" };
    set_proxy_mode(app_handle, state, mode.to_string()).await
}

#[tauri::command]
pub async fn start_proxy(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    port: Option<u16>,
    host: Option<String>,
) -> Result<ProxyConfig, String> {
    if let Some(p) = port {
        state.proxy_config.write().await.port = p;
    }
    if let Some(h) = host {
        state.proxy_config.write().await.host = h;
    }
    set_proxy_mode(app_handle, state, "on".to_string()).await
}

#[tauri::command]
pub async fn stop_proxy(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<ProxyConfig, String> {
    set_proxy_mode(app_handle, state, "off".to_string()).await
}

#[tauri::command]
pub async fn toggle_proxy_legacy(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<ProxyConfig, String> {
    let currently_enabled = state.proxy_config.read().await.proxy_enabled;
    toggle_proxy(app_handle, state, !currently_enabled).await
}

#[tauri::command]
pub async fn get_proxy_status(
    state: State<'_, AppState>,
) -> Result<ProxyConfig, String> {
    get_proxy_state(state).await
}
