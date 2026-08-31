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

            let state_arc = Arc::new((*state).clone());

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

#[tauri::command]
pub async fn update_network_settings(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    bindings: Vec<String>,
) -> Result<ProxyConfig, String> {
    if bindings.is_empty() {
        return Err("No listener bindings specified".to_string());
    }

    let raw_binding = bindings[0].trim();
    if raw_binding.is_empty() {
        return Err("Binding address cannot be empty".to_string());
    }

    let (host, port) = if let Some(colon_pos) = raw_binding.rfind(':') {
        let h = raw_binding[..colon_pos].trim();
        let p_str = raw_binding[colon_pos + 1..].trim();
        let p = p_str.parse::<u16>().map_err(|e| format!("Invalid port '{}': {}", p_str, e))?;
        let host_final = if h.is_empty() { "0.0.0.0".to_string() } else { h.to_string() };
        (host_final, p)
    } else {
        let p = raw_binding.parse::<u16>().map_err(|e| format!("Invalid port '{}': {}", raw_binding, e))?;
        ("0.0.0.0".to_string(), p)
    };

    // Update memory state
    {
        let mut cfg = state.proxy_config.write().await;
        cfg.host = host.clone();
        cfg.port = port;
    }

    // Persist to preferences
    let _ = set_preference(&state.db_path, "proxy_host", &host);
    let _ = set_preference(&state.db_path, "proxy_port", &port.to_string());

    // If proxy server is currently active, restart it on new binding address
    if state.is_proxy_active() {
        // Stop current listener
        {
            let mut stop_guard = state.stop_signal.lock().await;
            if let Some(tx) = stop_guard.take() {
                let _ = tx.send(());
            }
        }

        // Wait briefly for port to release
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let addr_str = format!("{}:{}", host, port);
        let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
        let ca_dir = app_data_dir.join("ca");
        let ca = Arc::new(crate::ca::get_ca(ca_dir));

        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        {
            let mut stop_guard = state.stop_signal.lock().await;
            *stop_guard = Some(stop_tx);
        }

        let state_arc = Arc::new((*state).clone());

        let app_handle_clone = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let _ = start_proxy_server(app_handle_clone, state_arc, ca, addr_str, stop_rx).await;
        });
    }

    let cfg = state.proxy_config.read().await;
    Ok(cfg.clone())
}

#[tauri::command]
pub async fn set_ws_mitm_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<ProxyConfig, String> {
    {
        let mut cfg = state.proxy_config.write().await;
        cfg.ws_mitm_enabled = enabled;
    }
    let _ = set_preference(&state.db_path, "ws_mitm_enabled", if enabled { "true" } else { "false" });
    let cfg = state.proxy_config.read().await;
    Ok(cfg.clone())
}
