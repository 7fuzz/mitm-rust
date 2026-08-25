use tauri::State;
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::AppState;
use crate::repeater::{
    create_repeater_tab_db, delete_repeater_tab_db, get_repeater_history_db,
    get_repeater_tabs_db, insert_repeater_history_db, update_repeater_tab_db, HeaderItem,
    RepeaterExecutionResult, RepeaterHistoryItem, RepeaterTab,
};
use crate::repeater::execute::execute_tab_request;

#[tauri::command]
pub async fn get_repeater_tabs(
    state: State<'_, AppState>,
) -> Result<Vec<RepeaterTab>, String> {
    get_repeater_tabs_db(&state.db_path)
}

#[tauri::command]
pub async fn create_repeater_tab(
    state: State<'_, AppState>,
    _name: Option<String>,
    _from_history_id: Option<String>,
) -> Result<RepeaterTab, String> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let tab_id = Uuid::new_v4().to_string();

    let tab = RepeaterTab {
        id: tab_id,
        method: "GET".to_string(),
        url: "https://httpbin.org/get".to_string(),
        headers: vec![HeaderItem {
            id: Uuid::new_v4().to_string(),
            key: "User-Agent".to_string(),
            value: "MITM-Developer-Studio/2.0".to_string(),
            enabled: true,
        }],
        params: vec![],
        body_type: "none".to_string(),
        body_content: None,
        extract_rules: vec![],
        order_index: 0,
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
        execution_count: 0,
        last_status_code: None,
        last_duration_ms: None,
    };

    create_repeater_tab_db(&state.db_path, &tab)?;
    Ok(tab)
}

#[tauri::command]
pub async fn update_repeater_tab(
    state: State<'_, AppState>,
    tab: RepeaterTab,
) -> Result<(), String> {
    update_repeater_tab_db(&state.db_path, &tab)
}

#[tauri::command]
pub async fn delete_repeater_tab(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    delete_repeater_tab_db(&state.db_path, &id)
}

#[tauri::command]
pub async fn execute_repeater_request(
    state: State<'_, AppState>,
    id: String,
) -> Result<RepeaterExecutionResult, String> {
    execute_tab_request(&state.db_path, &id).await
}

#[tauri::command]
pub async fn get_repeater_history(
    state: State<'_, AppState>,
    repeater_id: String,
    page: u32,
    limit: u32,
) -> Result<Vec<RepeaterHistoryItem>, String> {
    get_repeater_history_db(&state.db_path, &repeater_id, page, limit)
}

#[tauri::command]
pub async fn insert_repeater_history(
    state: State<'_, AppState>,
    history: RepeaterHistoryItem,
) -> Result<i64, String> {
    insert_repeater_history_db(&state.db_path, &history)
}
