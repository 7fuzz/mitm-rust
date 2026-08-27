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
    let (id, workspace_id, raw_method, target_url, logged_req_headers, final_body, req_headers_map, body_type, extract_rules_json) = {
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT r.id, r.collection_id, c.workspace_id, r.name, r.method, r.url, r.headers_json, r.params_json, r.body_type, r.body_json, r.body_raw, r.body_form_data, r.body_urlencoded, r.extract_rules_json
                 FROM requests r
                 JOIN collections c ON r.collection_id = c.id
                 WHERE r.id = ?"
            )
            .map_err(|e| e.to_string())?;

        let (id, _col_id, workspace_id, _name, raw_method, raw_url, headers_json, params_json, body_type, b_json, b_raw, b_form, b_url, raw_extract_rules): 
            (String, String, String, String, String, String, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) = 
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
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                ))
            }).map_err(|e| format!("Request with ID {} not found: {}", request_id, e))?;

        let raw_body = match body_type.as_str() {
            "json" => b_json,
            "raw" => b_raw,
            "form-data" | "multipart" => b_form,
            "urlencoded" | "x-www-form-urlencoded" => b_url,
            _ => None,
        };

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
        let extract_rules_json = raw_extract_rules.unwrap_or_else(|| "[]".to_string());

        (id, workspace_id, raw_method, url, logged_req_headers, final_body, req_headers_map, body_type, extract_rules_json)
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

    // 3. Perform auto-extraction if extract_rules_json is present
    perform_auto_extraction_db(db_path, &workspace_id, &extract_rules_json, &decoded_body, &response_headers);

    // 4. Store history log in separate DB connection
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

fn evaluate_rust_extract_rule(
    rule: &crate::repeater::ExtractRuleItem,
    response_body: &str,
    response_headers: &[HeaderItem],
) -> Option<String> {
    let mode = rule.r#type.as_str();

    match mode {
        "json" => {
            let parsed: serde_json::Value = serde_json::from_str(response_body).ok()?;
            let parts: Vec<&str> = rule.expression.trim().split('.').collect();
            let mut current = &parsed;
            for part in parts {
                current = current.get(part)?;
            }
            if current.is_string() {
                Some(current.as_str().unwrap_or("").to_string())
            } else if !current.is_null() {
                Some(current.to_string())
            } else {
                None
            }
        }
        "header" => {
            let target_k = rule.expression.trim().to_lowercase();
            response_headers.iter().find(|h| h.key.trim().to_lowercase() == target_k).map(|h| h.value.clone())
        }
        "after_string" => {
            let parts: Vec<&str> = rule.expression.split("||").collect();
            let prefix = parts.get(0)?;
            if prefix.is_empty() { return None; }
            let limit: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(256);
            let idx = response_body.find(prefix)?;
            let start = idx + prefix.len();
            let mut sub = &response_body[start..std::cmp::min(start + limit, response_body.len())];
            if let Some(nl) = sub.find(['\r', '\n']) {
                sub = &sub[..nl];
            }
            Some(sub.to_string())
        }
        "before_string" => {
            let parts: Vec<&str> = rule.expression.split("||").collect();
            let suffix = parts.get(0)?;
            if suffix.is_empty() { return None; }
            let limit: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(256);
            let idx = response_body.find(suffix)?;
            let start = if idx > limit { idx - limit } else { 0 };
            let sub = &response_body[start..idx];
            let line = sub.lines().last().unwrap_or(sub);
            Some(line.to_string())
        }
        "between_string" => {
            let parts: Vec<&str> = rule.expression.split("||").collect();
            let start_delim = parts.get(0)?;
            let end_delim = parts.get(1)?;
            if start_delim.is_empty() || end_delim.is_empty() { return None; }
            let start_idx = response_body.find(start_delim)?;
            let content_start = start_idx + start_delim.len();
            let end_idx = response_body[content_start..].find(end_delim)?;
            Some(response_body[content_start..content_start + end_idx].to_string())
        }
        "body_regex" | "regex" => {
            let re = regex::Regex::new(&rule.expression).ok()?;
            let caps = re.captures(response_body)?;
            let val = caps.get(1).or_else(|| caps.get(0))?;
            Some(val.as_str().to_string())
        }
        _ => None,
    }
}

pub fn perform_auto_extraction_db(
    db_path: &PathBuf,
    workspace_id: &str,
    extract_rules_json: &str,
    response_body: &str,
    response_headers: &[HeaderItem],
) {
    if extract_rules_json.trim().is_empty() || extract_rules_json == "[]" {
        return;
    }

    let rules: Vec<crate::repeater::ExtractRuleItem> = match serde_json::from_str(extract_rules_json) {
        Ok(r) => r,
        Err(_) => return,
    };

    if rules.is_empty() {
        return;
    }

    let mut envs = match crate::workspace::get_workspace_environments_db(db_path, workspace_id) {
        Ok(e) => e,
        Err(_) => return,
    };

    let active_env = match envs.iter_mut().find(|e| e.is_active) {
        Some(e) => e,
        None => return,
    };

    let mut modified = false;

    for rule in rules {
        if !rule.enabled || rule.target_variable.trim().is_empty() || rule.expression.trim().is_empty() {
            continue;
        }

        let target_var_name = rule.target_variable.trim();
        let extracted_val = evaluate_rust_extract_rule(&rule, response_body, response_headers);

        if let Some(val_str) = extracted_val {
            if val_str.trim().is_empty() {
                continue;
            }

            let var_entry = active_env.variables.iter_mut().find(|v| v.key.eq_ignore_ascii_case(target_var_name));

            match var_entry {
                Some(v) => {
                    if v.variants.is_empty() || v.variants[0].name != "(auto)" {
                        let auto_var = crate::workspace::VariableVariant {
                            name: "(auto)".to_string(),
                            value: val_str.clone(),
                        };
                        v.variants.insert(0, auto_var);
                    } else {
                        v.variants[0].value = val_str.clone();
                    }

                    if v.active_index == 0 {
                        v.value = val_str;
                    }
                    modified = true;
                }
                None => {
                    let auto_var = crate::workspace::VariableVariant {
                        name: "(auto)".to_string(),
                        value: val_str.clone(),
                    };
                    active_env.variables.push(crate::workspace::EnvironmentVariable {
                        key: target_var_name.to_string(),
                        value: val_str,
                        enabled: true,
                        r#type: "default".to_string(),
                        active_index: 0,
                        variants: vec![auto_var],
                    });
                    modified = true;
                }
            }
        }
    }

    if modified {
        let _ = crate::workspace::save_workspace_environment_db(db_path, active_env.clone());
    }
}
