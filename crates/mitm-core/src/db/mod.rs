pub mod actor;
pub mod browser;
mod error;
pub mod history_db;
pub mod migrations;
pub mod recovery;
pub mod webhook_db;
pub mod ws_db;

pub use error::InitError;
pub use history_db::*;

use std::path::{Path, PathBuf};
use std::time::Duration;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

fn default_intercept_action() -> String {
    "intercept".to_string()
}

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
    #[serde(default = "default_intercept_action")]
    pub action: String,       // 'intercept' (whitelist) or 'pass' (blacklist)
}

pub fn get_default_db_path() -> PathBuf {
    let app_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mitm");
    if !app_dir.exists() {
        let _ = std::fs::create_dir_all(&app_dir);
    }
    app_dir.join("mitm.db")
}

/// Opens a connection with the settings SQLite keeps per connection; journal_mode lives in the file.
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}

pub fn init_database(db_path: &PathBuf, event_bus: Option<&crate::events::EventBus>) -> Result<(), InitError> {
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| InitError::Io(e.to_string()))?;
        }
    }
    let conn = open(db_path).map_err(InitError::from)?;
    conn.execute_batch("PRAGMA journal_mode = WAL;").map_err(InitError::from)?;

    migrations::run_all(event_bus, &conn).map_err(InitError::migration)
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
        "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT ?)",
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
        .prepare("SELECT id, rule_id, rule_name, action_type, method, original_url, rewritten_url, original_headers, rewritten_headers, original_body, rewritten_body, status_code, duration_ms, created_at FROM rewrite_history ORDER BY id DESC LIMIT ?")
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DataCategory {
    Traffic,
    WebhookDeliveries,
    WsMessages,
    FuzzRuns,
    RewriteHistory,
}

impl DataCategory {
    pub const ALL: [DataCategory; 5] = [
        DataCategory::Traffic,
        DataCategory::WebhookDeliveries,
        DataCategory::WsMessages,
        DataCategory::FuzzRuns,
        DataCategory::RewriteHistory,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Traffic => "Traffic",
            Self::WebhookDeliveries => "Webhook deliveries",
            Self::WsMessages => "WebSocket messages",
            Self::FuzzRuns => "Fuzz runs",
            Self::RewriteHistory => "Rewrite history",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataCategoryCounts {
    pub traffic: u64,
    pub webhook_deliveries: u64,
    pub ws_messages: u64,
    pub fuzz_runs: u64,
    pub rewrite_history: u64,
}

impl DataCategoryCounts {
    pub fn get(&self, category: DataCategory) -> u64 {
        match category {
            DataCategory::Traffic => self.traffic,
            DataCategory::WebhookDeliveries => self.webhook_deliveries,
            DataCategory::WsMessages => self.ws_messages,
            DataCategory::FuzzRuns => self.fuzz_runs,
            DataCategory::RewriteHistory => self.rewrite_history,
        }
    }
}

pub fn get_data_category_counts(db_path: &PathBuf) -> Result<DataCategoryCounts, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let traffic = conn.query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0)).unwrap_or(0);
    let webhook_deliveries = conn.query_row("SELECT COUNT(*) FROM webhook_deliveries", [], |r| r.get(0)).unwrap_or(0);
    let ws_messages = conn.query_row("SELECT COUNT(*) FROM ws_messages", [], |r| r.get(0)).unwrap_or(0);
    let fuzz_runs = conn.query_row("SELECT COUNT(*) FROM fuzz_runs", [], |r| r.get(0)).unwrap_or(0);
    let rewrite_history = conn.query_row("SELECT COUNT(*) FROM rewrite_history", [], |r| r.get(0)).unwrap_or(0);

    Ok(DataCategoryCounts {
        traffic,
        webhook_deliveries,
        ws_messages,
        fuzz_runs,
        rewrite_history,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("mitm-counts-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("test.db");
        init_database(&db, None).unwrap();
        (db, dir)
    }

    #[test]
    fn deleting_a_workspace_cascades_on_opened_connections() {
        let (db, dir) = temp_db();
        let conn = open(&db).unwrap();
        conn.execute_batch(
            "INSERT INTO workspaces (id, name, created_at_ms, updated_at_ms) VALUES ('w', 'W', 0, 0);
             INSERT INTO collections (id, workspace_id, name, created_at_ms, updated_at_ms) VALUES ('c', 'w', 'C', 0, 0);
             INSERT INTO requests (id, collection_id, name, method, url, created_at_ms, updated_at_ms) VALUES ('r', 'c', 'R', 'GET', 'http://a', 0, 0);
             INSERT INTO request_histories (request_id, method, url, request_headers_json, status_code, response_headers_json, duration_ms, executed_at_ms)
                 VALUES ('r', 'GET', 'http://a', '[]', 200, '[]', 1, 0);",
        )
        .unwrap();

        open(&db).unwrap().execute("DELETE FROM workspaces WHERE id = 'w'", []).unwrap();

        let count = |table: &str| -> i64 { conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).unwrap() };
        assert_eq!(count("collections WHERE id = 'c'"), 0);
        assert_eq!(count("requests WHERE id = 'r'"), 0);
        assert_eq!(count("request_histories WHERE request_id = 'r'"), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn opened_connections_wait_for_busy_writers() {
        let (db, dir) = temp_db();
        let timeout: i64 = open(&db).unwrap().query_row("PRAGMA busy_timeout", [], |r| r.get(0)).unwrap();
        assert!(timeout > 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn counts_and_clears_per_data_category() {
        let (db, dir) = temp_db();
        let counts = get_data_category_counts(&db).unwrap();
        assert_eq!(counts, DataCategoryCounts::default());

        let conn = Connection::open(&db).unwrap();
        conn.execute("INSERT INTO history (id, method, url, host, status_code, request_headers, response_headers, request_body, response_body, phase) VALUES (1, 'GET', 'http://a', 'a', 200, '[]', '[]', '', '', 'done')", []).unwrap();
        conn.execute("INSERT INTO webhook_deliveries (endpoint_id, endpoint_path, headers, payload, signature_status, computed_hmac, provided_hmac, timestamp) VALUES ('e1', '/w', '[]', '', 'none', '', '', 1000)", []).unwrap();
        conn.execute("INSERT INTO ws_connections (id, url, status, handshake_time) VALUES (1, 'ws://localhost', 'open', 1000)", []).unwrap();
        conn.execute("INSERT INTO ws_messages (connection_id, direction, msg_type, payload, timestamp, length, is_injected) VALUES (1, 'c2s', 'text', 'hi', 1000, 2, 0)", []).unwrap();
        conn.execute("INSERT INTO fuzz_runs (id, name, attack_type, config_json, total, created_at_ms) VALUES (1, 'run', 'sniper', '{}', 1, 1000)", []).unwrap();
        conn.execute("INSERT INTO rewrite_history (id, rule_id, rule_name, action_type, method, original_url, rewritten_url, original_headers, rewritten_headers, original_body, rewritten_body, status_code, duration_ms) VALUES (1, 'r1', 'rule', 'modify', 'GET', 'http://a', 'http://a', '[]', '[]', '', '', 200, 10)", []).unwrap();

        let counts = get_data_category_counts(&db).unwrap();
        assert_eq!(counts.traffic, 1);
        assert_eq!(counts.webhook_deliveries, 1);
        assert_eq!(counts.ws_messages, 1);
        assert_eq!(counts.fuzz_runs, 1);
        assert_eq!(counts.rewrite_history, 1);

        clear_history(&db).unwrap();
        webhook_db::clear_webhook_deliveries(&db).unwrap();
        ws_db::clear_ws_messages(&db, None).unwrap();
        crate::fuzzer::clear_fuzz_runs(&db).unwrap();
        clear_rewrite_history_logs(&db).unwrap();

        let cleared = get_data_category_counts(&db).unwrap();
        assert_eq!(cleared, DataCategoryCounts::default());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
