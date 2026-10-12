use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::events::EventBus;
use regex::Regex;
use hyper::Uri;
use crate::state::AppState;

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

/// The text with the rule's pattern replaced. An invalid regex leaves it unchanged.
pub fn replace_in(rule: &RewriteRule, text: &str) -> String {
    if rule.is_regex {
        match Regex::new(&rule.match_pattern) {
            Ok(re) => re.replace_all(text, &rule.replacement_value).into_owned(),
            Err(_) => text.to_string(),
        }
    } else if rule.match_pattern.is_empty() {
        text.to_string()
    } else {
        text.replace(&rule.match_pattern, &rule.replacement_value)
    }
}

/// Sets a header, adding it when missing; an empty value removes it. Returns whether anything changed.
pub fn set_header(headers: &mut Vec<(String, String)>, name: &str, value: &str) -> bool {
    if value.is_empty() {
        let before = headers.len();
        headers.retain(|(k, _)| !k.eq_ignore_ascii_case(name));
        return headers.len() != before;
    }
    match headers.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(name)) {
        Some((_, v)) => *v = value.to_string(),
        None => headers.push((name.to_string(), value.to_string())),
    }
    true
}

pub fn redirect_target(rule: &RewriteRule, url: &str) -> String {
    if rule.is_regex {
        match Regex::new(&rule.match_pattern) {
            Ok(re) => re.replace_all(url, &rule.replacement_value).to_string(),
            Err(_) => rule.replacement_value.clone(),
        }
    } else if !rule.match_pattern.is_empty() && url.contains(&rule.match_pattern) {
        url.replace(&rule.match_pattern, &rule.replacement_value)
    } else if !rule.replacement_value.is_empty() {
        rule.replacement_value.clone()
    } else {
        url.to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RewritePreview {
    pub output: String,
    /// Byte ranges in the sample that the rule's pattern matches.
    pub matches: Vec<std::ops::Range<usize>>,
}

/// What a rule does to a sample of its target part (a URL, a body, or `Name: value` header lines), without traffic.
pub fn preview_rewrite(rule: &RewriteRule, sample: &str) -> Result<RewritePreview, String> {
    let regex = if rule.is_regex && !rule.match_pattern.is_empty() {
        Some(Regex::new(&rule.match_pattern).map_err(|e| e.to_string())?)
    } else {
        None
    };
    let matches = match &regex {
        Some(re) => re.find_iter(sample).map(|m| m.range()).collect(),
        None if !rule.is_regex && !rule.match_pattern.is_empty() => {
            sample.match_indices(&rule.match_pattern).map(|(i, m)| i..i + m.len()).collect()
        }
        None => Vec::new(),
    };
    let output = match (rule.action_type.as_str(), rule.target_part.as_str()) {
        ("partial_request" | "partial_response", "header") => {
            let mut headers: Vec<(String, String)> = sample
                .lines()
                .filter_map(|line| line.split_once(':').map(|(k, v)| (k.trim().to_string(), v.trim().to_string())))
                .collect();
            if let Some(name) = rule.target_header.as_deref().filter(|n| !n.is_empty()) {
                set_header(&mut headers, name, &rule.replacement_value);
            }
            headers.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join("\n")
        }
        ("partial_request", "query") => {
            let mut url = url::Url::parse(sample).map_err(|e| e.to_string())?;
            if !rule.match_pattern.is_empty() {
                url.query_pairs_mut().append_pair(&rule.match_pattern, &rule.replacement_value);
            }
            url.to_string()
        }
        ("partial_request" | "partial_response", _) => replace_in(rule, sample),
        ("full_request", "method") => rule.replacement_value.to_uppercase(),
        ("full_request", _) => rule.replacement_value.clone(),
        ("redirect", _) => redirect_target(rule, sample),
        ("full_response", _) => rule.mock_body.clone().unwrap_or_default(),
        _ => sample.to_string(),
    };
    Ok(RewritePreview { output, matches })
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

#[allow(clippy::too_many_arguments)]
pub async fn apply_request_rewrite_pipeline(
    _event_bus: Option<&EventBus>,
    state: &Arc<AppState>,
    method: &mut String,
    url: &mut String,
    host: &mut String,
    headers: &mut Vec<(String, String)>,
    body: &mut Vec<u8>,
    listener_label: &str,
) -> Option<MockResponse> {
    if !state.is_rewrite_enabled() {
        return None;
    }

    let scope = state.rewrite_source_scope.read().await.clone();
    if !state.source_matches(&scope, listener_label).await {
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
                        let new_url = replace_in(rule, url);
                        if new_url != *url {
                            *url = new_url;
                            changed = true;
                        }

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
                            changed = set_header(headers, hdr_name, &rule.replacement_value);
                        }
                    }
                    "body" => {
                        let current_body_str = String::from_utf8_lossy(body).to_string();
                        let new_body_str = replace_in(rule, &current_body_str);

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
                    "method" if !rule.replacement_value.is_empty() => {
                        *method = rule.replacement_value.to_uppercase();
                        changed = true;
                    }
                    _ => {}
                }
            }
            "redirect" => {
                let redirect_url = redirect_target(rule, url);

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
                    id: state.next_rewrite_history_id(),
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

                return Some(MockResponse {
                    status,
                    headers: redirect_headers,
                    body: redirect_body,
                });
            }
            "full_response" => {
                let status = rule.mock_status_code.unwrap_or(200);
                let mock_headers: Vec<(String, String)> = rule.mock_headers_json
                    .as_deref()
                    .and_then(|j| serde_json::from_str(j).ok())
                    .unwrap_or_else(|| vec![("Content-Type".to_string(), "application/json".to_string())]);

                let mock_body_bytes = rule.mock_body.as_deref().unwrap_or("").as_bytes().to_vec();

                let entry = RewriteHistoryEntry {
                    id: state.next_rewrite_history_id(),
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
                id: state.next_rewrite_history_id(),
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
        }
    }

    None
}

#[allow(clippy::too_many_arguments)]
pub async fn apply_response_rewrite_pipeline(
    _event_bus: Option<&EventBus>,
    state: &Arc<AppState>,
    method: &str,
    url: &str,
    host: &str,
    status: &mut u16,
    headers: &mut Vec<(String, String)>,
    body: &mut Vec<u8>,
    listener_label: &str,
) -> bool {
    if !state.is_rewrite_enabled() {
        return false;
    }

    let scope = state.rewrite_source_scope.read().await.clone();
    if !state.source_matches(&scope, listener_label).await {
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
                            changed = set_header(headers, hdr_name, &rule.replacement_value);
                        }
                    }
                    "body" => {
                        let current_body_str = String::from_utf8_lossy(body).to_string();
                        let new_body_str = replace_in(rule, &current_body_str);

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
                id: state.next_rewrite_history_id(),
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
        }
    }

    any_changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(action: &str, target: &str, pattern: &str, replacement: &str, is_regex: bool) -> RewriteRule {
        RewriteRule {
            id: "r".to_string(),
            name: "r".to_string(),
            enabled: true,
            action_type: action.to_string(),
            match_field: "all".to_string(),
            match_operator: "contains".to_string(),
            match_value: String::new(),
            target_part: target.to_string(),
            target_header: None,
            match_pattern: pattern.to_string(),
            replacement_value: replacement.to_string(),
            is_regex,
            mock_status_code: None,
            mock_headers_json: None,
            mock_body: None,
            order_index: 0,
            created_at_ms: 0,
        }
    }

    #[test]
    fn preview_replaces_and_reports_matched_spans() {
        let preview = preview_rewrite(&rule("partial_response", "body", r"\d+", "N", true), "a1 b22").unwrap();
        assert_eq!(preview.output, "aN bN");
        assert_eq!(preview.matches, vec![1..2, 4..6]);

        let literal = preview_rewrite(&rule("partial_request", "url", "prod", "dev", false), "https://prod.x/prod").unwrap();
        assert_eq!(literal.output, "https://dev.x/dev");
        assert_eq!(literal.matches.len(), 2);
    }

    #[test]
    fn preview_reports_bad_regex_and_handles_headers_and_redirects() {
        assert!(preview_rewrite(&rule("partial_request", "body", "(", "", true), "x").is_err());

        let mut header = rule("partial_request", "header", "", "abc", false);
        header.target_header = Some("X-Token".to_string());
        assert_eq!(preview_rewrite(&header, "Host: a").unwrap().output, "Host: a\nX-Token: abc");

        let redirect = rule("redirect", "url", "", "https://elsewhere", false);
        assert_eq!(preview_rewrite(&redirect, "https://here").unwrap().output, "https://elsewhere");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub action_type: String, // "partial_request" | "full_request" | "partial_response" | "full_response"
    pub match_field: String, // "all" | "url" | "host" | "path" | "method" | "header"
    pub match_operator: String, // "contains" | "equals" | "regex" | "starts_with"
    pub match_value: String,
    pub target_part: String, // "url" | "query" | "header" | "body" | "method" | "status"
    pub target_header: Option<String>,
    pub match_pattern: String,
    pub replacement_value: String,
    pub is_regex: bool,
    pub mock_status_code: Option<u16>,
    pub mock_headers_json: Option<String>,
    pub mock_body: Option<String>,
    pub order_index: i32,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteHistoryEntry {
    pub id: i64,
    pub rule_id: Option<String>,
    pub rule_name: String,
    pub action_type: String,
    pub method: String,
    pub original_url: String,
    pub rewritten_url: String,
    pub original_headers: Vec<(String, String)>,
    pub rewritten_headers: Vec<(String, String)>,
    pub original_body: String,
    pub rewritten_body: String,
    pub status_code: Option<u16>,
    pub duration_ms: Option<u64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteCapturedEvent {
    pub entry: RewriteHistoryEntry,
}
