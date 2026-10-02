use tauri::{AppHandle, Manager, State};
use crate::db::{save_intercept_rules, set_preference, InterceptRule};
use crate::state::{AppState, InterceptAction, PendingFlowPayload, ProxyConfig, SourceScope};

#[tauri::command]
pub async fn toggle_interceptor(
    state: State<'_, AppState>,
    enabled: bool,
    mode: String,
) -> Result<ProxyConfig, String> {
    {
        let mut cfg = state.proxy_config.write().await;
        cfg.intercept_enabled = enabled;
        cfg.intercept_mode = mode.clone();
    }

    let _ = set_preference(&state.db_path, "intercept_enabled", if enabled { "true" } else { "false" });
    let _ = set_preference(&state.db_path, "intercept_mode", &mode);

    let cfg = state.proxy_config.read().await;
    Ok(cfg.clone())
}

#[tauri::command]
pub async fn set_intercept_source_scope(
    state: State<'_, AppState>,
    scope: SourceScope,
) -> Result<ProxyConfig, String> {
    let json = serde_json::to_string(&scope).map_err(|e| e.to_string())?;
    let _ = set_preference(&state.db_path, "intercept_source_scope", &json);
    state.proxy_config.write().await.intercept_source_scope = scope;
    let cfg = state.proxy_config.read().await;
    Ok(cfg.clone())
}

#[tauri::command]
pub async fn update_intercept_rules(
    state: State<'_, AppState>,
    rules: Vec<InterceptRule>,
) -> Result<Vec<InterceptRule>, String> {
    save_intercept_rules(&state.db_path, &rules)?;

    {
        let mut rules_guard = state.rules.write().await;
        *rules_guard = rules.clone();
    }

    Ok(rules)
}

#[tauri::command]
pub async fn get_intercept_rules(
    state: State<'_, AppState>,
) -> Result<Vec<InterceptRule>, String> {
    let rules = state.rules.read().await.clone();
    Ok(rules)
}

#[tauri::command]
pub async fn forward_intercepted_flow(
    state: State<'_, AppState>,
    flow_id: String,
    modified_url: Option<String>,
    modified_method: Option<String>,
    modified_headers_json: Option<String>,
    modified_body: Option<Vec<u8>>,
) -> Result<(), String> {
    if let Some((_, pending)) = state.pending_flows.remove(&flow_id) {
        let modified_headers = if let Some(ref json_str) = modified_headers_json {
            serde_json::from_str::<Vec<(String, String)>>(json_str).ok()
        } else {
            None
        };

        let action = InterceptAction::Forward {
            modified_url,
            modified_method,
            modified_headers,
            modified_body,
        };

        let _ = pending.tx.send(action);
        Ok(())
    } else {
        Err(format!("Flow with ID {} not found", flow_id))
    }
}

#[tauri::command]
pub async fn drop_intercepted_flow(
    state: State<'_, AppState>,
    flow_id: String,
) -> Result<(), String> {
    if let Some((_, pending)) = state.pending_flows.remove(&flow_id) {
        let _ = pending.tx.send(InterceptAction::Drop);
        Ok(())
    } else {
        Err(format!("Flow with ID {} not found", flow_id))
    }
}

#[tauri::command]
pub async fn forward_all_intercepted_flows(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let keys: Vec<String> = state.pending_flows.iter().map(|kv| kv.key().clone()).collect();
    for key in keys {
        if let Some((_, pending)) = state.pending_flows.remove(&key) {
            let _ = pending.tx.send(InterceptAction::Forward {
                modified_url: None,
                modified_method: None,
                modified_headers: None,
                modified_body: None,
            });
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn drop_all_intercepted_flows(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let keys: Vec<String> = state.pending_flows.iter().map(|kv| kv.key().clone()).collect();
    for key in keys {
        if let Some((_, pending)) = state.pending_flows.remove(&key) {
            let _ = pending.tx.send(InterceptAction::Drop);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_pending_flows(
    state: State<'_, AppState>,
) -> Result<Vec<PendingFlowPayload>, String> {
    let payloads = state
        .pending_flows
        .iter()
        .map(|kv| {
            let p = kv.value();
            let body_text = String::from_utf8(p.body.clone()).unwrap_or_else(|_| format!("<binary data {} bytes>", p.body.len()));
            PendingFlowPayload {
                flow_id: p.flow_id.clone(),
                phase: match p.phase {
                    crate::state::InterceptPhase::Request => "request".to_string(),
                    crate::state::InterceptPhase::Response => "response".to_string(),
                },
                method: p.method.clone(),
                url: p.url.clone(),
                headers: p.headers.clone(),
                body: p.body.clone(),
                body_text,
            }
        })
        .collect();

    Ok(payloads)
}

#[tauri::command]
pub fn focus_app_window(app_handle: AppHandle) -> Result<(), String> {
    for (_, window) in app_handle.webview_windows() {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.set_always_on_top(true);
        let _ = window.set_always_on_top(false);
        let _ = window.request_user_attention(Some(tauri::UserAttentionType::Critical));
    }
    Ok(())
}

#[tauri::command]
pub fn set_focus_preference(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    set_preference(&state.db_path, "focus_on_intercepted", if enabled { "true" } else { "false" })?;
    Ok(())
}

#[tauri::command]
pub fn get_focus_preference(state: State<'_, AppState>) -> Result<bool, String> {
    let val = crate::db::get_preference(&state.db_path, "focus_on_intercepted");
    Ok(val.as_deref() != Some("false"))
}
