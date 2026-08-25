use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::Method;
use uuid::Uuid;

use super::{HeaderItem, RepeaterExecutionResult, RepeaterHistoryItem, RepeaterTab};
use crate::repeater::{get_repeater_tab_by_id, insert_repeater_history_db};
use crate::encoding::format_body_for_ui;

pub async fn execute_tab_request(
    db_path: &PathBuf,
    tab_id: &str,
) -> Result<RepeaterExecutionResult, String> {
    let tab = get_repeater_tab_by_id(db_path, tab_id)?;
    execute_repeater_tab(db_path, &tab).await
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

    for h in &enabled_headers {
        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(h.key.trim().as_bytes()),
            HeaderValue::from_str(&h.value),
        ) {
            req_builder = req_builder.header(name, val);
        }
    }

    // Attach body if present and enabled
    let req_body_str = if tab.body_type != "none" {
        tab.body_content.clone()
    } else {
        None
    };

    if let Some(ref body) = req_body_str {
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
                request_headers: enabled_headers,
                request_body: req_body_str,
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
                request_headers: enabled_headers,
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
