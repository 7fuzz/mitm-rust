use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::encoding::{build_multipart_payload, build_urlencoded_payload, format_body_for_ui};
use crate::repeater::HeaderItem;
use crate::workspace::interpolate_variables;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    pub history_id: i64,
    pub request_id: String,
    pub status_code: u16,
    pub status_text: String,
    pub response_headers: Vec<HeaderItem>,
    pub response_body: String,
    pub duration_ms: u64,
    pub response_size: u64,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn url_encode_str(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

pub async fn execute_collection_request_db(
    db_path: &PathBuf,
    request_id: &str,
) -> Result<ExecutionResult, String> {
    // 1. Fetch request details & interpolate variables in scope block so Connection is dropped before .await
    let (id, _workspace_id, raw_method, target_url, logged_req_headers, final_body, req_headers_map, body_type) = {
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT r.id, r.collection_id, c.workspace_id, r.name, r.method, r.url, r.headers_json, r.params_json, r.body_type, r.body_content
                 FROM requests r
                 JOIN collections c ON r.collection_id = c.id
                 WHERE r.id = ?"
            )
            .map_err(|e| e.to_string())?;

        let (id, _col_id, workspace_id, _name, raw_method, raw_url, headers_json, params_json, body_type, raw_body): 
            (String, String, String, String, String, String, String, String, String, Option<String>) = 
            stmt.query_row([request_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            }).map_err(|e| format!("Request with ID {} not found: {}", request_id, e))?;

        let interpolated_url = interpolate_variables(db_path, &workspace_id, &raw_url);
        let interpolated_body = raw_body.map(|b| interpolate_variables(db_path, &workspace_id, &b));

        let headers_list: Vec<HeaderItem> = serde_json::from_str(&headers_json).unwrap_or_default();
        let params_list: Vec<crate::repeater::ParamItem> = serde_json::from_str(&params_json).unwrap_or_default();

        let mut url = interpolated_url;
        if !params_list.is_empty() {
            let active_params: Vec<String> = params_list
                .into_iter()
                .filter(|p| p.enabled && !p.key.trim().is_empty())
                .map(|p| {
                    format!(
                        "{}={}",
                        url_encode_str(&interpolate_variables(db_path, &workspace_id, &p.key)),
                        url_encode_str(&interpolate_variables(db_path, &workspace_id, &p.value))
                    )
                })
                .collect();
            if !active_params.is_empty() {
                let joiner = if url.contains('?') { "&" } else { "?" };
                url = format!("{}{}{}", url, joiner, active_params.join("&"));
            }
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            url = format!("https://{}", url);
        }

        let is_multipart = body_type == "multipart" || body_type == "form-data" || body_type == "form";
        let is_urlencoded = body_type == "urlencoded" || body_type == "x-www-form-urlencoded";

        let mut req_headers_map = HeaderMap::new();
        let mut logged_req_headers = Vec::new();

        for h in headers_list {
            if h.enabled && !h.key.trim().is_empty() {
                let k_lower = h.key.trim().to_lowercase();
                if k_lower == "host" || k_lower == "content-length" || k_lower == "transfer-encoding"
                    || (is_multipart && k_lower == "content-type")
                    || (is_urlencoded && k_lower == "content-type") {
                    continue;
                }

                let interpolated_k = interpolate_variables(db_path, &workspace_id, &h.key);
                let interpolated_v = interpolate_variables(db_path, &workspace_id, &h.value);

                if let (Ok(hn), Ok(hv)) = (
                    HeaderName::from_bytes(interpolated_k.as_bytes()),
                    HeaderValue::from_str(&interpolated_v),
                ) {
                    req_headers_map.insert(hn, hv);
                    logged_req_headers.push(HeaderItem {
                        id: uuid::Uuid::new_v4().to_string(),
                        key: interpolated_k,
                        value: interpolated_v,
                        enabled: true,
                    });
                }
            }
        }

        let final_body = if body_type != "none" { interpolated_body } else { None };

        (id, workspace_id, raw_method, url, logged_req_headers, final_body, req_headers_map, body_type)
    };

    // 2. Construct reqwest HTTP client & send request
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let method = Method::from_bytes(raw_method.as_bytes()).map_err(|e| e.to_string())?;
    let mut req_builder = client.request(method, &target_url).headers(req_headers_map);

    let is_multipart = body_type == "multipart" || body_type == "form-data" || body_type == "form";
    let is_urlencoded = body_type == "urlencoded" || body_type == "x-www-form-urlencoded";

    if is_urlencoded {
        if let Some(ref content) = final_body {
            if let Ok(params) = build_urlencoded_payload(content) {
                req_builder = req_builder.form(&params);
            }
        }
    } else if is_multipart {
        if let Some(ref content) = final_body {
            if let Ok(form) = build_multipart_payload(content) {
                req_builder = req_builder.multipart(form);
            }
        }
    } else if let Some(ref body_str) = final_body {
        if !body_str.is_empty() {
            req_builder = req_builder.body(body_str.clone());
        }
    }

    let start_time = Instant::now();
    let response = req_builder.send().await.map_err(|e| format!("Request execution failed: {}", e))?;
    let duration_ms = start_time.elapsed().as_millis() as u64;

    let status_code = response.status().as_u16();
    let status_text = response.status().canonical_reason().unwrap_or("Custom Status").to_string();

    let mut response_headers = Vec::new();
    let mut content_type = "text/plain".to_string();
    let mut content_encoding: Option<String> = None;

    for (k, v) in response.headers() {
        let key_str = k.as_str().to_string();
        let val_str = v.to_str().unwrap_or("").to_string();
        if key_str.to_lowercase() == "content-type" {
            content_type = val_str.clone();
        }
        if key_str.to_lowercase() == "content-encoding" {
            content_encoding = Some(val_str.clone());
        }
        response_headers.push(HeaderItem {
            id: uuid::Uuid::new_v4().to_string(),
            key: key_str,
            value: val_str,
            enabled: true,
        });
    }

    let raw_bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let response_size = raw_bytes.len() as u64;
    let decoded_body = format_body_for_ui(&raw_bytes, &content_type, content_encoding.as_deref().unwrap_or(""));

    let executed_at = now_ms();

    // 3. Store history log in separate DB connection
    let history_id = {
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        let req_headers_json = serde_json::to_string(&logged_req_headers).unwrap_or_else(|_| "[]".to_string());
        let res_headers_json = serde_json::to_string(&response_headers).unwrap_or_else(|_| "[]".to_string());

        let mut insert_stmt = conn
            .prepare(
                "INSERT INTO request_histories 
                    (request_id, method, url, request_headers_json, request_body, status_code, response_headers_json, response_body, duration_ms, executed_at_ms)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .map_err(|e| e.to_string())?;

        insert_stmt.execute(params![
            id,
            raw_method,
            target_url,
            req_headers_json,
            final_body,
            status_code,
            res_headers_json,
            decoded_body,
            duration_ms as i64,
            executed_at,
        ]).map_err(|e| e.to_string())?;

        conn.last_insert_rowid()
    };

    Ok(ExecutionResult {
        history_id,
        request_id: id,
        status_code,
        status_text,
        response_headers,
        response_body: decoded_body,
        duration_ms,
        response_size,
    })
}
