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
    pub name: String,
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

pub fn get_repeater_tabs_db(db_path: &PathBuf) -> Result<Vec<RepeaterTab>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, method, url, headers_json, params_json, body_type, body_content, extract_rules_json, order_index, created_at_ms, updated_at_ms FROM repeaters ORDER BY order_index ASC, created_at_ms ASC")
        .map_err(|e| e.to_string())?;

    let tabs = stmt
        .query_map([], |row| {
            let headers_json: String = row.get(4)?;
            let params_json: String = row.get(5)?;
            let extract_rules_json: String = row.get(8)?;

            let headers: Vec<HeaderItem> = serde_json::from_str(&headers_json).unwrap_or_default();
            let params: Vec<ParamItem> = serde_json::from_str(&params_json).unwrap_or_default();
            let extract_rules: Vec<ExtractRuleItem> = serde_json::from_str(&extract_rules_json).unwrap_or_default();

            Ok(RepeaterTab {
                id: row.get(0)?,
                name: row.get(1)?,
                method: row.get(2)?,
                url: row.get(3)?,
                headers,
                params,
                body_type: row.get(6)?,
                body_content: row.get(7)?,
                extract_rules,
                order_index: row.get(9)?,
                created_at_ms: row.get(10)?,
                updated_at_ms: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(tabs)
}

pub fn create_repeater_tab_db(db_path: &PathBuf, tab: &RepeaterTab) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let headers_json = serde_json::to_string(&tab.headers).unwrap_or_else(|_| "[]".to_string());
    let params_json = serde_json::to_string(&tab.params).unwrap_or_else(|_| "[]".to_string());
    let extract_rules_json = serde_json::to_string(&tab.extract_rules).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT OR REPLACE INTO repeaters (id, name, method, url, headers_json, params_json, body_type, body_content, extract_rules_json, order_index, created_at_ms, updated_at_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            tab.id,
            tab.name,
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
    ).map_err(|e| e.to_string())?;

    Ok(())
}

pub fn update_repeater_tab_db(db_path: &PathBuf, tab: &RepeaterTab) -> Result<(), String> {
    create_repeater_tab_db(db_path, tab)
}

pub fn delete_repeater_tab_db(db_path: &PathBuf, id: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM repeaters WHERE id = ?", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_repeater_tab_by_id(db_path: &PathBuf, id: &str) -> Result<RepeaterTab, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, method, url, headers_json, params_json, body_type, body_content, extract_rules_json, order_index, created_at_ms, updated_at_ms FROM repeaters WHERE id = ?")
        .map_err(|e| e.to_string())?;

    let tab = stmt.query_row(params![id], |row| {
        let headers_json: String = row.get(4)?;
        let params_json: String = row.get(5)?;
        let extract_rules_json: String = row.get(8)?;

        let headers: Vec<HeaderItem> = serde_json::from_str(&headers_json).unwrap_or_default();
        let params: Vec<ParamItem> = serde_json::from_str(&params_json).unwrap_or_default();
        let extract_rules: Vec<ExtractRuleItem> = serde_json::from_str(&extract_rules_json).unwrap_or_default();

        Ok(RepeaterTab {
            id: row.get(0)?,
            name: row.get(1)?,
            method: row.get(2)?,
            url: row.get(3)?,
            headers,
            params,
            body_type: row.get(6)?,
            body_content: row.get(7)?,
            extract_rules,
            order_index: row.get(9)?,
            created_at_ms: row.get(10)?,
            updated_at_ms: row.get(11)?,
        })
    }).map_err(|e| e.to_string())?;

    Ok(tab)
}

pub fn insert_repeater_history_db(db_path: &PathBuf, history: &RepeaterHistoryItem) -> Result<i64, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let req_headers_json = serde_json::to_string(&history.request_headers).unwrap_or_else(|_| "[]".to_string());
    let res_headers_json = serde_json::to_string(&history.response_headers).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO repeater_histories (repeater_id, method, url, request_headers_json, request_body, status_code, response_headers_json, response_body, duration_ms, executed_at_ms)
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

    let mut stmt = conn
        .prepare("SELECT id, repeater_id, method, url, request_headers_json, request_body, status_code, response_headers_json, response_body, duration_ms, executed_at_ms FROM repeater_histories WHERE repeater_id = ? ORDER BY executed_at_ms DESC, id DESC LIMIT ? OFFSET ?")
        .map_err(|e| e.to_string())?;

    let histories = stmt
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

    Ok(histories)
}
