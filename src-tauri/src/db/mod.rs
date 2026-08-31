pub mod actor;
pub mod migrations;
pub mod webhook_db;
pub mod ws_db;

use tauri::AppHandle;
use tauri::Manager;
use std::path::PathBuf;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterceptRule {
    pub id: String,
    pub is_enabled: bool,
    pub target_phase: String, // 'request', 'response', 'both'
    pub match_field: String,  // 'url', 'host', 'path', 'method', 'header'
    pub operator: String,     // 'contains', 'equals', 'regex'
    pub match_value: String,
    pub order_index: i32,
    pub created_at_ms: i64,
    pub action: String,       // 'intercept' (whitelist) or 'pass' (blacklist)
}

pub fn get_db_path(app_handle: &AppHandle) -> PathBuf {
    let app_dir = app_handle.path().app_data_dir().expect("Failed to get app data dir");
    if !app_dir.exists() {
        let _ = std::fs::create_dir_all(&app_dir);
    }
    app_dir.join("mitm.db")
}

pub fn init_database(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let db_path = get_db_path(app_handle);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;"
    ).map_err(|e| e.to_string())?;

    migrations::run_all(app_handle, &conn)?;

    Ok(db_path)
}

pub fn get_preference(db_path: &PathBuf, key: &str) -> Option<String> {
    let conn = Connection::open(db_path).ok()?;
    conn.query_row(
        "SELECT value FROM app_preferences WHERE key = ?",
        [key],
        |row| row.get(0),
    ).ok()
}

pub fn set_preference(db_path: &PathBuf, key: &str, value: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO app_preferences (key, value) VALUES (?, ?)",
        params![key, value],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn prune_history_logs_conn(conn: &Connection, max_rows: u32) -> Result<usize, String> {
    if max_rows == 0 {
        return Ok(0);
    }
    conn.execute(
        "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY created_at DESC, rowid DESC LIMIT ?)",
        params![max_rows],
    ).map_err(|e| e.to_string())
}

pub fn prune_history_logs(db_path: &PathBuf, max_rows: u32) -> Result<usize, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    prune_history_logs_conn(&conn, max_rows)
}

pub fn load_intercept_rules(db_path: &PathBuf) -> Result<Vec<InterceptRule>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, is_enabled, target_phase, match_field, operator, match_value, order_index, created_at_ms, COALESCE(action, 'intercept') FROM intercept_rules ORDER BY order_index ASC")
        .map_err(|e| e.to_string())?;

    let rules = stmt
        .query_map([], |row| {
            let is_enabled_int: i32 = row.get(1)?;
            Ok(InterceptRule {
                id: row.get(0)?,
                is_enabled: is_enabled_int != 0,
                target_phase: row.get(2)?,
                match_field: row.get(3)?,
                operator: row.get(4)?,
                match_value: row.get(5)?,
                order_index: row.get(6)?,
                created_at_ms: row.get(7)?,
                action: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rules)
}

pub fn save_intercept_rules(db_path: &PathBuf, rules: &[InterceptRule]) -> Result<(), String> {
    let mut conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute("DELETE FROM intercept_rules", []).map_err(|e| e.to_string())?;

    for rule in rules {
        tx.execute(
            "INSERT INTO intercept_rules (id, is_enabled, target_phase, match_field, operator, match_value, order_index, created_at_ms, action)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                rule.id,
                if rule.is_enabled { 1 } else { 0 },
                rule.target_phase,
                rule.match_field,
                rule.operator,
                rule.match_value,
                rule.order_index,
                rule.created_at_ms,
                rule.action,
            ],
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_rewrite_rules(db_path: &PathBuf) -> Result<Vec<crate::state::RewriteRule>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, enabled, action_type, match_field, match_operator, match_value, target_part, target_header, match_pattern, replacement_value, is_regex, mock_status_code, mock_headers_json, mock_body, order_index, created_at_ms FROM rewrite_rules ORDER BY order_index ASC")
        .map_err(|e| e.to_string())?;

    let rules = stmt
        .query_map([], |row| {
            let enabled_int: i32 = row.get(2)?;
            let is_regex_int: i32 = row.get(11)?;
            Ok(crate::state::RewriteRule {
                id: row.get(0)?,
                name: row.get(1)?,
                enabled: enabled_int != 0,
                action_type: row.get(3)?,
                match_field: row.get(4)?,
                match_operator: row.get(5)?,
                match_value: row.get(6)?,
                target_part: row.get(7)?,
                target_header: row.get(8)?,
                match_pattern: row.get(9)?,
                replacement_value: row.get(10)?,
                is_regex: is_regex_int != 0,
                mock_status_code: row.get(12)?,
                mock_headers_json: row.get(13)?,
                mock_body: row.get(14)?,
                order_index: row.get(15)?,
                created_at_ms: row.get(16)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rules)
}

pub fn save_rewrite_rules(db_path: &PathBuf, rules: &[crate::state::RewriteRule]) -> Result<(), String> {
    let mut conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute("DELETE FROM rewrite_rules", []).map_err(|e| e.to_string())?;

    for rule in rules {
        tx.execute(
            "INSERT INTO rewrite_rules (id, name, enabled, action_type, match_field, match_operator, match_value, target_part, target_header, match_pattern, replacement_value, is_regex, mock_status_code, mock_headers_json, mock_body, order_index, created_at_ms)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                rule.id,
                rule.name,
                if rule.enabled { 1 } else { 0 },
                rule.action_type,
                rule.match_field,
                rule.match_operator,
                rule.match_value,
                rule.target_part,
                rule.target_header,
                rule.match_pattern,
                rule.replacement_value,
                if rule.is_regex { 1 } else { 0 },
                rule.mock_status_code,
                rule.mock_headers_json,
                rule.mock_body,
                rule.order_index,
                rule.created_at_ms,
            ],
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_rewrite_history_logs(db_path: &PathBuf, limit: u32) -> Result<Vec<crate::state::RewriteHistoryEntry>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, rule_id, rule_name, action_type, method, original_url, rewritten_url, original_headers, rewritten_headers, original_body, rewritten_body, status_code, duration_ms, created_at FROM rewrite_history ORDER BY created_at DESC, rowid DESC LIMIT ?")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![limit], |row| {
            let orig_headers_str: String = row.get(7).unwrap_or_else(|_| "[]".to_string());
            let rewr_headers_str: String = row.get(8).unwrap_or_else(|_| "[]".to_string());

            let original_headers = serde_json::from_str::<Vec<(String, String)>>(&orig_headers_str).unwrap_or_default();
            let rewritten_headers = serde_json::from_str::<Vec<(String, String)>>(&rewr_headers_str).unwrap_or_default();

            Ok(crate::state::RewriteHistoryEntry {
                id: row.get(0)?,
                rule_id: row.get(1)?,
                rule_name: row.get(2)?,
                action_type: row.get(3)?,
                method: row.get(4)?,
                original_url: row.get(5)?,
                rewritten_url: row.get(6)?,
                original_headers,
                rewritten_headers,
                original_body: row.get(9).unwrap_or_default(),
                rewritten_body: row.get(10).unwrap_or_default(),
                status_code: row.get(11)?,
                duration_ms: row.get(12)?,
                created_at: row.get(13)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}

pub fn clear_rewrite_history_logs(db_path: &PathBuf) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM rewrite_history", []).map_err(|e| e.to_string())?;
    Ok(())
}
