use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::oneshot;
use crate::db::set_preference;
use crate::proxy::{bind_proxy_listener, start_proxy_server};
use crate::state::{normalize_listener_address, AppState, ListenerConfig, ListenerInstance, ListenerStatus, ProxyConfig};

/// A wildcard (0.0.0.0 / ::) listener owns its port exclusively.
/// Returns the ids of specific-IP listeners that a new wildcard would displace;
/// every other clash (exact duplicate, specific IP on a wildcard port) is a hard error.
fn check_address_conflict(
    candidate: SocketAddr,
    configs: &[ListenerConfig],
    ignore_id: Option<u32>,
) -> Result<Vec<u32>, String> {
    let mut displaced = Vec::new();
    for c in configs.iter().filter(|c| Some(c.id) != ignore_id) {
        let Ok(other) = c.address.parse::<SocketAddr>() else { continue };
        if candidate.port() != other.port() {
            continue;
        }
        if candidate.ip() == other.ip() {
            return Err(format!("{} is already used by listener '{}'", candidate, c.label));
        }
        if other.ip().is_unspecified() {
            return Err(format!(
                "Port {} is already bound to all interfaces by listener '{}' ({})",
                candidate.port(), c.label, other
            ));
        }
        if candidate.ip().is_unspecified() {
            displaced.push(c.id);
        }
    }
    Ok(displaced)
}

/// Removes listeners displaced by a new wildcard binding, or errors if not confirmed.
fn resolve_displaced(
    configs: &mut Vec<ListenerConfig>,
    displaced: &[u32],
    replace_conflicts: bool,
) -> Result<(), String> {
    if displaced.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = configs
        .iter()
        .filter(|c| displaced.contains(&c.id))
        .map(|c| format!("'{}' ({})", c.label, c.address))
        .collect();
    if displaced.contains(&0) {
        return Err(format!(
            "Binding all interfaces would replace listener #0 {}. Change listener #0's address first.",
            names.join(", ")
        ));
    }
    if !replace_conflicts {
        return Err(format!(
            "Binding all interfaces on this port would replace {}",
            names.join(", ")
        ));
    }
    configs.retain(|c| !displaced.contains(&c.id));
    Ok(())
}

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
            state.set_proxy_active(true);
            start_all_listeners(&app_handle, &state).await;
        }
    } else if state.is_proxy_active() {
        stop_all_listeners(&state).await;
        state.set_proxy_active(false);
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

/// Legacy entry point: updates the address of listener #0.
#[tauri::command]
pub async fn update_network_settings(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    bindings: Vec<String>,
) -> Result<ProxyConfig, String> {
    let raw = bindings.first().ok_or_else(|| "No listener bindings specified".to_string())?;
    update_listener(app_handle, state.clone(), 0, None, Some(raw.clone()), None).await?;
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

// --- Multi-Listener Management Commands ---

#[tauri::command]
pub async fn get_listener_configs(
    state: State<'_, AppState>,
) -> Result<Vec<ListenerStatus>, String> {
    Ok(listener_statuses(&state).await)
}

#[tauri::command]
pub async fn add_listener(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    label: String,
    address: String,
    replace_conflicts: Option<bool>,
) -> Result<Vec<ListenerStatus>, String> {
    let label = label.trim().to_string();
    if label.is_empty() {
        return Err("Listener label cannot be empty".to_string());
    }
    let address = normalize_listener_address(&address)?;
    let candidate: SocketAddr = address.parse().map_err(|e| format!("{}", e))?;

    {
        let mut configs = state.listener_configs.write().await;
        if configs.len() >= 9 {
            return Err("Maximum of 9 listeners allowed".to_string());
        }
        if configs.iter().any(|c| c.label.eq_ignore_ascii_case(&label)) {
            return Err(format!("Listener with label '{}' already exists", label));
        }
        let displaced = check_address_conflict(candidate, &configs, None)?;
        resolve_displaced(&mut configs, &displaced, replace_conflicts.unwrap_or(false))?;

        let next_id = configs.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        configs.push(ListenerConfig { id: next_id, label, address, enabled: true });
    }

    persist_listener_configs(&state).await;

    reload_listeners(&app_handle, &state).await;

    Ok(listener_statuses(&state).await)
}

#[tauri::command]
pub async fn remove_listener(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    listener_id: u32,
) -> Result<Vec<ListenerStatus>, String> {
    if listener_id == 0 {
        return Err("Listener #0 cannot be removed (you can disable it instead)".to_string());
    }

    state.listener_configs.write().await.retain(|c| c.id != listener_id);
    persist_listener_configs(&state).await;
    reload_listeners(&app_handle, &state).await;

    Ok(listener_statuses(&state).await)
}

#[tauri::command]
pub async fn update_listener(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    listener_id: u32,
    label: Option<String>,
    address: Option<String>,
    replace_conflicts: Option<bool>,
) -> Result<Vec<ListenerStatus>, String> {
    let clean_label = label.map(|l| l.trim().to_string()).filter(|l| !l.is_empty());
    let clean_address = match address.as_deref().map(str::trim).filter(|a| !a.is_empty()) {
        Some(a) => Some(normalize_listener_address(a)?),
        None => None,
    };

    let (needs_restart, enabled) = {
        let mut configs = state.listener_configs.write().await;
        if let Some(ref lbl) = clean_label {
            if configs.iter().any(|c| c.id != listener_id && c.label.eq_ignore_ascii_case(lbl)) {
                return Err(format!("Another listener with label '{}' already exists", lbl));
            }
        }
        let current = configs
            .iter()
            .find(|c| c.id == listener_id)
            .cloned()
            .ok_or_else(|| format!("Listener with ID {} not found", listener_id))?;

        let address_changed = clean_address.as_ref().is_some_and(|a| *a != current.address);
        if address_changed {
            let candidate: SocketAddr = clean_address.as_ref().unwrap().parse().map_err(|e| format!("{}", e))?;
            let displaced = check_address_conflict(candidate, &configs, Some(listener_id))?;
            resolve_displaced(&mut configs, &displaced, replace_conflicts.unwrap_or(false))?;
        }
        let label_changed = clean_label.as_ref().is_some_and(|l| *l != current.label);

        let config = configs.iter_mut().find(|c| c.id == listener_id).unwrap();
        if let Some(lbl) = clean_label {
            config.label = lbl;
        }
        if let Some(addr) = clean_address {
            config.address = addr;
        }
        // Label is baked into the running server for traffic tagging, so restart on either change
        (address_changed || label_changed, current.enabled)
    };

    persist_listener_configs(&state).await;
    sync_legacy_primary(&state).await;

    if needs_restart && enabled {
        reload_listeners(&app_handle, &state).await;
    }

    Ok(listener_statuses(&state).await)
}

#[tauri::command]
pub async fn set_listener_enabled(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    listener_id: u32,
    enabled: bool,
) -> Result<Vec<ListenerStatus>, String> {
    {
        let mut configs = state.listener_configs.write().await;
        let current = configs
            .iter()
            .find(|c| c.id == listener_id)
            .cloned()
            .ok_or_else(|| format!("Listener with ID {} not found", listener_id))?;
        if enabled && !current.enabled {
            let candidate: SocketAddr = current.address.parse().map_err(|e| format!("{}", e))?;
            if !check_address_conflict(candidate, &configs, Some(listener_id))?.is_empty() {
                return Err(format!("{} conflicts with other listeners on the same port", current.address));
            }
        }
        configs.iter_mut().find(|c| c.id == listener_id).unwrap().enabled = enabled;
    }

    persist_listener_configs(&state).await;

    reload_listeners(&app_handle, &state).await;

    Ok(listener_statuses(&state).await)
}

// --- Internal Helpers ---

async fn listener_statuses(state: &AppState) -> Vec<ListenerStatus> {
    let configs = state.listener_configs.read().await.clone();
    let running = state.listeners.read().await;
    configs
        .into_iter()
        .map(|config| ListenerStatus {
            running: running.iter().any(|l| l.id == config.id),
            error: state.listener_errors.get(&config.id).map(|e| e.clone()),
            config,
        })
        .collect()
}

async fn persist_listener_configs(state: &AppState) {
    let configs = state.listener_configs.read().await;
    let json = serde_json::to_string(&*configs).unwrap_or_else(|_| "[]".to_string());
    let _ = set_preference(&state.db_path, "proxy_listeners", &json);
}

/// Mirrors listener #0 into the legacy proxy_host/proxy_port fields.
async fn sync_legacy_primary(state: &AppState) {
    let primary = {
        let configs = state.listener_configs.read().await;
        configs.iter().find(|c| c.id == 0).and_then(|c| c.address.parse::<SocketAddr>().ok())
    };
    if let Some(addr) = primary {
        let host = addr.ip().to_string();
        {
            let mut cfg = state.proxy_config.write().await;
            cfg.host = host.clone();
            cfg.port = addr.port();
        }
        let _ = set_preference(&state.db_path, "proxy_host", &host);
        let _ = set_preference(&state.db_path, "proxy_port", &addr.port().to_string());
    }
}

pub async fn start_all_listeners(app_handle: &AppHandle, state: &AppState) {
    state.listener_errors.clear();
    let configs = state.listener_configs.read().await.clone();

    let ca = match app_handle.path().app_data_dir() {
        Ok(dir) => Arc::new(crate::ca::get_ca(dir.join("ca"))),
        Err(e) => {
            for c in &configs {
                state.listener_errors.insert(c.id, e.to_string());
            }
            return;
        }
    };

    for config in configs.into_iter().filter(|c| c.enabled) {
        let tcp_listener = match config.address.parse::<SocketAddr>() {
            Ok(addr) => bind_proxy_listener(addr).await,
            Err(e) => Err(format!("Invalid bind address '{}': {}", config.address, e)),
        };
        let tcp_listener = match tcp_listener {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to start listener '{}': {}", config.label, e);
                state.listener_errors.insert(config.id, e);
                continue;
            }
        };

        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        state.listeners.write().await.push(ListenerInstance { id: config.id, stop_tx: Some(stop_tx) });

        let state_arc = Arc::new(state.clone());
        let app_handle_clone = app_handle.clone();
        let ca = Arc::clone(&ca);
        tauri::async_runtime::spawn(async move {
            let _ = start_proxy_server(app_handle_clone, state_arc, ca, tcp_listener, config.label, stop_rx).await;
        });
    }
}

async fn stop_all_listeners(state: &AppState) -> bool {
    let mut listeners = state.listeners.write().await;
    let had_any = !listeners.is_empty();
    for instance in listeners.drain(..) {
        if let Some(tx) = instance.stop_tx {
            let _ = tx.send(());
        }
    }
    had_any
}

/// Re-applies the listener configs: stops every socket and rebinds the enabled set.
async fn reload_listeners(app_handle: &AppHandle, state: &AppState) {
    if !state.is_proxy_active() {
        state.listener_errors.clear();
        return;
    }
    if stop_all_listeners(state).await {
        // Give the OS a moment to release the ports
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    }
    start_all_listeners(app_handle, state).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(id: u32, label: &str, address: &str) -> ListenerConfig {
        ListenerConfig { id, label: label.into(), address: address.into(), enabled: true }
    }

    #[test]
    fn specific_ips_share_a_port() {
        let configs = vec![cfg(0, "Localhost", "127.0.0.1:8080"), cfg(1, "Mobile", "192.168.1.250:8080")];
        assert_eq!(check_address_conflict("192.168.1.250:8080".parse().unwrap(), &configs, Some(1)), Ok(vec![]));
        assert!(check_address_conflict("127.0.0.1:8080".parse().unwrap(), &configs, Some(1)).is_err());
    }

    #[test]
    fn wildcard_owns_its_port() {
        let configs = vec![cfg(0, "Any", "0.0.0.0:8080")];
        assert!(check_address_conflict("192.168.1.250:8080".parse().unwrap(), &configs, None).is_err());
        assert!(check_address_conflict("0.0.0.0:8080".parse().unwrap(), &configs, None).is_err());
        assert_eq!(check_address_conflict("127.0.0.1:9090".parse().unwrap(), &configs, None), Ok(vec![]));
    }

    #[test]
    fn wildcard_displaces_specific_ips_only_when_confirmed() {
        let mut configs = vec![
            cfg(0, "Localhost", "127.0.0.1:9000"),
            cfg(1, "Mobile", "192.168.1.250:8080"),
            cfg(2, "Lan", "10.0.0.2:8080"),
        ];
        let displaced = check_address_conflict("0.0.0.0:8080".parse().unwrap(), &configs, None).unwrap();
        assert_eq!(displaced, vec![1, 2]);
        assert!(resolve_displaced(&mut configs, &displaced, false).is_err());
        assert_eq!(configs.len(), 3);
        resolve_displaced(&mut configs, &displaced, true).unwrap();
        assert_eq!(configs.len(), 1);
        // Listener #0 is never displaced
        assert!(resolve_displaced(&mut vec![cfg(0, "L", "127.0.0.1:8080")], &[0], true).is_err());
    }

    #[test]
    fn normalizes_addresses() {
        assert_eq!(normalize_listener_address("localhost:8080").unwrap(), "127.0.0.1:8080");
        assert_eq!(normalize_listener_address("8081").unwrap(), "0.0.0.0:8081");
        assert!(normalize_listener_address("a:8080").is_err());
    }
}
