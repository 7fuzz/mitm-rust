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
    pub body_json: Option<String>,
    pub body_raw: Option<String>,
    pub body_form_data: Option<String>,
    pub body_urlencoded: Option<String>,
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
    pub status_text: Option<String>,
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

fn ensure_schema_columns(conn: &Connection) {
    let _ = conn.execute("ALTER TABLE requests ADD COLUMN body_json TEXT", []);
    let _ = conn.execute("ALTER TABLE requests ADD COLUMN body_raw TEXT", []);
    let _ = conn.execute("ALTER TABLE requests ADD COLUMN body_form_data TEXT", []);
    let _ = conn.execute("ALTER TABLE requests ADD COLUMN body_urlencoded TEXT", []);
}

pub fn get_collections_db(
    db_path: &PathBuf,
    workspace_id: &str,
) -> Result<Vec<CollectionTreeItem>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    ensure_schema_columns(&conn);

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
            "SELECT r.id, r.collection_id, r.name, r.method, r.url, r.headers_json, r.params_json, r.body_type, r.body_content, r.body_json, r.body_raw, r.body_form_data, r.body_urlencoded, r.extract_rules_json, r.description, r.order_index, r.created_at_ms, r.updated_at_ms 
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
            let extract_rules_json: String = row.get(13)?;

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
                body_json: row.get(9)?,
                body_raw: row.get(10)?,
                body_form_data: row.get(11)?,
                body_urlencoded: row.get(12)?,
                extract_rules,
                description: row.get(14)?,
                order_index: row.get(15)?,
                created_at_ms: row.get(16)?,
                updated_at_ms: row.get(17)?,
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
        body_json: None,
        body_raw: None,
        body_form_data: None,
        body_urlencoded: None,
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

pub fn get_request_histories_db(
    db_path: &PathBuf,
    request_id: &str,
) -> Result<Vec<RequestHistoryItem>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, request_id, method, url, request_headers_json, request_body, status_code, response_headers_json, response_body, duration_ms, executed_at_ms
             FROM request_histories
             WHERE request_id = ?
             ORDER BY executed_at_ms DESC
             LIMIT 50"
        )
        .map_err(|e| e.to_string())?;

    let histories = stmt
        .query_map([request_id], |row| {
            let req_headers_json: String = row.get(4)?;
            let res_headers_json: String = row.get(7)?;

            let req_headers: Vec<HeaderItem> = serde_json::from_str(&req_headers_json).unwrap_or_default();
            let res_headers: Vec<HeaderItem> = serde_json::from_str(&res_headers_json).unwrap_or_default();
            let duration_ms: i64 = row.get(9)?;

            Ok(RequestHistoryItem {
                id: row.get(0)?,
                request_id: row.get(1)?,
                method: row.get(2)?,
                url: row.get(3)?,
                request_headers: req_headers,
                request_body: row.get(5)?,
                status_code: row.get(6)?,
                status_text: None,
                response_headers: res_headers,
                response_body: row.get(8)?,
                duration_ms: duration_ms as u64,
                executed_at_ms: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(histories)
}

pub fn clear_request_histories_db(db_path: &PathBuf, request_id: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM request_histories WHERE request_id = ?", params![request_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn move_request_db(
    db_path: &PathBuf,
    request_id: &str,
    target_collection_id: &str,
) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();
    conn.execute(
        "UPDATE requests SET collection_id = ?, updated_at_ms = ? WHERE id = ?",
        params![target_collection_id, ts, request_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn move_collection_db(
    db_path: &PathBuf,
    collection_id: &str,
    target_parent_id: Option<String>,
) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();
    conn.execute(
        "UPDATE collections SET parent_id = ?, updated_at_ms = ? WHERE id = ?",
        params![target_parent_id, ts, collection_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn duplicate_request_db(db_path: &PathBuf, request_id: &str) -> Result<RequestItem, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();

    let mut req: RequestItem = conn.query_row(
        "SELECT id, collection_id, name, method, url, headers_json, params_json, body_type, body_content, body_json, body_raw, body_form_data, body_urlencoded, extract_rules_json, description, order_index
         FROM requests WHERE id = ?",
        params![request_id],
        |row| {
            let headers_json: String = row.get(5)?;
            let params_json: String = row.get(6)?;
            let extract_rules_json: String = row.get(13)?;

            let headers: Vec<HeaderItem> = serde_json::from_str(&headers_json).unwrap_or_default();
            let params: Vec<ParamItem> = serde_json::from_str(&params_json).unwrap_or_default();
            let extract_rules: Vec<ExtractRuleItem> = serde_json::from_str(&extract_rules_json).unwrap_or_default();

            Ok(RequestItem {
                id: Uuid::new_v4().to_string(),
                collection_id: row.get(1)?,
                name: format!("{} (Copy)", row.get::<_, String>(2)?),
                method: row.get(3)?,
                url: row.get(4)?,
                headers,
                params,
                body_type: row.get(7)?,
                body_content: row.get(8)?,
                body_json: row.get(9)?,
                body_raw: row.get(10)?,
                body_form_data: row.get(11)?,
                body_urlencoded: row.get(12)?,
                extract_rules,
                description: row.get(14)?,
                order_index: row.get::<_, i32>(15)? + 1,
                created_at_ms: ts,
                updated_at_ms: ts,
            })
        },
    ).map_err(|e| e.to_string())?;

    let headers_json = serde_json::to_string(&req.headers).unwrap_or_else(|_| "[]".to_string());
    let params_json = serde_json::to_string(&req.params).unwrap_or_else(|_| "[]".to_string());
    let extract_rules_json = serde_json::to_string(&req.extract_rules).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO requests (id, collection_id, name, method, url, headers_json, params_json, body_type, body_content, body_json, body_raw, body_form_data, body_urlencoded, extract_rules_json, description, order_index, created_at_ms, updated_at_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            req.id,
            req.collection_id,
            req.name,
            req.method,
            req.url,
            headers_json,
            params_json,
            req.body_type,
            req.body_content,
            req.body_json,
            req.body_raw,
            req.body_form_data,
            req.body_urlencoded,
            extract_rules_json,
            req.description,
            req.order_index,
            ts,
            ts
        ],
    ).map_err(|e| e.to_string())?;

    Ok(req)
}

pub fn duplicate_collection_db(db_path: &PathBuf, collection_id: &str) -> Result<String, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let ts = now_ms();

    let (ws_id, parent_id, name, desc, order_idx): (String, Option<String>, String, Option<String>, i32) = conn.query_row(
        "SELECT workspace_id, parent_id, name, description, order_index FROM collections WHERE id = ?",
        params![collection_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).map_err(|e| e.to_string())?;

    let new_col_id = Uuid::new_v4().to_string();
    let dup_name = format!("{} (Copy)", name);

    conn.execute(
        "INSERT INTO collections (id, workspace_id, parent_id, name, description, order_index, created_at_ms, updated_at_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![new_col_id, ws_id, parent_id, dup_name, desc, order_idx + 1, ts, ts],
    ).map_err(|e| e.to_string())?;

    let mut req_stmt = conn.prepare("SELECT name, method, url, headers_json, params_json, body_type, body_content, body_json, body_raw, body_form_data, body_urlencoded, extract_rules_json, description, order_index FROM requests WHERE collection_id = ?").map_err(|e| e.to_string())?;
    let reqs: Vec<(String, String, String, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, String, Option<String>, i32)> = req_stmt.query_map([collection_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?, row.get(11)?, row.get(12)?, row.get(13)?))
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    for (r_name, r_method, r_url, r_headers, r_params, r_body_type, r_body_content, r_b_json, r_b_raw, r_b_form, r_b_url, r_extract, r_desc, r_order) in reqs {
        let new_req_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO requests (id, collection_id, name, method, url, headers_json, params_json, body_type, body_content, body_json, body_raw, body_form_data, body_urlencoded, extract_rules_json, description, order_index, created_at_ms, updated_at_ms)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![new_req_id, new_col_id, r_name, r_method, r_url, r_headers, r_params, r_body_type, r_body_content, r_b_json, r_b_raw, r_b_form, r_b_url, r_extract, r_desc, r_order, ts, ts],
        ).map_err(|e| e.to_string())?;
    }

    Ok(new_col_id)
}
