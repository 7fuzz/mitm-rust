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
    limiter_enabled: Option<bool>,
    enabled: Option<bool>,
    max_rows: Option<u32>,
    max_limit: Option<u32>,
) -> Result<HistorySettings, String> {
    let is_enabled = limiter_enabled.or(enabled).unwrap_or(true);
    let limit = max_rows.or(max_limit).unwrap_or(500);

    {
        let mut settings = state.history_settings.write().await;
        settings.limiter_enabled = is_enabled;
        settings.max_rows = limit;
    }

    let _ = set_preference(&state.db_path, "history_limiter_enabled", if is_enabled { "true" } else { "false" });
    let _ = set_preference(&state.db_path, "history_limiter_max_rows", &limit.to_string());

    if is_enabled {
        let _ = prune_history_logs(&state.db_path, limit);
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
    status_range: Option<String>,
    only_intercepted: Option<bool>,
    only_rewritten: Option<bool>,
    only_failed: Option<bool>,
    only_waiting: Option<bool>,
    include_bodies: Option<bool>,
) -> Result<Vec<HistoryEntry>, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    let offset = (page.saturating_sub(1)) * limit;
    let should_include_bodies = include_bodies.unwrap_or(false);

    let select_fields = if should_include_bodies {
        "id, method, url, host, path, content_type, response_size, status_code, request_headers, response_headers, request_body, response_body, phase, duration_ms, created_at, COALESCE(is_intercepted, 0), COALESCE(is_rewritten, 0), COALESCE(is_failed, 0)"
    } else {
        "id, method, url, host, path, content_type, response_size, status_code, phase, duration_ms, created_at, COALESCE(is_intercepted, 0), COALESCE(is_rewritten, 0), COALESCE(is_failed, 0)"
    };

    let mut query = format!("SELECT {} FROM history WHERE 1=1", select_fields);
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

    if let Some(ref range) = status_range {
        match range.as_str() {
            "2xx" => query.push_str(" AND (status_code >= 200 AND status_code < 300)"),
            "3xx" => query.push_str(" AND (status_code >= 300 AND status_code < 400)"),
            "4xx" => query.push_str(" AND (status_code >= 400 AND status_code < 500)"),
            "5xx" => query.push_str(" AND (status_code >= 500)"),
            _ => {}
        }
    }

    if let Some(true) = only_intercepted {
        query.push_str(" AND COALESCE(is_intercepted, 0) = 1");
    }

    if let Some(true) = only_rewritten {
        query.push_str(" AND COALESCE(is_rewritten, 0) = 1");
    }

    if let Some(true) = only_failed {
        query.push_str(" AND COALESCE(is_failed, 0) = 1");
    }

    if let Some(true) = only_waiting {
        query.push_str(" AND phase = 'request'");
    }

    query.push_str(" ORDER BY CAST(id AS INTEGER) DESC, created_at DESC LIMIT ? OFFSET ?");
    params.push(Box::new(limit));
    params.push(Box::new(offset));

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let logs = if should_include_bodies {
        stmt.query_map(&param_refs[..], |row| {
            let req_headers_json: String = row.get(8)?;
            let res_headers_json: String = row.get(9)?;
            let request_body: String = row.get(10)?;
            let response_body: String = row.get(11)?;

            let request_headers: Vec<(String, String)> = serde_json::from_str(&req_headers_json).unwrap_or_default();
            let response_headers: Vec<(String, String)> = serde_json::from_str(&res_headers_json).unwrap_or_default();

            let is_intercepted_int: i32 = row.get(15)?;
            let is_rewritten_int: i32 = row.get(16)?;
            let is_failed_int: i32 = row.get(17)?;

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
                is_intercepted: is_intercepted_int != 0,
                is_rewritten: is_rewritten_int != 0,
                is_failed: is_failed_int != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect()
    } else {
        stmt.query_map(&param_refs[..], |row| {
            let is_intercepted_int: i32 = row.get(11)?;
            let is_rewritten_int: i32 = row.get(12)?;
            let is_failed_int: i32 = row.get(13)?;

            Ok(HistoryEntry {
                id: row.get(0)?,
                method: row.get(1)?,
                url: row.get(2)?,
                host: row.get(3)?,
                path: row.get(4)?,
                content_type: row.get(5)?,
                response_size: row.get(6)?,
                status_code: row.get(7)?,
                request_headers: Vec::new(),
                response_headers: Vec::new(),
                request_body: String::new(),
                response_body: String::new(),
                phase: row.get(8)?,
                duration_ms: row.get(9)?,
                created_at: row.get(10)?,
                is_intercepted: is_intercepted_int != 0,
                is_rewritten: is_rewritten_int != 0,
                is_failed: is_failed_int != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect()
    };

    Ok(logs)
}

#[tauri::command]
pub async fn get_history_detail(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<crate::state::HistoryDetail>, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, request_headers, response_headers, request_body, response_body FROM history WHERE id = ?")
        .map_err(|e| e.to_string())?;

    let mut rows = stmt
        .query_map([&id], |row| {
            let req_headers_json: String = row.get(1)?;
            let res_headers_json: String = row.get(2)?;
            let request_body: String = row.get(3)?;
            let response_body: String = row.get(4)?;

            let request_headers: Vec<(String, String)> = serde_json::from_str(&req_headers_json).unwrap_or_default();
            let response_headers: Vec<(String, String)> = serde_json::from_str(&res_headers_json).unwrap_or_default();

            Ok(crate::state::HistoryDetail {
                id: row.get(0)?,
                request_headers,
                response_headers,
                request_body,
                response_body,
            })
        })
        .map_err(|e| e.to_string())?;

    if let Some(res) = rows.next() {
        let detail = res.map_err(|e| e.to_string())?;
        Ok(Some(detail))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn get_history_count(
    state: State<'_, AppState>,
    search_term: Option<String>,
    method_filter: Option<String>,
    status_filter: Option<u16>,
    status_range: Option<String>,
    only_intercepted: Option<bool>,
    only_rewritten: Option<bool>,
    only_failed: Option<bool>,
    only_waiting: Option<bool>,
) -> Result<u64, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    let mut query = String::from("SELECT COUNT(*) FROM history WHERE 1=1");
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

    if let Some(ref range) = status_range {
        match range.as_str() {
            "2xx" => query.push_str(" AND (status_code >= 200 AND status_code < 300)"),
            "3xx" => query.push_str(" AND (status_code >= 300 AND status_code < 400)"),
            "4xx" => query.push_str(" AND (status_code >= 400 AND status_code < 500)"),
            "5xx" => query.push_str(" AND (status_code >= 500)"),
            _ => {}
        }
    }

    if let Some(true) = only_intercepted {
        query.push_str(" AND COALESCE(is_intercepted, 0) = 1");
    }

    if let Some(true) = only_rewritten {
        query.push_str(" AND COALESCE(is_rewritten, 0) = 1");
    }

    if let Some(true) = only_failed {
        query.push_str(" AND COALESCE(is_failed, 0) = 1");
    }

    if let Some(true) = only_waiting {
        query.push_str(" AND phase = 'request'");
    }

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let count: u64 = stmt.query_row(&param_refs[..], |row| row.get(0)).map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
pub async fn clear_history_logs(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM history", []).map_err(|e| e.to_string())?;
    let _ = conn.execute("DELETE FROM sqlite_sequence WHERE name = 'history'", []);
    state.next_history_id.store(0, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}
