use tauri::State;
use rusqlite::Connection;
use crate::db::{prune_history_logs, set_preference};
use crate::state::{AppState, HistoryEntry, HistorySettings};

#[tauri::command]
pub async fn get_history_settings(
    state: State<'_, AppState>,
) -> Result<HistorySettings, String> {
    let settings = state.history_settings.read().await;
    Ok(settings.clone())
}

#[tauri::command]
pub async fn update_history_settings(
    state: State<'_, AppState>,
    enabled: bool,
    max_rows: u32,
) -> Result<HistorySettings, String> {
    {
        let mut settings = state.history_settings.write().await;
        settings.limiter_enabled = enabled;
        settings.max_rows = max_rows;
    }

    let _ = set_preference(&state.db_path, "history_limiter_enabled", if enabled { "true" } else { "false" });
    let _ = set_preference(&state.db_path, "history_limiter_max_rows", &max_rows.to_string());

    if enabled {
        let _ = prune_history_logs(&state.db_path, max_rows);
    }

    let updated = state.history_settings.read().await;
    Ok(updated.clone())
}

#[tauri::command]
pub async fn get_history_logs(
    state: State<'_, AppState>,
    page: u32,
    limit: u32,
    search_term: Option<String>,
    method_filter: Option<String>,
    status_filter: Option<u16>,
) -> Result<Vec<HistoryEntry>, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    let offset = (page.saturating_sub(1)) * limit;
    let mut query = String::from(
        "SELECT id, method, url, host, path, content_type, response_size, status_code, request_headers, response_headers, request_body, response_body, phase, duration_ms, created_at FROM history WHERE 1=1"
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref search) = search_term {
        if !search.trim().is_empty() {
            query.push_str(" AND (url LIKE ? OR host LIKE ? OR method LIKE ?)");
            let pattern = format!("%{}%", search.trim());
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern));
        }
    }

    if let Some(ref method) = method_filter {
        if !method.trim().is_empty() && method.to_uppercase() != "ALL" {
            query.push_str(" AND method = ?");
            params.push(Box::new(method.to_uppercase()));
        }
    }

    if let Some(status) = status_filter {
        query.push_str(" AND status_code = ?");
        params.push(Box::new(status));
    }

    query.push_str(" ORDER BY created_at DESC, rowid DESC LIMIT ? OFFSET ?");
    params.push(Box::new(limit));
    params.push(Box::new(offset));

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let logs = stmt
        .query_map(&param_refs[..], |row| {
            let req_headers_json: String = row.get(8)?;
            let res_headers_json: String = row.get(9)?;
            let request_body: String = row.get(10)?;
            let response_body: String = row.get(11)?;

            let request_headers: Vec<(String, String)> = serde_json::from_str(&req_headers_json).unwrap_or_default();
            let response_headers: Vec<(String, String)> = serde_json::from_str(&res_headers_json).unwrap_or_default();

            Ok(HistoryEntry {
                id: row.get(0)?,
                method: row.get(1)?,
                url: row.get(2)?,
                host: row.get(3)?,
                path: row.get(4)?,
                content_type: row.get(5)?,
                response_size: row.get(6)?,
                status_code: row.get(7)?,
                request_headers,
                response_headers,
                request_body,
                response_body,
                phase: row.get(12)?,
                duration_ms: row.get(13)?,
                created_at: row.get(14)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(logs)
}

#[tauri::command]
pub async fn clear_history_logs(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM history", []).map_err(|e| e.to_string())?;
    Ok(())
}
