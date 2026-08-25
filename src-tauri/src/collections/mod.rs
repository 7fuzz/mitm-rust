pub mod execute;

use std::path::PathBuf;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::repeater::{HeaderItem, ParamItem, ExtractRuleItem};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub workspace_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub order_index: i32,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestItem {
    pub id: String,
    pub collection_id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<HeaderItem>,
    pub params: Vec<ParamItem>,
    pub body_type: String,
    pub body_content: Option<String>,
    pub extract_rules: Vec<ExtractRuleItem>,
    pub description: Option<String>,
    pub order_index: i32,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionTreeItem {
    pub id: String,
    pub workspace_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub order_index: i32,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub children: Vec<CollectionTreeItem>,
    pub requests: Vec<RequestItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestHistoryItem {
    pub id: i64,
    pub request_id: String,
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

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn get_collections_db(
    db_path: &PathBuf,
    workspace_id: &str,
) -> Result<Vec<CollectionTreeItem>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Load all collections for this workspace
    let mut stmt = conn
        .prepare("SELECT id, workspace_id, parent_id, name, description, order_index, created_at_ms, updated_at_ms FROM collections WHERE workspace_id = ? ORDER BY order_index ASC, created_at_ms ASC")
        .map_err(|e| e.to_string())?;

    let all_cols: Vec<Collection> = stmt
        .query_map([workspace_id], |row| {
            Ok(Collection {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                parent_id: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                order_index: row.get(5)?,
                created_at_ms: row.get(6)?,
                updated_at_ms: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Load all requests for these collections
    let mut req_stmt = conn
        .prepare(
            "SELECT r.id, r.collection_id, r.name, r.method, r.url, r.headers_json, r.params_json, r.body_type, r.body_content, r.extract_rules_json, r.description, r.order_index, r.created_at_ms, r.updated_at_ms 
             FROM requests r
             JOIN collections c ON r.collection_id = c.id
             WHERE c.workspace_id = ?
             ORDER BY r.order_index ASC, r.created_at_ms ASC"
        )
        .map_err(|e| e.to_string())?;

    let all_reqs: Vec<RequestItem> = req_stmt
        .query_map([workspace_id], |row| {
            let headers_json: String = row.get(5)?;
            let params_json: String = row.get(6)?;
            let extract_rules_json: String = row.get(9)?;

            let headers: Vec<HeaderItem> = serde_json::from_str(&headers_json).unwrap_or_default();
            let params: Vec<ParamItem> = serde_json::from_str(&params_json).unwrap_or_default();
            let extract_rules: Vec<ExtractRuleItem> = serde_json::from_str(&extract_rules_json).unwrap_or_default();

            Ok(RequestItem {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                name: row.get(2)?,
                method: row.get(3)?,
                url: row.get(4)?,
                headers,
                params,
                body_type: row.get(7)?,
                body_content: row.get(8)?,
                extract_rules,
                description: row.get(10)?,
                order_index: row.get(11)?,
                created_at_ms: row.get(12)?,
                updated_at_ms: row.get(13)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Build hierarchical tree
    fn build_tree_level(parent_id: Option<&str>, cols: &[Collection], reqs: &[RequestItem]) -> Vec<CollectionTreeItem> {
        cols.iter()
            .filter(|c| c.parent_id.as_deref() == parent_id)
            .map(|c| {
                let children = build_tree_level(Some(&c.id), cols, reqs);
                let col_reqs: Vec<RequestItem> = reqs.iter().filter(|r| r.collection_id == c.id).cloned().collect();
                CollectionTreeItem {
                    id: c.id.clone(),
                    workspace_id: c.workspace_id.clone(),
                    parent_id: c.parent_id.clone(),
                    name: c.name.clone(),
                    description: c.description.clone(),
                    order_index: c.order_index,
                    created_at_ms: c.created_at_ms,
                    updated_at_ms: c.updated_at_ms,
                    children,
                    requests: col_reqs,
                }
            })
            .collect()
    }

    Ok(build_tree_level(None, &all_cols, &all_reqs))
}

pub fn create_collection_db(
    db_path: &PathBuf,
    workspace_id: String,
    parent_id: Option<String>,
    name: String,
) -> Result<Collection, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();
    let id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO collections (id, workspace_id, parent_id, name, description, order_index, created_at_ms, updated_at_ms)
         VALUES (?, ?, ?, ?, NULL, 0, ?, ?)",
        params![id, workspace_id, parent_id, name, ts, ts],
    )
    .map_err(|e| e.to_string())?;

    Ok(Collection {
        id,
        workspace_id,
        parent_id,
        name,
        description: None,
        order_index: 0,
        created_at_ms: ts,
        updated_at_ms: ts,
    })
}

pub fn update_collection_db(db_path: &PathBuf, col: Collection) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();

    conn.execute(
        "UPDATE collections SET name = ?, description = ?, parent_id = ?, order_index = ?, updated_at_ms = ? WHERE id = ?",
        params![col.name, col.description, col.parent_id, col.order_index, ts, col.id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn delete_collection_db(db_path: &PathBuf, id: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM collections WHERE id = ?", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn create_request_db(
    db_path: &PathBuf,
    collection_id: String,
    name: String,
) -> Result<RequestItem, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();
    let id = Uuid::new_v4().to_string();

    let default_headers = vec![HeaderItem {
        id: Uuid::new_v4().to_string(),
        key: "User-Agent".to_string(),
        value: "MITM-Developer-Studio/2.0".to_string(),
        enabled: true,
    }];
    let headers_json = serde_json::to_string(&default_headers).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO requests (id, collection_id, name, method, url, headers_json, params_json, body_type, body_content, extract_rules_json, description, order_index, created_at_ms, updated_at_ms)
         VALUES (?, ?, ?, 'GET', 'https://httpbin.org/get', ?, '[]', 'none', NULL, '[]', NULL, 0, ?, ?)",
        params![id, collection_id, name, headers_json, ts, ts],
    )
    .map_err(|e| e.to_string())?;

    Ok(RequestItem {
        id,
        collection_id,
        name,
        method: "GET".to_string(),
        url: "https://httpbin.org/get".to_string(),
        headers: default_headers,
        params: vec![],
        body_type: "none".to_string(),
        body_content: None,
        extract_rules: vec![],
        description: None,
        order_index: 0,
        created_at_ms: ts,
        updated_at_ms: ts,
    })
}

pub fn update_request_db(db_path: &PathBuf, req: RequestItem) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();

    let headers_json = serde_json::to_string(&req.headers).unwrap_or_else(|_| "[]".to_string());
    let params_json = serde_json::to_string(&req.params).unwrap_or_else(|_| "[]".to_string());
    let extract_rules_json = serde_json::to_string(&req.extract_rules).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "UPDATE requests SET name = ?, method = ?, url = ?, headers_json = ?, params_json = ?, body_type = ?, body_content = ?, extract_rules_json = ?, description = ?, order_index = ?, updated_at_ms = ? WHERE id = ?",
        params![
            req.name,
            req.method,
            req.url,
            headers_json,
            params_json,
            req.body_type,
            req.body_content,
            extract_rules_json,
            req.description,
            req.order_index,
            ts,
            req.id
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn delete_request_db(db_path: &PathBuf, id: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM requests WHERE id = ?", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}
