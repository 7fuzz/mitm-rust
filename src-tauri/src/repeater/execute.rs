use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::Method;
use uuid::Uuid;

use super::{HeaderItem, RepeaterExecutionResult, RepeaterHistoryItem, RepeaterTab};
use crate::repeater::{get_repeater_tab_by_id, insert_repeater_history_db};
use crate::encoding::{build_multipart_payload, build_urlencoded_payload, format_body_for_ui};

pub async fn execute_tab_request(
    db_path: &PathBuf,
    tab_id: &str,
) -> Result<RepeaterExecutionResult, String> {
    let tab = get_repeater_tab_by_id(db_path, tab_id)?;
    execute_repeater_tab(db_path, &tab).await
}

fn detect_body_content_type(body_type: &str, body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    match body_type {
        "json" => Some("application/json".to_string()),
        "raw" | "text" => {
            if (trimmed.starts_with('{') && trimmed.ends_with('}'))
                || (trimmed.starts_with('[') && trimmed.ends_with(']'))
            {
                Some("application/json".to_string())
            } else if trimmed.starts_with("<!DOCTYPE html")
                || trimmed.starts_with("<html")
                || (trimmed.starts_with('<') && trimmed.ends_with('>') && trimmed.contains("</html"))
            {
                Some("text/html; charset=utf-8".to_string())
            } else if trimmed.starts_with("<?xml")
                || (trimmed.starts_with('<') && trimmed.ends_with('>') && (trimmed.contains("</") || trimmed.contains("/>")))
            {
                Some("application/xml".to_string())
            } else {
                Some("text/plain; charset=utf-8".to_string())
            }
        }
        _ => None,
    }
}

pub async fn execute_repeater_tab(
    db_path: &PathBuf,
    tab: &RepeaterTab,
) -> Result<RepeaterExecutionResult, String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .zstd(true)
        .build()
        .map_err(|e| e.to_string())?;

    // Parse method (fallback to GET if invalid)
    let method = Method::from_bytes(tab.method.as_bytes())
        .unwrap_or(Method::GET);

    // Filter enabled query parameters
    let enabled_params: Vec<&super::ParamItem> = tab
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .collect();

    // Auto-prefix http:// if URL missing scheme
    let raw_url = tab.url.trim();
    let url_with_scheme = if !raw_url.starts_with("http://") && !raw_url.starts_with("https://") {
        format!("http://{}", raw_url)
    } else {
        raw_url.to_string()
    };

    let mut parsed_url = reqwest::Url::parse(&url_with_scheme)
        .map_err(|e| format!("Invalid URL '{}': {}", tab.url, e))?;

    if !enabled_params.is_empty() {
        let mut query_pairs = parsed_url.query_pairs_mut();
        for p in enabled_params {
            query_pairs.append_pair(&p.key, &p.value);
        }
    }
    let final_url_str = parsed_url.to_string();

    let mut req_builder = client.request(method, &final_url_str);

    // Filter enabled request headers
    let enabled_headers: Vec<HeaderItem> = tab
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .cloned()
        .collect();

    let is_multipart = tab.body_type == "multipart" || tab.body_type == "form-data" || tab.body_type == "form";
    let is_urlencoded = tab.body_type == "urlencoded" || tab.body_type == "x-www-form-urlencoded";

    let mut has_explicit_content_type = false;

    for h in &enabled_headers {
        let key_lower = h.key.trim().to_lowercase();
        // Skip headers managed automatically by reqwest
        if key_lower == "host" || key_lower == "content-length" || key_lower == "transfer-encoding" {
            continue;
        }
        if key_lower == "content-type" {
            if is_multipart || is_urlencoded {
                continue;
            }
            has_explicit_content_type = true;
        }

        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(h.key.trim().as_bytes()),
            HeaderValue::from_str(&h.value),
        ) {
            req_builder = req_builder.header(name, val);
        }
    }

    // Attach multipart form, urlencoded form, or raw body
    let req_body_str = tab.body_content.clone().filter(|b| !b.trim().is_empty());
    let mut logged_headers = enabled_headers.clone();

    if is_urlencoded {
        if let Some(ref content) = req_body_str {
            if let Ok(params) = build_urlencoded_payload(content) {
                req_builder = req_builder.form(&params);
                if !logged_headers.iter().any(|h| h.key.eq_ignore_ascii_case("content-type")) {
                    logged_headers.push(HeaderItem {
                        id: Uuid::new_v4().to_string(),
                        key: "Content-Type".to_string(),
                        value: "application/x-www-form-urlencoded".to_string(),
                        enabled: true,
                    });
                }
            }
        }
    } else if is_multipart {
        if let Some(ref content) = req_body_str {
            if let Ok(form) = build_multipart_payload(content) {
                req_builder = req_builder.multipart(form);
                if !logged_headers.iter().any(|h| h.key.eq_ignore_ascii_case("content-type")) {
                    logged_headers.push(HeaderItem {
                        id: Uuid::new_v4().to_string(),
                        key: "Content-Type".to_string(),
                        value: "multipart/form-data".to_string(),
                        enabled: true,
                    });
                }
            }
        }
    } else if let Some(ref body) = req_body_str {
        if !has_explicit_content_type {
            if let Some(detected_ct) = detect_body_content_type(&tab.body_type, body) {
                if let Ok(val) = HeaderValue::from_str(&detected_ct) {
                    req_builder = req_builder.header(reqwest::header::CONTENT_TYPE, val);
                    logged_headers.push(HeaderItem {
                        id: Uuid::new_v4().to_string(),
                        key: "Content-Type".to_string(),
                        value: detected_ct,
                        enabled: true,
                    });
                }
            }
        }
        req_builder = req_builder.body(body.clone());
    }

    let start_time = Instant::now();
    let response_res = req_builder.send().await;
    let duration_ms = start_time.elapsed().as_millis() as u64;

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    match response_res {
        Ok(res) => {
            let status_code = res.status().as_u16();
            let status_text = res.status().canonical_reason().unwrap_or("").to_string();

            let mut response_headers: Vec<HeaderItem> = Vec::new();
            let mut content_encoding = String::new();
            let mut content_type = String::new();

            for (key, val) in res.headers() {
                let key_str = key.to_string();
                let val_str = val.to_str().unwrap_or("").to_string();

                if key_str.eq_ignore_ascii_case("content-encoding") {
                    content_encoding = val_str.clone();
                } else if key_str.eq_ignore_ascii_case("content-type") {
                    content_type = val_str.clone();
                }

                response_headers.push(HeaderItem {
                    id: Uuid::new_v4().to_string(),
                    key: key_str,
                    value: val_str,
                    enabled: true,
                });
            }

            let res_bytes = res.bytes().await.map_err(|e| e.to_string())?;
            let response_size = res_bytes.len() as u64;
            let response_body = format_body_for_ui(&res_bytes, &content_type, &content_encoding);

            let history_entry = RepeaterHistoryItem {
                id: 0,
                repeater_id: tab.id.clone(),
                method: tab.method.clone(),
                url: final_url_str.clone(),
                request_headers: logged_headers.clone(),
                request_body: req_body_str.clone(),
                status_code,
                response_headers: response_headers.clone(),
                response_body: Some(response_body.clone()),
                duration_ms,
                executed_at_ms: now_ms,
            };

            let history_id = insert_repeater_history_db(db_path, &history_entry).unwrap_or(0);

            Ok(RepeaterExecutionResult {
                history_id,
                repeater_id: tab.id.clone(),
                status_code,
                status_text,
                response_headers,
                response_body,
                duration_ms,
                response_size,
            })
        }
        Err(err) => {
            let err_msg = format!("Network Error: {}", err);
            let history_entry = RepeaterHistoryItem {
                id: 0,
                repeater_id: tab.id.clone(),
                method: tab.method.clone(),
                url: final_url_str.clone(),
                request_headers: logged_headers,
                request_body: req_body_str,
                status_code: 0,
                response_headers: vec![],
                response_body: Some(err_msg.clone()),
                duration_ms,
                executed_at_ms: now_ms,
            };

            let history_id = insert_repeater_history_db(db_path, &history_entry).unwrap_or(0);

            Ok(RepeaterExecutionResult {
                history_id,
                repeater_id: tab.id.clone(),
                status_code: 0,
                status_text: "ERR_FAILED".to_string(),
                response_headers: vec![],
                response_body: err_msg.clone(),
                duration_ms,
                response_size: err_msg.len() as u64,
            })
        }
    }
}
