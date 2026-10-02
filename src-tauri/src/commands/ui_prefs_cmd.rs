use std::collections::HashMap;
use rusqlite::{params, Connection};
use tauri::State;
use crate::state::AppState;

/// UI preferences live in `app_preferences` under this prefix so they never collide with backend settings.
const UI_PREF_PREFIX: &str = "ui.";

/// Returns every stored UI preference as `{ key (without prefix): JSON value }`.
#[tauri::command]
pub async fn get_ui_preferences(
    state: State<'_, AppState>,
) -> Result<HashMap<String, serde_json::Value>, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM app_preferences WHERE key LIKE 'ui.%'")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;

    let mut prefs = HashMap::new();
    for (key, raw) in rows.filter_map(|r| r.ok()) {
        // Skip values that are not valid JSON; the frontend falls back to its default
        if let Ok(value) = serde_json::from_str(&raw) {
            prefs.insert(key[UI_PREF_PREFIX.len()..].to_string(), value);
        }
    }
    Ok(prefs)
}

/// Stores one UI preference; `null` removes it so the frontend default applies again.
#[tauri::command]
pub async fn set_ui_preference(
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let full_key = format!("{}{}", UI_PREF_PREFIX, key);
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    if value.is_null() {
        conn.execute("DELETE FROM app_preferences WHERE key = ?", params![full_key])
            .map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "INSERT OR REPLACE INTO app_preferences (key, value) VALUES (?, ?)",
            params![full_key, value.to_string()],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
