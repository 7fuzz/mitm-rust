use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use crate::proxy;
use uuid::Uuid;
use bytes::Bytes;
use http_body_util::{Full, BodyExt};
use hyper::Request;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepeaterGroup {
    pub id: String,
    pub name: String,
    pub order_index: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepeaterRequest {
    pub id: String,
    pub name: String,
    pub group_id: Option<String>,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub response: Option<proxy::Traffic>,
    pub hit_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRepeaterItem {
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub group_id: Option<String>,
    pub response: Option<proxy::Traffic>,
}

fn get_db_path(app_handle: &AppHandle) -> std::path::PathBuf {
    app_handle.path().app_data_dir().expect("Failed to get app data dir").join("mitm.db")
}

#[tauri::command]
pub async fn create_repeater_item(app_handle: AppHandle, item: CreateRepeaterItem) -> Result<String, String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let id = Uuid::new_v4().to_string();
    let headers_json = serde_json::to_string(&item.headers).unwrap_or_default();
    
    let (res_status, res_headers, res_body) = if let Some(res) = &item.response {
        (Some(res.status_code), Some(serde_json::to_string(&res.response_headers).unwrap_or_default()), Some(res.response_body.clone()))
    } else {
        (None, None, None)
    };

    conn.execute(
        "INSERT INTO repeater_requests (id, name, method, url, headers, body, response_status, response_headers, response_body, group_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![id, item.name, item.method, item.url, headers_json, item.body, res_status, res_headers, res_body, item.group_id],
    ).map_err(|e| e.to_string())?;

    Ok(id)
}

#[tauri::command]
pub async fn update_repeater_request(app_handle: AppHandle, id: String, updates: RepeaterRequest) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let headers_json = serde_json::to_string(&updates.headers).unwrap_or_default();
    
    conn.execute(
        "UPDATE repeater_requests SET name = ?, method = ?, url = ?, headers = ?, body = ?, group_id = ? WHERE id = ?",
        rusqlite::params![updates.name, updates.method, updates.url, headers_json, updates.body, updates.group_id, id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_repeater_request(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM repeater_requests WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn create_repeater_group(app_handle: AppHandle, name: String) -> Result<String, String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    conn.execute("INSERT INTO repeater_groups (id, name) VALUES (?, ?)", [id.clone(), name]).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn delete_repeater_group(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM repeater_groups WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn reorder_repeater_requests(app_handle: AppHandle, ids: Vec<String>) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    for (idx, id) in ids.iter().enumerate() {
        conn.execute("UPDATE repeater_requests SET order_index = ? WHERE id = ?", rusqlite::params![idx as i32, id]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn reorder_repeater_groups(app_handle: AppHandle, ids: Vec<String>) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    for (idx, id) in ids.iter().enumerate() {
        conn.execute("UPDATE repeater_groups SET order_index = ? WHERE id = ?", rusqlite::params![idx as i32, id]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn rename_repeater_group(app_handle: AppHandle, id: String, name: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("UPDATE repeater_groups SET name = ? WHERE id = ?", [name, id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn manage_group_assignment(
    app_handle: AppHandle,
    action: String,
    group_id: Option<String>,
    env_id: Option<String>,
    target_env_id: Option<String>,
    bulk_links: Option<Vec<(String, String)>>,
    bulk_unlinks: Option<Vec<(String, String)>>
) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app_handle);
    let mut conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;

    match action.as_str() {
        "link" => {
            if let (Some(gid), Some(eid)) = (group_id, env_id) {
                conn.execute("INSERT OR IGNORE INTO environment_groups (group_id, environment_id) VALUES (?, ?)", [gid, eid]).map_err(|e| e.to_string())?;
            }
        },
        "unlink" => {
            if let (Some(gid), Some(eid)) = (group_id, env_id) {
                conn.execute("DELETE FROM environment_groups WHERE group_id = ? AND environment_id = ?", [gid, eid]).map_err(|e| e.to_string())?;
            }
        },
        "move" => {
            if let (Some(gid), Some(eid), Some(teid)) = (group_id, env_id, target_env_id) {
                conn.execute("UPDATE environment_groups SET environment_id = ? WHERE group_id = ? AND environment_id = ?", [teid, gid, eid]).map_err(|e| e.to_string())?;
            }
        },
        "get_assignments" => {
            if let Some(gid) = group_id {
                let mut stmt = conn.prepare("SELECT environment_id FROM environment_groups WHERE group_id = ?").map_err(|e| e.to_string())?;
                let env_ids: Vec<String> = stmt.query_map([gid], |row| row.get(0)).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
                return Ok(serde_json::to_value(env_ids).unwrap());
            }
        },
        "get_groups_for_env" => {
            if let Some(eid) = env_id {
                let mut stmt = conn.prepare("SELECT group_id FROM environment_groups WHERE environment_id = ?").map_err(|e| e.to_string())?;
                let group_ids: Vec<String> = stmt.query_map([eid], |row| row.get(0)).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
                return Ok(serde_json::to_value(group_ids).unwrap());
            }
        },
        "bulk" => {
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            if let Some(links) = bulk_links {
                for (gid, eid) in links {
                    tx.execute("INSERT OR IGNORE INTO environment_groups (group_id, environment_id) VALUES (?, ?)", [gid, eid]).map_err(|e| e.to_string())?;
                }
            }
            if let Some(unlinks) = bulk_unlinks {
                for (gid, eid) in unlinks {
                    tx.execute("DELETE FROM environment_groups WHERE group_id = ? AND environment_id = ?", [gid, eid]).map_err(|e| e.to_string())?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
        },
        _ => return Err("Invalid action".to_string()),
    }

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub async fn execute_repeater_request(app_handle: AppHandle, id: String) -> Result<proxy::Traffic, String> {
    let db_path = get_db_path(&app_handle);
    
    let (method, url, headers, body) = {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT method, url, headers, body FROM repeater_requests WHERE id = ?").map_err(|e| e.to_string())?;
        stmt.query_row([id.clone()], |row| {
            let headers_str: String = row.get(2)?;
            let headers: Vec<(String, String)> = serde_json::from_str(&headers_str).unwrap_or_default();
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, headers, row.get::<_, String>(3)?))
        }).map_err(|e| e.to_string())?
    };

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_or_http()
        .enable_http1()
        .build();
    let client = Client::builder(TokioExecutor::new()).build(https);

    let mut builder = Request::builder()
        .method(method.as_str())
        .uri(&url);

    for (k, v) in headers.iter() {
        builder = builder.header(k, v);
    }
    
    let req = builder.body(Full::new(Bytes::from(body.clone()))).map_err(|e| e.to_string())?;
    let res = client.request(req).await.map_err(|e| e.to_string())?;

    let status = res.status().as_u16();
    let mut res_headers = Vec::new();
    for (name, value) in res.headers().iter() {
        res_headers.push((name.to_string(), value.to_str().unwrap_or("").to_string()));
    }
    
    let collected_body = res.into_body().collect().await.map_err(|e| e.to_string())?.to_bytes();
    let res_body_str = String::from_utf8_lossy(&collected_body).to_string();

    let traffic = proxy::Traffic {
        id: format!("{}_res", id),
        method: method.clone(),
        url: url.clone(),
        host: String::new(),
        status_code: status,
        request_headers: headers.clone(),
        response_headers: res_headers.clone(),
        request_body: body.clone(),
        response_body: res_body_str.clone(),
        phase: "response".to_string(),
        is_intercepted: false,
        intercepted_at: None,
    };

    let res_headers_json = serde_json::to_string(&res_headers).unwrap_or_default();
    let req_headers_json = serde_json::to_string(&headers).unwrap_or_default();
    {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE repeater_requests SET response_status = ?, response_headers = ?, response_body = ?, hit_count = hit_count + 1 WHERE id = ?",
            rusqlite::params![status, res_headers_json, res_body_str, id],
        ).map_err(|e| e.to_string())?;

        // Add to history
        let history_id = Uuid::new_v4().to_string();
        let _ = conn.execute(
            "INSERT INTO repeater_history (id, repeater_id, method, url, request, response, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                history_id, id, method, url, 
                serde_json::json!({ "headers": req_headers_json, "body": body }).to_string(),
                serde_json::json!({ "status": status, "headers": res_headers_json, "body": res_body_str }).to_string(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
            ],
        );
    }

    Ok(traffic)
}

#[tauri::command]
pub async fn get_repeater_history(app_handle: AppHandle, repeater_id: String) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, method, url, request, response, timestamp FROM repeater_history WHERE repeater_id = ? ORDER BY timestamp DESC").map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([repeater_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, String>(0)?,
            "method": row.get::<_, String>(1)?,
            "url": row.get::<_, String>(2)?,
            "request": row.get::<_, String>(3)?,
            "response": row.get::<_, String>(4)?,
            "timestamp": row.get::<_, i64>(5)?
        }))
    }).map_err(|e| e.to_string())?;

    let mut history = Vec::new();
    for row in rows {
        if let Ok(h) = row { history.push(h); }
    }
    Ok(serde_json::to_value(history).unwrap())
}

#[tauri::command]
pub async fn clear_repeater_history(app_handle: AppHandle, repeater_id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM repeater_history WHERE repeater_id = ?", [repeater_id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_repeater_history_item(app_handle: AppHandle, id: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM repeater_history WHERE id = ?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}
