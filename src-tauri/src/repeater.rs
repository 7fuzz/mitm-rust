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
pub struct RepeaterGroup {
    pub id: String,
    pub name: String,
    pub order_index: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Deserialize)]
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
        method, url, host: String::new(), status_code: status,
        request_headers: headers, response_headers: res_headers.clone(),
        request_body: body, response_body: res_body_str.clone(),
        phase: "response".to_string(), is_intercepted: false, intercepted_at: None,
    };

    let res_headers_json = serde_json::to_string(&res_headers).unwrap_or_default();
    {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE repeater_requests SET response_status = ?, response_headers = ?, response_body = ?, hit_count = hit_count + 1 WHERE id = ?",
            rusqlite::params![status, res_headers_json, res_body_str, id],
        ).map_err(|e| e.to_string())?;
    }

    Ok(traffic)
}
