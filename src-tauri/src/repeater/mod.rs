pub mod execute;

use std::path::PathBuf;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderItem {
    pub id: String,
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamItem {
    pub id: String,
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractRuleItem {
    pub id: String,
    pub r#type: String,
    pub expression: String,
    pub target_variable: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepeaterTab {
    pub id: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<HeaderItem>,
    pub params: Vec<ParamItem>,
    pub body_type: String,
    pub body_content: Option<String>,
    pub extract_rules: Vec<ExtractRuleItem>,
    pub order_index: i32,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    #[serde(default)]
    pub execution_count: u32,
    pub last_status_code: Option<u16>,
    pub last_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepeaterHistoryItem {
    pub id: i64,
    pub repeater_id: String,
    pub method: String,
    pub url: String,
    pub request_headers: Vec<HeaderItem>,
    pub request_body: Option<String>,
    pub status_code: u16,
    pub response_headers: Vec<HeaderItem>,
    pub response_body: Option<String>,
    pub duration_ms: u64,
    pub executed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepeaterExecutionResult {
    pub history_id: i64,
    pub repeater_id: String,
    pub status_code: u16,
    pub status_text: String,
    pub response_headers: Vec<HeaderItem>,
    pub response_body: String,
    pub duration_ms: u64,
    pub response_size: u64,
}

pub const REPEATER_WORKSPACE_ID: &str = "00000000-0000-4000-8000-000000000001";
/// Where Repeater tabs live until the Repeater UI gets folders.
pub const REPEATER_COLLECTION_ID: &str = "00000000-0000-4000-8000-000000000002";

/// The `requests` body buffer a Repeater body type is stored in.
fn body_column(body_type: &str) -> &'static str {
    match body_type {
        "json" => "body_json",
        "form" | "form-data" | "multipart" => "body_form_data",
        "urlencoded" | "x-www-form-urlencoded" => "body_urlencoded",
        _ => "body_raw",
    }
}

const TAB_SELECT: &str = "
    SELECT
        r.id, r.method, r.url, r.headers_json, r.params_json, r.body_type,
        r.body_json, r.body_raw, r.body_form_data, r.body_urlencoded,
        r.extract_rules_json, r.order_index, r.created_at_ms, r.updated_at_ms,
        (SELECT COUNT(*) FROM request_histories WHERE request_id = r.id) as execution_count,
        (SELECT status_code FROM request_histories WHERE request_id = r.id ORDER BY executed_at_ms DESC, id DESC LIMIT 1) as last_status_code,
        (SELECT duration_ms FROM request_histories WHERE request_id = r.id ORDER BY executed_at_ms DESC, id DESC LIMIT 1) as last_duration_ms
    FROM requests r
    JOIN collections c ON r.collection_id = c.id
";

fn row_to_tab(row: &rusqlite::Row) -> rusqlite::Result<RepeaterTab> {
    let headers_json: String = row.get(3)?;
    let params_json: String = row.get(4)?;
    let body_type: String = row.get(5)?;
    let extract_rules_json: String = row.get(10)?;

    let body_content: Option<String> = match body_column(&body_type) {
        "body_json" => row.get(6)?,
        "body_form_data" => row.get(8)?,
        "body_urlencoded" => row.get(9)?,
        _ => row.get(7)?,
    };
    let count: i64 = row.get(14)?;

    Ok(RepeaterTab {
        id: row.get(0)?,
        method: row.get(1)?,
        url: row.get(2)?,
        headers: serde_json::from_str(&headers_json).unwrap_or_default(),
        params: serde_json::from_str(&params_json).unwrap_or_default(),
        body_type,
        body_content,
        extract_rules: serde_json::from_str(&extract_rules_json).unwrap_or_default(),
        order_index: row.get(11)?,
        created_at_ms: row.get(12)?,
        updated_at_ms: row.get(13)?,
        execution_count: count as u32,
        last_status_code: row.get(15)?,
        last_duration_ms: row.get(16)?,
    })
}

pub fn get_repeater_tabs_db(db_path: &PathBuf) -> Result<Vec<RepeaterTab>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let query = format!(
        "{} WHERE c.workspace_id = ? ORDER BY r.order_index ASC, r.created_at_ms DESC",
        TAB_SELECT
    );
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let tabs = stmt
        .query_map(params![REPEATER_WORKSPACE_ID], row_to_tab)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(tabs)
}

/// An order index that places a new tab above every existing one.
pub fn top_order_index_db(db_path: &PathBuf) -> Result<i32, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let min: Option<i32> = conn
        .query_row(
            "SELECT MIN(r.order_index) FROM requests r
             JOIN collections c ON r.collection_id = c.id
             WHERE c.workspace_id = ?",
            params![REPEATER_WORKSPACE_ID],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(min.map(|m| m - 1).unwrap_or(0))
}

pub fn create_repeater_tab_db(db_path: &PathBuf, tab: &RepeaterTab) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    let headers_json = serde_json::to_string(&tab.headers).unwrap_or_else(|_| "[]".to_string());
    let params_json = serde_json::to_string(&tab.params).unwrap_or_else(|_| "[]".to_string());
    let extract_rules_json = serde_json::to_string(&tab.extract_rules).unwrap_or_else(|_| "[]".to_string());
    let body_col = body_column(&tab.body_type);
    let name = format!("{} {}", tab.method, tab.url);

    // Upsert in place: INSERT OR REPLACE deletes the old row first, which cascades
    // (foreign keys are on by default in the bundled SQLite) and wipes the request's run history.
    // The tab's folder and position are left alone on update.
    let sql = format!(
        "INSERT INTO requests (id, collection_id, name, method, url, headers_json, params_json, body_type, {col}, extract_rules_json, order_index, created_at_ms, updated_at_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            method = excluded.method,
            url = excluded.url,
            headers_json = excluded.headers_json,
            params_json = excluded.params_json,
            body_type = excluded.body_type,
            {col} = excluded.{col},
            extract_rules_json = excluded.extract_rules_json,
            updated_at_ms = excluded.updated_at_ms",
        col = body_col
    );
    conn.execute(
        &sql,
        params![
            tab.id,
            REPEATER_COLLECTION_ID,
            name,
            tab.method,
            tab.url,
            headers_json,
            params_json,
            tab.body_type,
            tab.body_content,
            extract_rules_json,
            tab.order_index,
            tab.created_at_ms,
            tab.updated_at_ms,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn update_repeater_tab_db(db_path: &PathBuf, tab: &RepeaterTab) -> Result<(), String> {
    create_repeater_tab_db(db_path, tab)
}

pub fn delete_repeater_tab_db(db_path: &PathBuf, id: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM request_histories WHERE request_id = ?", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM requests WHERE id = ?", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_repeater_tab_by_id(db_path: &PathBuf, id: &str) -> Result<RepeaterTab, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let query = format!("{} WHERE r.id = ?", TAB_SELECT);
    conn.query_row(&query, params![id], row_to_tab)
        .map_err(|e| e.to_string())
}

pub fn insert_repeater_history_db(
    db_path: &PathBuf,
    history: &RepeaterHistoryItem,
) -> Result<i64, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    let req_headers_json = serde_json::to_string(&history.request_headers).unwrap_or_else(|_| "[]".to_string());
    let res_headers_json = serde_json::to_string(&history.response_headers).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO request_histories
            (request_id, method, url, request_headers_json, request_body, status_code, response_headers_json, response_body, duration_ms, executed_at_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            history.repeater_id,
            history.method,
            history.url,
            req_headers_json,
            history.request_body,
            history.status_code,
            res_headers_json,
            history.response_body,
            history.duration_ms,
            history.executed_at_ms,
        ],
    ).map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

pub fn get_repeater_history_db(
    db_path: &PathBuf,
    repeater_id: &str,
    page: u32,
    limit: u32,
) -> Result<Vec<RepeaterHistoryItem>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let offset = (page.saturating_sub(1)) * limit;

    let query = "
        SELECT id, request_id, method, url, request_headers_json, request_body, status_code, response_headers_json, response_body, duration_ms, executed_at_ms
        FROM request_histories
        WHERE request_id = ?
        ORDER BY executed_at_ms DESC, id DESC
        LIMIT ? OFFSET ?
    ";

    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;

    let history = stmt
        .query_map(params![repeater_id, limit, offset], |row| {
            let req_headers_json: String = row.get(4)?;
            let res_headers_json: String = row.get(7)?;

            let request_headers: Vec<HeaderItem> = serde_json::from_str(&req_headers_json).unwrap_or_default();
            let response_headers: Vec<HeaderItem> = serde_json::from_str(&res_headers_json).unwrap_or_default();

            Ok(RepeaterHistoryItem {
                id: row.get(0)?,
                repeater_id: row.get(1)?,
                method: row.get(2)?,
                url: row.get(3)?,
                request_headers,
                request_body: row.get(5)?,
                status_code: row.get(6)?,
                response_headers,
                response_body: row.get(8)?,
                duration_ms: row.get(9)?,
                executed_at_ms: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(history)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply_migrations(db_path: &PathBuf) {
        apply_migrations_up_to(db_path, u32::MAX);
    }

    fn apply_migrations_up_to(db_path: &PathBuf, version: u32) {
        let conn = Connection::open(db_path).unwrap();
        for m in crate::db::migrations::MIGRATIONS.iter().filter(|m| m.version <= version) {
            for statement in m.sql.split(';') {
                let trimmed = statement.trim();
                if !trimmed.is_empty() {
                    let _ = conn.execute(trimmed, []);
                }
            }
        }
    }

    fn temp_db() -> PathBuf {
        std::env::temp_dir().join(format!("repeater-test-{}.db", uuid::Uuid::new_v4()))
    }

    fn blank_tab(id: &str, order_index: i32, created_at_ms: i64) -> RepeaterTab {
        RepeaterTab {
            id: id.into(),
            method: "POST".into(),
            url: "https://example.com".into(),
            headers: vec![],
            params: vec![],
            body_type: "json".into(),
            body_content: Some("{}".into()),
            extract_rules: vec![],
            order_index,
            created_at_ms,
            updated_at_ms: created_at_ms,
            execution_count: 0,
            last_status_code: None,
            last_duration_ms: None,
        }
    }

    #[test]
    fn migration_moves_repeater_tabs_and_runs_into_the_repeater_workspace() {
        let db_path = temp_db();
        apply_migrations_up_to(&db_path, 15);
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "INSERT INTO repeaters (id, method, url, body_type, body_content, created_at_ms, updated_at_ms)
                    VALUES ('old', 'GET', 'https://a', 'raw', 'hello', 1, 1),
                           ('new', 'POST', 'https://b', 'json', '{\"a\":1}', 2, 2);
                 INSERT INTO repeater_histories (repeater_id, method, url, request_headers_json, status_code, response_headers_json, duration_ms, executed_at_ms)
                    VALUES ('old', 'GET', 'https://a', '[]', 200, '[]', 5, 10);",
            )
            .unwrap();
        }
        apply_migrations(&db_path);

        let tabs = get_repeater_tabs_db(&db_path).unwrap();
        let history = get_repeater_history_db(&db_path, "old", 1, 10).unwrap();
        let old_table_gone = Connection::open(&db_path).unwrap().prepare("SELECT 1 FROM repeaters").is_err();
        let _ = std::fs::remove_file(&db_path);

        assert_eq!(tabs.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["new", "old"]);
        assert_eq!(tabs[0].body_content.as_deref(), Some("{\"a\":1}"));
        assert_eq!(tabs[1].body_content.as_deref(), Some("hello"));
        assert_eq!(tabs[1].execution_count, 1);
        assert_eq!(history.len(), 1);
        assert!(old_table_gone);
    }

    #[test]
    fn a_new_tab_goes_to_the_top() {
        let db_path = temp_db();
        apply_migrations(&db_path);

        create_repeater_tab_db(&db_path, &blank_tab("first", top_order_index_db(&db_path).unwrap(), 1)).unwrap();
        create_repeater_tab_db(&db_path, &blank_tab("second", top_order_index_db(&db_path).unwrap(), 2)).unwrap();
        // Saving an older tab again must not move it
        create_repeater_tab_db(&db_path, &blank_tab("first", 99, 1)).unwrap();

        let ids: Vec<String> = get_repeater_tabs_db(&db_path).unwrap().into_iter().map(|t| t.id).collect();
        let _ = std::fs::remove_file(&db_path);
        assert_eq!(ids, ["second", "first"]);
    }

    #[test]
    fn updating_a_tab_keeps_its_run_history() {
        let db_path = temp_db();
        apply_migrations(&db_path);

        let mut tab = RepeaterTab {
            id: "tab-1".into(),
            method: "GET".into(),
            url: "https://example.com".into(),
            headers: vec![],
            params: vec![],
            body_type: "none".into(),
            body_content: None,
            extract_rules: vec![],
            order_index: 0,
            created_at_ms: 1,
            updated_at_ms: 1,
            execution_count: 0,
            last_status_code: None,
            last_duration_ms: None,
        };
        create_repeater_tab_db(&db_path, &tab).unwrap();

        for i in 0..3 {
            let run = RepeaterHistoryItem {
                id: 0,
                repeater_id: tab.id.clone(),
                method: "GET".into(),
                url: tab.url.clone(),
                request_headers: vec![],
                request_body: None,
                status_code: 200,
                response_headers: vec![],
                response_body: None,
                duration_ms: 10,
                executed_at_ms: i,
            };
            insert_repeater_history_db(&db_path, &run).unwrap();
            // Every send saves the tab first; that must not drop earlier runs
            tab.url = format!("https://example.com/{}", i);
            update_repeater_tab_db(&db_path, &tab).unwrap();
        }

        let history = get_repeater_history_db(&db_path, &tab.id, 1, 100).unwrap();
        let _ = std::fs::remove_file(&db_path);
        assert_eq!(history.len(), 3);
    }
}
