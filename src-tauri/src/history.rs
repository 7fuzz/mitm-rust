use tauri::AppHandle;
use crate::db;
use crate::proxy::Traffic;

#[tauri::command]
pub async fn clear_history(app_handle: AppHandle) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM history", []).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_history_item(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM history WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn save_traffic_history(app_handle: AppHandle, traffic: Traffic) -> Result<(), String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let req_headers_json = serde_json::to_string(&traffic.request_headers).map_err(|e| e.to_string())?;
    let res_headers_json = serde_json::to_string(&traffic.response_headers).map_err(|e| e.to_string())?;
    
    conn.execute(
        "INSERT OR REPLACE INTO history (id, method, url, host, status_code, request_headers, response_headers, request_body, response_body, phase, duration_ms, created_at) 
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
        rusqlite::params![
            traffic.id,
            traffic.method,
            traffic.url,
            traffic.host,
            traffic.status_code,
            req_headers_json,
            res_headers_json,
            traffic.request_body,
            traffic.response_body,
            traffic.phase,
            traffic.duration_ms,
        ],
    ).map_err(|e| e.to_string())?;

    // Enforce history limits from backend
    if let Ok(config_str) = conn.query_row::<String, _, _>(
        "SELECT value FROM app_state WHERE key = 'history_limits'",
        [],
        |row| row.get(0)
    ) {
        if let Ok(config_json) = serde_json::from_str::<serde_json::Value>(&config_str) {
            if let Some(enabled) = config_json.get("enabled").and_then(|v| v.as_bool()) {
                if enabled {
                    if let Some(limit) = config_json.get("value").and_then(|v| v.as_i64()) {
                        let _ = conn.execute(
                            "DELETE FROM history WHERE id NOT IN (
                                SELECT id FROM history ORDER BY created_at DESC, id DESC LIMIT ?
                             )",
                            rusqlite::params![limit],
                        );
                    }
                }
            }
        }
    }
    
    Ok(())
}
