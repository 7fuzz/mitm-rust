use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use regex::Regex;
use hyper::Uri;
use crate::state::{AppState, RewriteRule, RewriteHistoryEntry, RewriteCapturedEvent};

#[derive(Debug, Clone)]
pub struct MockResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

fn parse_path_from_url(url_str: &str) -> String {
    if let Ok(uri) = url_str.parse::<Uri>() {
        let p = uri.path();
        let q = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
        format!("{}{}", p, q)
    } else {
        "/".to_string()
    }
}

pub fn matches_rule(
    rule: &RewriteRule,
    method: &str,
    url: &str,
    host: &str,
    path: &str,
    headers: &[(String, String)],
) -> bool {
    let target_str = match rule.match_field.to_lowercase().as_str() {
        "all" => return true,
        "url" => url,
        "host" => host,
        "path" => path,
        "method" => method,
        "header" => {
            if let Some(ref target_hdr) = rule.target_header {
                let found = headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(target_hdr));
                if let Some((_, val)) = found {
                    val.as_str()
                } else {
                    return false;
                }
            } else {
                // Check all headers string
                return headers.iter().any(|(k, v)| {
                    k.to_lowercase().contains(&rule.match_value.to_lowercase())
                        || v.to_lowercase().contains(&rule.match_value.to_lowercase())
                });
            }
        }
        _ => url,
    };

    let match_val = &rule.match_value;
    if match_val.is_empty() && rule.match_field.to_lowercase() != "all" {
        return true;
    }

    match rule.match_operator.to_lowercase().as_str() {
        "contains" => target_str.to_lowercase().contains(&match_val.to_lowercase()),
        "equals" => target_str.eq_ignore_ascii_case(match_val),
        "starts_with" => target_str.to_lowercase().starts_with(&match_val.to_lowercase()),
        "ends_with" => target_str.to_lowercase().ends_with(&match_val.to_lowercase()),
        "regex" => {
            if let Ok(re) = Regex::new(match_val) {
                re.is_match(target_str)
            } else {
                false
            }
        }
        _ => target_str.to_lowercase().contains(&match_val.to_lowercase()),
    }
}

pub async fn apply_request_rewrite_pipeline(
    app_handle: &AppHandle,
    state: &Arc<AppState>,
    method: &mut String,
    url: &mut String,
    host: &mut String,
    headers: &mut Vec<(String, String)>,
    body: &mut Vec<u8>,
) -> Option<MockResponse> {
    if !state.is_rewrite_enabled() {
        return None;
    }

    let rules = {
        let r = state.rewrite_rules.read().await;
        r.clone()
    };

    let path = parse_path_from_url(url);

    for rule in rules.iter().filter(|r| r.enabled) {
        let is_req_action = rule.action_type == "partial_request"
            || rule.action_type == "full_request"
            || rule.action_type == "redirect"
            || rule.action_type == "full_response";

        if !is_req_action {
            continue;
        }

        if !matches_rule(rule, method, url, host, &path, headers) {
            continue;
        }

        let orig_method = method.clone();
        let orig_url = url.clone();
        let orig_headers = headers.clone();
        let orig_body_text = String::from_utf8_lossy(body).to_string();

        let mut changed = false;

        match rule.action_type.as_str() {
            "partial_request" => {
                match rule.target_part.as_str() {
                    "url" => {
                        if rule.is_regex {
                            if let Ok(re) = Regex::new(&rule.match_pattern) {
                                let new_url = re.replace_all(url, &rule.replacement_value).to_string();
                                if new_url != *url {
                                    *url = new_url;
                                    changed = true;
                                }
                            }
                        } else if !rule.match_pattern.is_empty() && url.contains(&rule.match_pattern) {
                            *url = url.replace(&rule.match_pattern, &rule.replacement_value);
                            changed = true;
                        }

                        // Re-extract host if URL changed
                        if changed {
                            if let Ok(parsed_uri) = url.parse::<Uri>() {
                                if let Some(h) = parsed_uri.host() {
                                    *host = h.to_string();
                                    for (k, v) in headers.iter_mut() {
                                        if k.eq_ignore_ascii_case("host") {
                                            *v = h.to_string();
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "query" => {
                        if !rule.match_pattern.is_empty() {
                            if let Ok(mut parsed_uri) = url.parse::<url::Url>() {
                                parsed_uri.query_pairs_mut().append_pair(&rule.match_pattern, &rule.replacement_value);
                                *url = parsed_uri.to_string();
                                changed = true;
                            }
                        }
                    }
                    "header" => {
                        if let Some(ref hdr_name) = rule.target_header {
                            if rule.replacement_value.is_empty() {
                                // Remove header
                                let prev_len = headers.len();
                                headers.retain(|(k, _)| !k.eq_ignore_ascii_case(hdr_name));
                                if headers.len() != prev_len {
                                    changed = true;
                                }
                            } else {
                                // Modify / Insert header
                                let mut found = false;
                                for (k, v) in headers.iter_mut() {
                                    if k.eq_ignore_ascii_case(hdr_name) {
                                        *v = rule.replacement_value.clone();
                                        found = true;
                                        changed = true;
                                        break;
                                    }
                                }
                                if !found {
                                    headers.push((hdr_name.clone(), rule.replacement_value.clone()));
                                    changed = true;
                                }
                            }
                        }
                    }
                    "body" => {
                        let current_body_str = String::from_utf8_lossy(body).to_string();
                        let new_body_str = if rule.is_regex {
                            if let Ok(re) = Regex::new(&rule.match_pattern) {
                                re.replace_all(&current_body_str, &rule.replacement_value).to_string()
                            } else {
                                current_body_str.clone()
                            }
                        } else if !rule.match_pattern.is_empty() {
                            current_body_str.replace(&rule.match_pattern, &rule.replacement_value)
                        } else {
                            current_body_str.clone()
                        };

                        if new_body_str != current_body_str {
                            *body = new_body_str.into_bytes();
                            changed = true;
                        }
                    }
                    _ => {}
                }
            }
            "full_request" => {
                match rule.target_part.as_str() {
                    "url" => {
                        if !rule.replacement_value.is_empty() {
                            *url = rule.replacement_value.clone();
                            if let Ok(parsed_uri) = url.parse::<Uri>() {
                                if let Some(h) = parsed_uri.host() {
                                    *host = h.to_string();
                                    for (k, v) in headers.iter_mut() {
                                        if k.eq_ignore_ascii_case("host") {
                                            *v = h.to_string();
                                            break;
                                        }
                                    }
                                }
                            }
                            changed = true;
                        }
                    }
                    "body" => {
                        *body = rule.replacement_value.as_bytes().to_vec();
                        changed = true;
                    }
                    "method" => {
                        if !rule.replacement_value.is_empty() {
                            *method = rule.replacement_value.to_uppercase();
                            changed = true;
                        }
                    }
                    _ => {}
                }
            }
            "redirect" => {
                let redirect_url = if rule.is_regex {
                    if let Ok(re) = Regex::new(&rule.match_pattern) {
                        re.replace_all(url, &rule.replacement_value).to_string()
                    } else {
                        rule.replacement_value.clone()
                    }
                } else if !rule.match_pattern.is_empty() && url.contains(&rule.match_pattern) {
                    url.replace(&rule.match_pattern, &rule.replacement_value)
                } else if !rule.replacement_value.is_empty() {
                    rule.replacement_value.clone()
                } else {
                    url.clone()
                };

                let status = rule.mock_status_code.unwrap_or(307);
                let mut redirect_headers: Vec<(String, String)> = vec![
                    ("Location".to_string(), redirect_url.clone()),
                    ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
                    ("Access-Control-Allow-Credentials".to_string(), "true".to_string()),
                    ("Access-Control-Allow-Methods".to_string(), "GET, POST, PUT, DELETE, PATCH, OPTIONS".to_string()),
                    ("Access-Control-Allow-Headers".to_string(), "*".to_string()),
                    ("Content-Type".to_string(), "text/plain".to_string()),
                ];

                if let Some(ref j) = rule.mock_headers_json {
                    if let Ok(extra_hdrs) = serde_json::from_str::<Vec<(String, String)>>(j) {
                        for (k, v) in extra_hdrs {
                            redirect_headers.retain(|(ek, _)| !ek.eq_ignore_ascii_case(&k));
                            redirect_headers.push((k, v));
                        }
                    }
                }

                let redirect_body = format!("Redirecting to {}...", redirect_url).into_bytes();

                let entry = RewriteHistoryEntry {
                    id: format!("rw-{}", uuid::Uuid::new_v4()),
                    rule_id: Some(rule.id.clone()),
                    rule_name: rule.name.clone(),
                    action_type: format!("redirect ({})", status),
                    method: orig_method,
                    original_url: orig_url,
                    rewritten_url: redirect_url,
                    original_headers: orig_headers,
                    rewritten_headers: redirect_headers.clone(),
                    original_body: orig_body_text,
                    rewritten_body: String::from_utf8_lossy(&redirect_body).to_string(),
                    status_code: Some(status),
                    duration_ms: Some(0),
                    created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                };

                let _ = state.rewrite_tx.send(entry.clone()).await;
                let _ = app_handle.emit("rewrite_captured", RewriteCapturedEvent { entry });

                return Some(MockResponse {
                    status,
                    headers: redirect_headers,
                    body: redirect_body,
                });
            }
            "full_response" => {
                // Short-circuit with mock response
                let status = rule.mock_status_code.unwrap_or(200);
                let mock_headers: Vec<(String, String)> = rule.mock_headers_json
                    .as_deref()
                    .and_then(|j| serde_json::from_str(j).ok())
                    .unwrap_or_else(|| vec![("Content-Type".to_string(), "application/json".to_string())]);

                let mock_body_bytes = rule.mock_body.as_deref().unwrap_or("").as_bytes().to_vec();

                let entry = RewriteHistoryEntry {
                    id: format!("rw-{}", uuid::Uuid::new_v4()),
                    rule_id: Some(rule.id.clone()),
                    rule_name: rule.name.clone(),
                    action_type: "full_response (Mock)".to_string(),
                    method: orig_method,
                    original_url: orig_url,
                    rewritten_url: url.clone(),
                    original_headers: orig_headers,
                    rewritten_headers: mock_headers.clone(),
                    original_body: orig_body_text,
                    rewritten_body: rule.mock_body.clone().unwrap_or_default(),
                    status_code: Some(status),
                    duration_ms: Some(0),
                    created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                };

                let _ = state.rewrite_tx.send(entry.clone()).await;
                let _ = app_handle.emit("rewrite_captured", RewriteCapturedEvent { entry });

                return Some(MockResponse {
                    status,
                    headers: mock_headers,
                    body: mock_body_bytes,
                });
            }
            _ => {}
        }

        if changed {
            let rewr_body_text = String::from_utf8_lossy(body).to_string();
            let entry = RewriteHistoryEntry {
                id: format!("rw-{}", uuid::Uuid::new_v4()),
                rule_id: Some(rule.id.clone()),
                rule_name: rule.name.clone(),
                action_type: rule.action_type.clone(),
                method: method.clone(),
                original_url: orig_url,
                rewritten_url: url.clone(),
                original_headers: orig_headers,
                rewritten_headers: headers.clone(),
                original_body: orig_body_text,
                rewritten_body: rewr_body_text,
                status_code: None,
                duration_ms: None,
                created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            let _ = state.rewrite_tx.send(entry.clone()).await;
            let _ = app_handle.emit("rewrite_captured", RewriteCapturedEvent { entry });
        }
    }

    None
}

pub async fn apply_response_rewrite_pipeline(
    app_handle: &AppHandle,
    state: &Arc<AppState>,
    method: &str,
    url: &str,
    host: &str,
    status: &mut u16,
    headers: &mut Vec<(String, String)>,
    body: &mut Vec<u8>,
) -> bool {
    if !state.is_rewrite_enabled() {
        return false;
    }

    let rules = {
        let r = state.rewrite_rules.read().await;
        r.clone()
    };

    let mut any_changed = false;
    let path = parse_path_from_url(url);

    for rule in rules.iter().filter(|r| r.enabled) {
        let is_res_action = rule.action_type == "partial_response"
            || rule.action_type == "full_response";

        if !is_res_action {
            continue;
        }

        if !matches_rule(rule, method, url, host, &path, headers) {
            continue;
        }

        let orig_headers = headers.clone();
        let orig_body_text = String::from_utf8_lossy(body).to_string();
        let _orig_status = *status;

        let mut changed = false;

        match rule.action_type.as_str() {
            "partial_response" => {
                match rule.target_part.as_str() {
                    "header" => {
                        if let Some(ref hdr_name) = rule.target_header {
                            if rule.replacement_value.is_empty() {
                                let prev_len = headers.len();
                                headers.retain(|(k, _)| !k.eq_ignore_ascii_case(hdr_name));
                                if headers.len() != prev_len {
                                    changed = true;
                                }
                            } else {
                                let mut found = false;
                                for (k, v) in headers.iter_mut() {
                                    if k.eq_ignore_ascii_case(hdr_name) {
                                        *v = rule.replacement_value.clone();
                                        found = true;
                                        changed = true;
                                        break;
                                    }
                                }
                                if !found {
                                    headers.push((hdr_name.clone(), rule.replacement_value.clone()));
                                    changed = true;
                                }
                            }
                        }
                    }
                    "body" => {
                        let current_body_str = String::from_utf8_lossy(body).to_string();
                        let new_body_str = if rule.is_regex {
                            if let Ok(re) = Regex::new(&rule.match_pattern) {
                                re.replace_all(&current_body_str, &rule.replacement_value).to_string()
                            } else {
                                current_body_str.clone()
                            }
                        } else if !rule.match_pattern.is_empty() {
                            current_body_str.replace(&rule.match_pattern, &rule.replacement_value)
                        } else {
                            current_body_str.clone()
                        };

                        if new_body_str != current_body_str {
                            *body = new_body_str.into_bytes();
                            changed = true;
                        }
                    }
                    _ => {}
                }
            }
            "full_response" => {
                if let Some(code) = rule.mock_status_code {
                    *status = code;
                    changed = true;
                }
                if let Some(ref j) = rule.mock_headers_json {
                    if let Ok(hdrs) = serde_json::from_str::<Vec<(String, String)>>(j) {
                        *headers = hdrs;
                        changed = true;
                    }
                }
                if let Some(ref b) = rule.mock_body {
                    *body = b.as_bytes().to_vec();
                    changed = true;
                }
            }
            _ => {}
        }

        if changed {
            any_changed = true;
            let rewr_body_text = String::from_utf8_lossy(body).to_string();
            let entry = RewriteHistoryEntry {
                id: format!("rw-{}", uuid::Uuid::new_v4()),
                rule_id: Some(rule.id.clone()),
                rule_name: rule.name.clone(),
                action_type: rule.action_type.clone(),
                method: method.to_string(),
                original_url: url.to_string(),
                rewritten_url: url.to_string(),
                original_headers: orig_headers,
                rewritten_headers: headers.clone(),
                original_body: orig_body_text,
                rewritten_body: rewr_body_text,
                status_code: Some(*status),
                duration_ms: None,
                created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            let _ = state.rewrite_tx.send(entry.clone()).await;
            let _ = app_handle.emit("rewrite_captured", RewriteCapturedEvent { entry });
        }
    }

    any_changed
}
