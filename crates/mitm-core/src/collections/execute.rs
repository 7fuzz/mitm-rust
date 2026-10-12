use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::cookie::Jar;
use reqwest::Method;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::http::body::{build_multipart_payload, build_urlencoded_payload, format_body_for_ui};
use crate::http::steps::{jar_cookie, pre_request_failure, request_steps};
use crate::repeater::HeaderItem;
use crate::workspace::{active_environment_db, interpolate, CredentialScope, Interpolation};

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
    let jar = Arc::new(Jar::default());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .cookie_provider(jar.clone())
        .build()
        .map_err(|e| e.to_string())?;

    let steps = request_steps(db_path, request_id)?;
    for pre_id in &steps.pre {
        if let Err(error) = send_collection_request(db_path, pre_id, &client, &jar).await {
            if error == crate::credentials::LOCKED {
                return Err(error);
            }
            return record_unsent_run(db_path, request_id, pre_request_failure(db_path, pre_id, &error));
        }
    }

    let result = send_collection_request(db_path, request_id, &client, &jar).await?;

    // The main response is what the user asked for; a failing post-request only shows in its own history
    for post_id in &steps.post {
        let _ = send_collection_request(db_path, post_id, &client, &jar).await;
    }
    Ok(result)
}

/// Logs a run that never reached the server, so the failure shows up like any other response.
fn record_unsent_run(db_path: &PathBuf, request_id: &str, error: String) -> Result<ExecutionResult, String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let (method, url): (String, String) = conn
        .query_row("SELECT method, url FROM requests WHERE id = ?", [request_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO request_histories
            (request_id, method, url, request_headers_json, request_body, status_code, response_headers_json, response_body, duration_ms, executed_at_ms)
         VALUES (?, ?, ?, '[]', NULL, 0, '[]', ?, 0, ?)",
        params![request_id, method, url, error, now_ms()],
    )
    .map_err(|e| e.to_string())?;

    Ok(ExecutionResult {
        history_id: conn.last_insert_rowid(),
        request_id: request_id.to_string(),
        status_code: 0,
        status_text: "ERR_FAILED".to_string(),
        response_headers: vec![],
        response_size: error.len() as u64,
        response_body: error,
        duration_ms: 0,
    })
}

async fn send_collection_request(
    db_path: &PathBuf,
    request_id: &str,
    client: &reqwest::Client,
    jar: &Jar,
) -> Result<ExecutionResult, String> {
    // Scoped so the Connection is dropped before the .await below.
    let (id, workspace_id, raw_method, target_url, mut logged_req_headers, final_body, mut req_headers_map, body_type, extract_rules_json, record, extract_needs_credentials) = {
        let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT r.id, r.collection_id, c.workspace_id, r.name, r.method, r.url, r.headers_json, r.params_json, r.body_type, r.body_json, r.body_raw, r.body_form_data, r.body_urlencoded, r.extract_rules_json
                 FROM requests r
                 JOIN collections c ON r.collection_id = c.id
                 WHERE r.id = ?"
            )
            .map_err(|e| e.to_string())?;

        #[allow(clippy::type_complexity)]
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

        let env = active_environment_db(db_path, &workspace_id);
        let scope = CredentialScope { db_path, workspace_id: &workspace_id };
        let mut record = Interpolation::default();
        let mut fill = |text: &str| interpolate(env.as_ref(), Some(scope), text, &mut record);

        let interpolated_url = fill(&raw_url);
        let interpolated_body = raw_body.map(|b| fill(&b));

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
                        url_encode_str(&fill(&p.key)),
                        url_encode_str(&fill(&p.value))
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

                let interpolated_k = fill(&h.key);
                let interpolated_v = fill(&h.value);

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

        let rules: Vec<crate::repeater::ExtractRuleItem> = serde_json::from_str(&extract_rules_json).unwrap_or_default();
        let extract_needs_credentials = extract_targets_credentials(env.as_ref(), &rules);

        (id, workspace_id, raw_method, url, logged_req_headers, final_body, req_headers_map, body_type, extract_rules_json, record, extract_needs_credentials)
    };

    // Sending with a ref left unresolved, or dropping an extracted token, would both be silent failures.
    if record.locked || (extract_needs_credentials && crate::credentials::is_locked(db_path)) {
        return Err(crate::credentials::LOCKED.to_string());
    }

    if !req_headers_map.contains_key(reqwest::header::COOKIE) {
        if let Some(cookie) = jar_cookie(jar, &target_url) {
            if let Ok(hv) = HeaderValue::from_str(&cookie) {
                req_headers_map.insert(reqwest::header::COOKIE, hv);
                logged_req_headers.push(HeaderItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    key: "Cookie".to_string(),
                    value: cookie,
                    enabled: true,
                });
            }
        }
    }

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

    perform_auto_extraction_db(db_path, &workspace_id, Some(&workspace_id), &extract_rules_json, &decoded_body, &response_headers);

    let history_id = {
        let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
        for header in &mut logged_req_headers {
            header.key = record.mask(&header.key);
            header.value = record.mask(&header.value);
        }
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
            record.mask(&target_url),
            req_headers_json,
            final_body.as_deref().map(|b| record.mask(b)),
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

fn evaluate_json_path(value: &serde_json::Value, expr: &str) -> Option<serde_json::Value> {
    let clean_expr = expr.trim();
    let path = clean_expr
        .strip_prefix("$.")
        .or_else(|| clean_expr.strip_prefix('$'))
        .unwrap_or(clean_expr);

    if path.is_empty() {
        return Some(value.clone());
    }

    // Tokenize path into segments (supporting 'a.b.c', 'a[0].b', 'a.0.b', 'a["b"]')
    let mut normalized = String::new();
    for ch in path.chars() {
        match ch {
            '[' => {
                if !normalized.is_empty() && !normalized.ends_with('.') {
                    normalized.push('.');
                }
            }
            ']' => {}
            '\'' | '"' => {}
            _ => {
                normalized.push(ch);
            }
        }
    }

    let segments: Vec<&str> = normalized
        .split('.')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut current = value;

    for seg in segments {
        match current {
            serde_json::Value::Object(map) => {
                if let Some(v) = map.get(seg) {
                    current = v;
                } else if let Some((_, v)) = map.iter().find(|(k, _)| k.eq_ignore_ascii_case(seg)) {
                    current = v;
                } else if let Some((_, v)) = map.iter().find(|(k, _)| {
                    k.replace('_', "").eq_ignore_ascii_case(&seg.replace('_', ""))
                }) {
                    current = v;
                } else {
                    return None;
                }
            }
            serde_json::Value::Array(arr) => {
                if let Ok(idx) = seg.parse::<usize>() {
                    current = arr.get(idx)?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(current.clone())
}

fn json_value_to_string(val: &serde_json::Value) -> Option<String> {
    match val {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => Some(val.to_string()),
        serde_json::Value::Null => None,
    }
}

pub fn evaluate_rust_extract_rule(
    rule: &crate::repeater::ExtractRuleItem,
    response_body: &str,
    response_headers: &[HeaderItem],
) -> Option<String> {
    let mode = rule.r#type.trim().to_lowercase();

    match mode.as_str() {
        "json" | "jsonpath" | "json_path" | "body_json" => {
            let parsed: serde_json::Value = serde_json::from_str(response_body).ok()?;
            let val = evaluate_json_path(&parsed, &rule.expression)?;
            json_value_to_string(&val)
        }
        "header" | "headers" => {
            let target_k = rule.expression.trim().to_lowercase();
            response_headers
                .iter()
                .find(|h| h.key.trim().to_lowercase() == target_k)
                .map(|h| h.value.clone())
        }
        "after_string" => {
            let parts: Vec<&str> = rule.expression.split("||").collect();
            let prefix = parts.first()?;
            if prefix.is_empty() {
                return None;
            }
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
            let suffix = parts.first()?;
            if suffix.is_empty() {
                return None;
            }
            let limit: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(256);
            let idx = response_body.find(suffix)?;
            let start = idx.saturating_sub(limit);
            let sub = &response_body[start..idx];
            let line = sub.lines().last().unwrap_or(sub);
            Some(line.to_string())
        }
        "between_string" => {
            let parts: Vec<&str> = rule.expression.split("||").collect();
            let start_delim = parts.first()?;
            let end_delim = parts.get(1)?;
            if start_delim.is_empty() || end_delim.is_empty() {
                return None;
            }
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
        _ => {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(response_body) {
                if let Some(val) = evaluate_json_path(&parsed, &rule.expression) {
                    return json_value_to_string(&val);
                }
            }
            None
        }
    }
}

/// Whether a run's extract rules write into credentials, directly or through a linked `(auto)` variant.
pub(crate) fn extract_targets_credentials(env: Option<&crate::workspace::Environment>, rules: &[crate::repeater::ExtractRuleItem]) -> bool {
    rules.iter().filter(|r| r.enabled).any(|rule| {
        let target = rule.target_variable.trim().trim_start_matches("{{").trim_end_matches("}}").trim();
        if crate::credentials::parse_ref(target).is_some() {
            return true;
        }
        env.and_then(|env| env.variables.iter().find(|v| v.key.eq_ignore_ascii_case(target)))
            .and_then(|var| var.variants.iter().find(|v| v.name.eq_ignore_ascii_case("(auto)")))
            .is_some_and(|auto| crate::credentials::is_link(&auto.value))
    })
}

/// Writes extracted values into `(auto)` variants of the env workspace's active environment. A target that is a
/// `$cred:entry.field` ref, or a variable whose `(auto)` variant links to one, is written into that credential
/// (looked up in `credentials_workspace_id`) instead, so the variant keeps its link.
pub fn perform_auto_extraction_db(
    db_path: &PathBuf,
    workspace_id: &str,
    credentials_workspace_id: Option<&str>,
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

    let mut envs = crate::workspace::get_workspace_environments_db(db_path, workspace_id).unwrap_or_default();
    let mut active_env = envs.iter_mut().find(|e| e.is_active);
    let mut modified = false;
    let write_credential = |entry: &str, field: &str, value: &str| {
        let Some(credentials_workspace_id) = credentials_workspace_id else { return };
        if let Err(e) = crate::credentials::write_extracted(db_path, credentials_workspace_id, entry, field, value) {
            eprintln!("[extract] skipped $cred:{entry}.{field}: {e}");
        }
    };

    for rule in rules {
        if !rule.enabled || rule.target_variable.trim().is_empty() || rule.expression.trim().is_empty() {
            continue;
        }

        // Clean target variable name (e.g. strip "{{access_token}}" -> "access_token")
        let raw_target = rule.target_variable.trim();
        let target_var_name = raw_target
            .trim_start_matches("{{")
            .trim_end_matches("}}")
            .trim();

        if target_var_name.is_empty() {
            continue;
        }

        let extracted_val = evaluate_rust_extract_rule(&rule, response_body, response_headers);

        if let Some(val_str) = extracted_val {
            if val_str.trim().is_empty() {
                continue;
            }

            if let Some((entry, field)) = crate::credentials::parse_ref(target_var_name) {
                write_credential(entry, field, &val_str);
                continue;
            }
            let Some(active_env) = active_env.as_deref_mut() else { continue };

            let var_entry = active_env
                .variables
                .iter_mut()
                .find(|v| v.key.eq_ignore_ascii_case(target_var_name));

            match var_entry {
                Some(v) => {
                    let auto_idx = v
                        .variants
                        .iter()
                        .position(|variant| variant.name.eq_ignore_ascii_case("(auto)"));

                    match auto_idx {
                        Some(idx) => {
                            if let Some((entry, field)) = crate::credentials::link_target(&v.variants[idx].value) {
                                write_credential(entry, field, &val_str);
                                continue;
                            }
                            v.variants[idx].value = val_str.clone();
                            if v.active_index == idx {
                                v.value = val_str;
                            }
                        }
                        None => {
                            v.variants.insert(
                                0,
                                crate::workspace::VariableVariant {
                                    name: "(auto)".to_string(),
                                    value: val_str.clone(),
                                },
                            );
                            if v.active_index == 0 {
                                v.value = val_str;
                            }
                        }
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

    if let (true, Some(active_env)) = (modified, active_env) {
        let _ = crate::workspace::save_workspace_environment_db(db_path, active_env.clone());
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::repeater::ExtractRuleItem;

    #[test]
    fn test_extract_json_path() {
        let body = r#"{
            "token_type": "Bearer",
            "expires_in": 3600,
            "access_token": "token_abc_123",
            "data": {
                "id": "278d9778-cce7-4d70",
                "client_secret": "secret_xyz_789"
            },
            "items": [
                {"id": "item_0"},
                {"id": "item_1"}
            ]
        }"#;

        let rule1 = ExtractRuleItem {
            id: "1".to_string(),
            r#type: "json".to_string(),
            expression: "access_token".to_string(),
            target_variable: "access_token".to_string(),
            enabled: true,
        };
        assert_eq!(
            evaluate_rust_extract_rule(&rule1, body, &[]),
            Some("token_abc_123".to_string())
        );

        let rule2 = ExtractRuleItem {
            id: "2".to_string(),
            r#type: "json".to_string(),
            expression: "data.client_secret".to_string(),
            target_variable: "client_secret".to_string(),
            enabled: true,
        };
        assert_eq!(
            evaluate_rust_extract_rule(&rule2, body, &[]),
            Some("secret_xyz_789".to_string())
        );

        let rule3 = ExtractRuleItem {
            id: "3".to_string(),
            r#type: "json".to_string(),
            expression: "items.0.id".to_string(),
            target_variable: "first_item".to_string(),
            enabled: true,
        };
        assert_eq!(
            evaluate_rust_extract_rule(&rule3, body, &[]),
            Some("item_0".to_string())
        );

        let rule4 = ExtractRuleItem {
            id: "4".to_string(),
            r#type: "json".to_string(),
            expression: "$.items[1].id".to_string(),
            target_variable: "second_item".to_string(),
            enabled: true,
        };
        assert_eq!(
            evaluate_rust_extract_rule(&rule4, body, &[]),
            Some("item_1".to_string())
        );
    }

    #[tokio::test]
    async fn unreachable_pre_request_stops_the_main_request() {
        let db_path = std::env::temp_dir().join(format!("collection-pre-request-{}.db", uuid::Uuid::new_v4()));
        {
            let conn = crate::db::open(&db_path).unwrap();
            for m in crate::db::migrations::MIGRATIONS {
                for statement in m.sql.split(';').map(str::trim).filter(|s| !s.is_empty()) {
                    let _ = conn.execute(statement, []);
                }
            }
        }
        let closed_port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        let col = crate::collections::create_collection_db(&db_path, crate::repeater::REPEATER_WORKSPACE_ID.into(), None, "c".into()).unwrap();
        let mut fetch = crate::collections::create_request_db(&db_path, col.id.clone(), "Get CSRF Token".into()).unwrap();
        fetch.url = format!("http://127.0.0.1:{}/form", closed_port);
        crate::collections::update_request_db(&db_path, fetch.clone()).unwrap();
        let mut submit = crate::collections::create_request_db(&db_path, col.id, "Submit".into()).unwrap();
        submit.pre_requests = vec![fetch.id.clone()];
        crate::collections::update_request_db(&db_path, submit.clone()).unwrap();

        let result = execute_collection_request_db(&db_path, &submit.id).await.unwrap();
        let history = crate::collections::get_request_histories_db(&db_path, &submit.id).unwrap();
        let _ = std::fs::remove_file(&db_path);

        assert_eq!(result.status_code, 0);
        assert!(result.response_body.starts_with("Pre-request \"Get CSRF Token\" failed:"), "{}", result.response_body);
        assert_eq!(history.len(), 1);
    }

    /// Answers every request with the bytes it received, so a test can see what was actually sent.
    pub(crate) async fn serve_echo() -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                let mut received = Vec::new();
                let mut buf = vec![0u8; 8192];
                loop {
                    let n = sock.read(&mut buf).await.unwrap_or(0);
                    received.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&received);
                    let Some(end) = text.find("\r\n\r\n") else {
                        if n == 0 { break } else { continue }
                    };
                    let length = text[..end]
                        .lines()
                        .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0)))
                        .unwrap_or(0);
                    if n == 0 || received.len() >= end + 4 + length {
                        break;
                    }
                }
                let head = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", received.len());
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(&received).await;
            }
        });
        format!("http://{}", addr)
    }

    #[tokio::test]
    async fn history_masks_credential_secrets_but_keeps_codes_and_dynamic_values() {
        use crate::credentials::{create_credential, save_credential, Field, FieldKind};
        use crate::workspace::{save_workspace_environment_db, Environment, EnvironmentVariable, VariableVariant};

        let db_path = crate::credentials::tests_support::temp_db();
        let ws = crate::workspace::create_workspace_db(&db_path, "W".into(), None).unwrap().id;
        let mut cred = create_credential(&db_path, &ws, "dgAdmin").unwrap();
        let field = |key: &str, kind, value: &str| Field { key: key.into(), kind, extracted: false, value: value.into() };
        cred.fields = vec![field("password", FieldKind::Secret, "hunter2!"), field("totp", FieldKind::Totp, "MZXW6"), field("user", FieldKind::Text, "admin@x")];
        save_credential(&db_path, &cred).unwrap();
        save_workspace_environment_db(&db_path, Environment {
            id: "env".into(),
            workspace_id: ws.clone(),
            name: "dev".into(),
            is_active: true,
            variables: vec![EnvironmentVariable {
                key: "adminPassword".into(),
                value: String::new(),
                enabled: true,
                r#type: "default".into(),
                active_index: 0,
                variants: vec![VariableVariant { name: "dev".into(), value: "{{$cred:dgAdmin.password}}".into() }],
            }],
            created_at_ms: 0,
            updated_at_ms: 0,
        })
        .unwrap();

        let base = serve_echo().await;
        let col = crate::collections::create_collection_db(&db_path, ws.clone(), None, "c".into()).unwrap();
        let mut req = crate::collections::create_request_db(&db_path, col.id, "signin".into()).unwrap();
        req.method = "POST".into();
        req.url = format!("{base}/signin");
        req.params = vec![crate::repeater::ParamItem { id: "p".into(), key: "pw".into(), value: "{{adminPassword}}".into(), enabled: true }];
        req.headers = vec![HeaderItem { id: "h".into(), key: "X-Pass".into(), value: "{{$cred:dgAdmin.password}}".into(), enabled: true }];
        req.body_type = "raw".into();
        req.body_raw = Some("{{adminPassword}}|{{$cred:dgAdmin.totp}}|{{$cred:dgAdmin.user}}|{{$uuid}}".into());
        crate::collections::update_request_db(&db_path, req.clone()).unwrap();

        let result = execute_collection_request_db(&db_path, &req.id).await.unwrap();
        let history = crate::collections::get_request_histories_db(&db_path, &req.id).unwrap();
        let _ = std::fs::remove_file(&db_path);

        assert!(result.response_body.contains("x-pass: hunter2!"), "{}", result.response_body);
        assert!(result.response_body.contains("pw=hunter2%21"), "{}", result.response_body);
        let logged = &history[0];
        let everything = format!("{} {:?} {:?}", logged.url, logged.request_headers, logged.request_body);
        assert!(!everything.contains("hunter2"), "{everything}");
        assert!(logged.url.ends_with("pw=••••"), "{}", logged.url);
        let body = logged.request_body.as_deref().unwrap();
        let parts: Vec<&str> = body.split('|').collect();
        assert_eq!(parts[0], "••••");
        assert_eq!(parts[1].len(), 6);
        assert_eq!(parts[2], "admin@x");
        assert_eq!(parts[3].len(), 36);
        assert!(result.response_body.ends_with(&format!("|{}|admin@x|{}", parts[1], parts[3])), "history matches what was sent");
    }

    fn rule(target: &str) -> ExtractRuleItem {
        ExtractRuleItem {
            id: "r".into(),
            r#type: "json".into(),
            expression: "access_token".into(),
            target_variable: target.into(),
            enabled: true,
        }
    }

    fn linked_env(ws: &str, variants: Vec<(&str, &str)>) -> crate::workspace::Environment {
        crate::workspace::Environment {
            id: "env".into(),
            workspace_id: ws.into(),
            name: "dev".into(),
            is_active: true,
            variables: vec![crate::workspace::EnvironmentVariable {
                key: "accessToken".into(),
                value: variants[0].1.into(),
                enabled: true,
                r#type: "default".into(),
                active_index: 0,
                variants: variants
                    .into_iter()
                    .map(|(name, value)| crate::workspace::VariableVariant { name: name.into(), value: value.into() })
                    .collect(),
            }],
            created_at_ms: 0,
            updated_at_ms: 0,
        }
    }

    fn setup_extract() -> (PathBuf, String) {
        let db = crate::credentials::tests_support::temp_db();
        let ws = crate::workspace::create_workspace_db(&db, "W".into(), None).unwrap().id;
        crate::credentials::create_credential(&db, &ws, "dgAdmin").unwrap();
        (db, ws)
    }

    fn extract(db: &PathBuf, ws: &str, target: &str) {
        let rules = serde_json::to_string(&[rule(target)]).unwrap();
        perform_auto_extraction_db(db, ws, Some(ws), &rules, r#"{"access_token": "eyJtoken"}"#, &[]);
    }

    #[test]
    fn extract_writes_through_a_linked_auto_variant() {
        let (db, ws) = setup_extract();
        crate::workspace::save_workspace_environment_db(&db, linked_env(&ws, vec![("(auto)", "{{$cred:dgAdmin.accessToken}}")])).unwrap();
        extract(&db, &ws, "accessToken");

        let cred = crate::credentials::find_credential(&db, &ws, "dgAdmin").unwrap().unwrap();
        let field = cred.field("accessToken").unwrap();
        assert_eq!((field.value.as_str(), field.kind, field.extracted), ("eyJtoken", crate::credentials::FieldKind::Secret, true));
        let env = crate::workspace::active_environment_db(&db, &ws).unwrap();
        assert_eq!(env.variables[0].variants[0].value, "{{$cred:dgAdmin.accessToken}}");
        assert_eq!(env.variables[0].value, "{{$cred:dgAdmin.accessToken}}");
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn extract_writes_a_direct_cred_target_and_skips_a_missing_entry() {
        let (db, ws) = setup_extract();
        extract(&db, &ws, "$cred:dgAdmin.token");
        extract(&db, &ws, "{{$cred:nobody.token}}");
        let cred = crate::credentials::find_credential(&db, &ws, "dgAdmin").unwrap().unwrap();
        assert_eq!(cred.field("token").unwrap().value, "eyJtoken");
        assert!(crate::credentials::find_credential(&db, &ws, "nobody").unwrap().is_none());
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn extract_never_touches_a_linked_variant_other_than_auto() {
        let (db, ws) = setup_extract();
        let env = linked_env(&ws, vec![("(auto)", ""), ("manual", "{{$cred:dgAdmin.accessToken}}")]);
        crate::workspace::save_workspace_environment_db(&db, env).unwrap();
        extract(&db, &ws, "accessToken");

        let env = crate::workspace::active_environment_db(&db, &ws).unwrap();
        assert_eq!(env.variables[0].variants[0].value, "eyJtoken");
        assert_eq!(env.variables[0].variants[1].value, "{{$cred:dgAdmin.accessToken}}");
        let cred = crate::credentials::find_credential(&db, &ws, "dgAdmin").unwrap().unwrap();
        assert!(cred.field("accessToken").is_none());
        let _ = std::fs::remove_file(&db);
    }

    #[tokio::test]
    async fn an_extracted_token_is_masked_in_the_next_request() {
        let (db, ws) = setup_extract();
        crate::workspace::save_workspace_environment_db(&db, linked_env(&ws, vec![("(auto)", "{{$cred:dgAdmin.accessToken}}")])).unwrap();
        extract(&db, &ws, "accessToken");

        let base = serve_echo().await;
        let col = crate::collections::create_collection_db(&db, ws.clone(), None, "c".into()).unwrap();
        let mut req = crate::collections::create_request_db(&db, col.id, "me".into()).unwrap();
        req.url = format!("{base}/me");
        req.headers = vec![HeaderItem { id: "h".into(), key: "Authorization".into(), value: "Bearer {{accessToken}}".into(), enabled: true }];
        crate::collections::update_request_db(&db, req.clone()).unwrap();

        let result = execute_collection_request_db(&db, &req.id).await.unwrap();
        let history = crate::collections::get_request_histories_db(&db, &req.id).unwrap();
        let _ = std::fs::remove_file(&db);

        assert!(result.response_body.contains("authorization: Bearer eyJtoken"), "{}", result.response_body);
        let auth = history[0].request_headers.iter().find(|h| h.key == "Authorization").unwrap();
        assert_eq!(auth.value, "Bearer ••••");
    }

    #[tokio::test]
    async fn locked_credentials_abort_the_send_without_a_history_row() {
        let (db, ws) = setup_extract();
        crate::credentials::set_master_password(&db, None, "pw").unwrap();
        crate::credentials::lock(&db);

        let col = crate::collections::create_collection_db(&db, ws.clone(), None, "c".into()).unwrap();
        let mut uses_ref = crate::collections::create_request_db(&db, col.id.clone(), "signin".into()).unwrap();
        uses_ref.url = "http://127.0.0.1:9/{{$cred:dgAdmin.password}}".into();
        crate::collections::update_request_db(&db, uses_ref.clone()).unwrap();
        let mut extracts = crate::collections::create_request_db(&db, col.id.clone(), "token".into()).unwrap();
        extracts.url = "http://127.0.0.1:9/token".into();
        extracts.extract_rules = vec![rule("$cred:dgAdmin.accessToken")];
        crate::collections::update_request_db(&db, extracts.clone()).unwrap();
        let mut plain = crate::collections::create_request_db(&db, col.id, "plain".into()).unwrap();
        plain.url = "http://127.0.0.1:9/plain".into();
        plain.pre_requests = vec![uses_ref.id.clone()];
        crate::collections::update_request_db(&db, plain.clone()).unwrap();

        for id in [&uses_ref.id, &extracts.id, &plain.id] {
            assert_eq!(execute_collection_request_db(&db, id).await.unwrap_err(), crate::credentials::LOCKED);
            assert!(crate::collections::get_request_histories_db(&db, id).unwrap().is_empty());
        }
        crate::credentials::unlock(&db, "pw").unwrap();
        assert!(execute_collection_request_db(&db, &extracts.id).await.unwrap_err() != crate::credentials::LOCKED);
        let _ = std::fs::remove_file(&db);
    }
}
