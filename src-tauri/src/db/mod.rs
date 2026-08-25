pub mod actor;
pub mod migrations;

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
