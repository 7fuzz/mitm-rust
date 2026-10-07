use std::path::PathBuf;
use rusqlite::Connection;
use crate::state::{HistoryDetail, HistoryEntry};

pub fn get_history_logs(
    db_path: &PathBuf,
    limit: u32,
    offset: u32,
    search_term: Option<&str>,
    method_filter: Option<&str>,
    status_range: Option<&str>,
    include_bodies: bool,
) -> Result<Vec<HistoryEntry>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    let select_fields = if include_bodies {
        "id, method, url, host, path, content_type, response_size, status_code, request_headers, response_headers, request_body, response_body, phase, duration_ms, created_at, COALESCE(is_intercepted, 0), COALESCE(is_rewritten, 0), COALESCE(is_failed, 0), COALESCE(listener_label, ''), request_at, response_at"
    } else {
        "id, method, url, host, path, content_type, response_size, status_code, phase, duration_ms, created_at, COALESCE(is_intercepted, 0), COALESCE(is_rewritten, 0), COALESCE(is_failed, 0), COALESCE(listener_label, ''), request_at, response_at"
    };

    let mut query = format!("SELECT {} FROM history WHERE 1=1", select_fields);
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(search) = search_term {
        if !search.trim().is_empty() {
            query.push_str(" AND (url LIKE ? OR host LIKE ? OR method LIKE ?)");
            let pattern = format!("%{}%", search.trim());
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern));
        }
    }

    if let Some(method) = method_filter {
        if !method.trim().is_empty() && method.to_uppercase() != "ALL" {
            query.push_str(" AND method = ?");
            params.push(Box::new(method.to_uppercase()));
        }
    }

    if let Some(range) = status_range {
        match range {
            "2xx" => query.push_str(" AND (status_code >= 200 AND status_code < 300)"),
            "3xx" => query.push_str(" AND (status_code >= 300 AND status_code < 400)"),
            "4xx" => query.push_str(" AND (status_code >= 400 AND status_code < 500)"),
            "5xx" => query.push_str(" AND (status_code >= 500)"),
            _ => {}
        }
    }

    query.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
    params.push(Box::new(limit));
    params.push(Box::new(offset));

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    if include_bodies {
        let entries = stmt.query_map(&param_refs[..], |row| {
            let req_headers_json: String = row.get(8).unwrap_or_else(|_| "[]".to_string());
            let res_headers_json: String = row.get(9).unwrap_or_else(|_| "[]".to_string());
            let request_headers: Vec<(String, String)> = serde_json::from_str(&req_headers_json).unwrap_or_default();
            let response_headers: Vec<(String, String)> = serde_json::from_str(&res_headers_json).unwrap_or_default();
            let request_body: String = row.get(10).unwrap_or_default();
            let response_body: String = row.get(11).unwrap_or_default();
            let is_intercepted_int: i32 = row.get(15)?;
            let is_rewritten_int: i32 = row.get(16)?;
            let is_failed_int: i32 = row.get(17)?;
            let listener_label: String = row.get(18)?;

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
                listener_label,
                request_at: row.get(19)?,
                response_at: row.get(20)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        Ok(entries)
    } else {
        let entries = stmt.query_map(&param_refs[..], |row| {
            let is_intercepted_int: i32 = row.get(11)?;
            let is_rewritten_int: i32 = row.get(12)?;
            let is_failed_int: i32 = row.get(13)?;
            let listener_label: String = row.get(14)?;

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
                listener_label,
                request_at: row.get(15)?,
                response_at: row.get(16)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        Ok(entries)
    }
}

pub fn get_history_detail(
    db_path: &PathBuf,
    id: i64,
) -> Result<Option<HistoryDetail>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, request_headers, response_headers, request_body, response_body FROM history WHERE id = ?")
        .map_err(|e| e.to_string())?;

    let mut rows = stmt
        .query_map([id], |row| {
            let req_headers_json: String = row.get(1)?;
            let res_headers_json: String = row.get(2)?;
            let request_body: String = row.get(3)?;
            let response_body: String = row.get(4)?;

            let request_headers: Vec<(String, String)> = serde_json::from_str(&req_headers_json).unwrap_or_default();
            let response_headers: Vec<(String, String)> = serde_json::from_str(&res_headers_json).unwrap_or_default();

            Ok(HistoryDetail {
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

pub fn clear_history(db_path: &PathBuf) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "DELETE FROM history;
         DELETE FROM sqlite_sequence WHERE name = 'history';"
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_history_entry(db_path: &PathBuf, id: i64) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM history WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
