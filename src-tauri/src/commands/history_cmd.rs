use tauri::State;
use rusqlite::Connection;
use crate::db::{prune_history_logs, set_preference};
use crate::state::{AppState, HistoryDetailItem, HistorySettings, HistorySummaryItem};

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
) -> Result<Vec<HistorySummaryItem>, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    let offset = (page.saturating_sub(1)) * limit;
    let mut query = String::from(
        "SELECT id, uuid, method, url, host, status_code, response_headers, response_body, duration_ms, created_at FROM history WHERE 1=1"
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

    query.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
    params.push(Box::new(limit));
    params.push(Box::new(offset));

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let logs = stmt
        .query_map(&param_refs[..], |row| {
            let url: String = row.get(3)?;
            let host: String = row.get(4)?;
            let res_headers_json: String = row.get(6)?;
            let res_body: String = row.get(7)?;

            let res_headers: Vec<(String, String)> = serde_json::from_str(&res_headers_json).unwrap_or_default();
            let content_type = res_headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "-".to_string());

            let full_host = if host.starts_with("http://") || host.starts_with("https://") {
                host
            } else if url.starts_with("https://") {
                format!("https://{}", host)
            } else {
                format!("http://{}", host)
            };

            let path = parse_path_from_url(&url);

            Ok(HistorySummaryItem {
                id: row.get(0)?,
                uuid: row.get(1)?,
                method: row.get(2)?,
                url,
                host: full_host,
                path,
                content_type,
                response_size: res_body.len() as u64,
                status_code: row.get(5)?,
                duration_ms: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(logs)
}

fn parse_path_from_url(url_str: &str) -> String {
    if let Some(pos) = url_str.find("://") {
        let rest = &url_str[pos + 3..];
        if let Some(slash_pos) = rest.find('/') {
            return rest[slash_pos..].to_string();
        }
        return "/".to_string();
    }
    if url_str.starts_with('/') {
        url_str.to_string()
    } else {
        "/".to_string()
    }
}

#[tauri::command]
pub async fn get_history_detail(
    state: State<'_, AppState>,
    uuid: String,
) -> Result<HistoryDetailItem, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, uuid, method, url, host, status_code, request_headers, response_headers, request_body, response_body, phase, duration_ms, created_at FROM history WHERE uuid = ?"
        )
        .map_err(|e| e.to_string())?;

    let item = stmt
        .query_row([uuid.clone()], |row| {

            let req_headers_json: String = row.get(6)?;
            let res_headers_json: String = row.get(7)?;
            let request_body: String = row.get(8)?;
            let response_body: String = row.get(9)?;

            let request_headers: Vec<(String, String)> =
                serde_json::from_str(&req_headers_json).unwrap_or_default();
            let response_headers: Vec<(String, String)> =
                serde_json::from_str(&res_headers_json).unwrap_or_default();

            let request_body_hex = if !request_body.is_empty() {
                Some(hex::encode(request_body.as_bytes()))
            } else {
                None
            };

            let response_body_hex = if !response_body.is_empty() {
                Some(hex::encode(response_body.as_bytes()))
            } else {
                None
            };

            let host: String = row.get(4)?;
            let url: String = row.get(3)?;
            let full_host = if host.starts_with("http://") || host.starts_with("https://") {
                host
            } else if url.starts_with("https://") {
                format!("https://{}", host)
            } else {
                format!("http://{}", host)
            };

            Ok(HistoryDetailItem {
                id: row.get(0)?,
                uuid: row.get(1)?,
                method: row.get(2)?,
                url,
                host: full_host,
                status_code: row.get(5)?,
                request_headers,
                response_headers,
                request_body,
                response_body,
                request_body_hex,
                response_body_hex,
                phase: row.get(10)?,
                duration_ms: row.get(11)?,
                created_at: row.get(12)?,
            })
        })
        .map_err(|e| format!("History item with uuid {} not found: {}", uuid, e))?;


    Ok(item)
}

#[tauri::command]
pub async fn clear_history_logs(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM history", []).map_err(|e| e.to_string())?;
    Ok(())
}
