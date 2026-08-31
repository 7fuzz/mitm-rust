use tauri::State;
use crate::state::{AppState, RewriteRule, RewriteHistoryEntry};
use crate::db::{save_rewrite_rules as db_save_rewrite_rules, get_rewrite_history_logs, clear_rewrite_history_logs, set_preference};

#[tauri::command]
pub async fn get_rewrite_rules(state: State<'_, AppState>) -> Result<Vec<RewriteRule>, String> {
    let rules = state.rewrite_rules.read().await;
    Ok(rules.clone())
}

#[tauri::command]
pub async fn save_rewrite_rules(
    state: State<'_, AppState>,
    rules: Vec<RewriteRule>,
) -> Result<Vec<RewriteRule>, String> {
    db_save_rewrite_rules(&state.db_path, &rules)?;
    let mut state_rules = state.rewrite_rules.write().await;
    *state_rules = rules.clone();
    Ok(rules)
}

#[tauri::command]
pub async fn toggle_rewrite_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    state.set_rewrite_enabled(enabled);
    let _ = set_preference(&state.db_path, "rewrite_enabled", if enabled { "true" } else { "false" });
    Ok(enabled)
}

#[tauri::command]
pub async fn get_rewrite_enabled(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.is_rewrite_enabled())
}

#[tauri::command]
pub async fn get_rewrite_history(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<RewriteHistoryEntry>, String> {
    let lim = limit.unwrap_or(200);
    get_rewrite_history_logs(&state.db_path, lim)
}

#[tauri::command]
pub async fn clear_rewrite_history(state: State<'_, AppState>) -> Result<(), String> {
    clear_rewrite_history_logs(&state.db_path)
}
