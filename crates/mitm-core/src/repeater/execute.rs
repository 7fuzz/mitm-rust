use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::cookie::Jar;
use reqwest::Method;
use uuid::Uuid;
use base64::Engine;

use super::{HeaderItem, RepeaterExecutionResult, RepeaterHistoryItem, RepeaterTab};
use crate::repeater::{get_repeater_tab_by_id, insert_repeater_history_db, REPEATER_WORKSPACE_ID};
use crate::workspace::{get_workspace_environments_db, interpolate_dynamic_variables, interpolate_variables_with_env};
use crate::collections::execute::perform_auto_extraction_db;
use crate::encoding::{build_multipart_payload, build_urlencoded_payload, format_body_for_ui};
use crate::request_steps::{jar_cookie, pre_request_failure, request_steps};

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

pub struct SentRequest {
    pub final_url: String,
    pub request_headers: Vec<HeaderItem>,
    pub request_body: Option<String>,
    pub status_code: u16,
    pub status_text: String,
    pub response_headers: Vec<HeaderItem>,
    pub response_body: String,
    pub response_size: u64,
    pub duration_ms: u64,
}

fn http_client(jar: Option<Arc<Jar>>) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .zstd(true);
    if let Some(jar) = jar {
        builder = builder.cookie_provider(jar);
    }
    builder.build().map_err(|e| e.to_string())
}

/// Builds and sends a request from a tab and returns what was sent plus the response.
/// Does not touch the database; callers decide whether to log it.
pub async fn send_tab_request(tab: &RepeaterTab) -> Result<SentRequest, String> {
    send_tab_request_with(&http_client(None)?, tab).await
}

async fn send_tab_request_with(client: &reqwest::Client, tab: &RepeaterTab) -> Result<SentRequest, String> {
    // Parse method (fallback to GET if invalid)
    let method = Method::from_bytes(tab.method.as_bytes())
        .unwrap_or(Method::GET);

    // Filter enabled query parameters
    let enabled_params: Vec<&super::ParamItem> = tab
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .collect();

    let url_with_scheme = with_scheme(&tab.url);
    let mut parsed_url = reqwest::Url::parse(&url_with_scheme)
        .map_err(|e| format!("Invalid URL '{}': {}", tab.url, e))?;

    if !enabled_params.is_empty() {
        parsed_url.set_query(None);
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
    } else if tab.body_type == "binary" {
        if let Some(ref body) = req_body_str {
            let encoded: String = body.trim().trim_start_matches("base64:").split_whitespace().collect();
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|e| format!("Invalid base64 binary body: {}", e))?;
            if !has_explicit_content_type {
                req_builder = req_builder.header(reqwest::header::CONTENT_TYPE, "application/octet-stream");
                logged_headers.push(HeaderItem {
                    id: Uuid::new_v4().to_string(),
                    key: "Content-Type".to_string(),
                    value: "application/octet-stream".to_string(),
                    enabled: true,
                });
            }
            req_builder = req_builder.body(bytes);
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

            Ok(SentRequest {
                final_url: final_url_str,
                request_headers: logged_headers,
                request_body: req_body_str,
                status_code,
                status_text,
                response_headers,
                response_body,
                response_size,
                duration_ms,
            })
        }
        Err(err) => {
            let err_msg = format!("Network Error: {}", err);
            Ok(SentRequest {
                final_url: final_url_str,
                request_headers: logged_headers,
                request_body: req_body_str,
                status_code: 0,
                status_text: "ERR_FAILED".to_string(),
                response_headers: vec![],
                response_body: err_msg.clone(),
                response_size: err_msg.len() as u64,
                duration_ms,
            })
        }
    }
}

fn with_scheme(url: &str) -> String {
    let url = url.trim();
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("http://{}", url)
    }
}

/// Fills `{{vars}}` from the Repeater workspace's active environment.
fn interpolate_tab(db_path: &PathBuf, tab: &RepeaterTab) -> RepeaterTab {
    let env = get_workspace_environments_db(db_path, REPEATER_WORKSPACE_ID)
        .unwrap_or_default()
        .into_iter()
        .find(|e| e.is_active);
    let fill = |s: &str| match &env {
        Some(env) => interpolate_variables_with_env(env, s),
        None => interpolate_dynamic_variables(s),
    };

    let mut out = tab.clone();
    out.url = fill(&tab.url);
    for p in &mut out.params {
        p.key = fill(&p.key);
        p.value = fill(&p.value);
    }
    for h in &mut out.headers {
        h.key = fill(&h.key);
        h.value = fill(&h.value);
    }
    out.body_content = tab.body_content.as_deref().map(fill);
    out
}

pub async fn execute_repeater_tab(
    db_path: &PathBuf,
    tab: &RepeaterTab,
) -> Result<RepeaterExecutionResult, String> {
    let jar = Arc::new(Jar::default());
    let client = http_client(Some(jar.clone()))?;

    let steps = request_steps(db_path, &tab.id)?;
    for pre_id in &steps.pre {
        let pre_tab = get_repeater_tab_by_id(db_path, pre_id)?;
        let error = match run_repeater_tab(db_path, &pre_tab, &client, &jar).await {
            Ok(run) if run.status_code != 0 => continue,
            Ok(run) => run.response_body,
            Err(e) => e,
        };
        let message = pre_request_failure(db_path, pre_id, &error);
        return Ok(record_run(db_path, tab, unsent_request(&tab.url, message)));
    }

    let result = run_repeater_tab(db_path, tab, &client, &jar).await?;
    if result.status_code == 0 {
        return Ok(result);
    }

    // The main response is what the user asked for; a failing post-request only shows in its own history
    for post_id in &steps.post {
        if let Ok(post_tab) = get_repeater_tab_by_id(db_path, post_id) {
            let _ = run_repeater_tab(db_path, &post_tab, &client, &jar).await;
        }
    }
    Ok(result)
}

fn unsent_request(url: &str, error: String) -> SentRequest {
    SentRequest {
        final_url: url.to_string(),
        request_headers: vec![],
        request_body: None,
        status_code: 0,
        status_text: "ERR_FAILED".to_string(),
        response_headers: vec![],
        response_size: error.len() as u64,
        response_body: error,
        duration_ms: 0,
    }
}

async fn run_repeater_tab(
    db_path: &PathBuf,
    tab: &RepeaterTab,
    client: &reqwest::Client,
    jar: &Jar,
) -> Result<RepeaterExecutionResult, String> {
    let mut filled = interpolate_tab(db_path, tab);
    let has_cookie = filled.headers.iter().any(|h| h.enabled && h.key.trim().eq_ignore_ascii_case("cookie"));
    if !has_cookie {
        if let Some(cookie) = jar_cookie(jar, &with_scheme(&filled.url)) {
            filled.headers.push(HeaderItem {
                id: Uuid::new_v4().to_string(),
                key: "Cookie".to_string(),
                value: cookie,
                enabled: true,
            });
        }
    }

    let sent = send_tab_request_with(client, &filled).await?;
    let extract_rules_json = serde_json::to_string(&tab.extract_rules).unwrap_or_default();
    perform_auto_extraction_db(db_path, REPEATER_WORKSPACE_ID, &extract_rules_json, &sent.response_body, &sent.response_headers);
    Ok(record_run(db_path, tab, sent))
}

fn record_run(db_path: &PathBuf, tab: &RepeaterTab, sent: SentRequest) -> RepeaterExecutionResult {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let history_entry = RepeaterHistoryItem {
        id: 0,
        repeater_id: tab.id.clone(),
        method: tab.method.clone(),
        url: sent.final_url.clone(),
        request_headers: sent.request_headers.clone(),
        request_body: sent.request_body.clone(),
        status_code: sent.status_code,
        response_headers: sent.response_headers.clone(),
        response_body: Some(sent.response_body.clone()),
        duration_ms: sent.duration_ms,
        executed_at_ms: now_ms,
    };
    let history_id = insert_repeater_history_db(db_path, &history_entry).unwrap_or(0);

    RepeaterExecutionResult {
        history_id,
        repeater_id: tab.id.clone(),
        status_code: sent.status_code,
        status_text: sent.status_text,
        response_headers: sent.response_headers,
        response_body: sent.response_body,
        duration_ms: sent.duration_ms,
        response_size: sent.response_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repeater::{create_repeater_tab_db, ExtractRuleItem};
    use crate::workspace::{save_workspace_environment_db, Environment};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// `/form` hands out a session cookie and a CSRF token; any other path echoes the request head.
    async fn serve_csrf_site() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                let mut buf = vec![0u8; 8192];
                let n = sock.read(&mut buf).await.unwrap();
                let head = String::from_utf8_lossy(&buf[..n]).to_string();
                let response = if head.starts_with("GET /form") {
                    let body = r#"<input name="csrf" value="tok123">"#;
                    format!("HTTP/1.1 200 OK\r\nSet-Cookie: sid=abc; Path=/\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                } else {
                    format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", head.len(), head)
                };
                sock.write_all(response.as_bytes()).await.unwrap();
            }
        });
        format!("http://{}", addr)
    }

    fn tab(id: &str, url: String) -> RepeaterTab {
        RepeaterTab {
            id: id.into(),
            method: "GET".into(),
            url,
            headers: vec![],
            params: vec![],
            body_type: "none".into(),
            body_content: None,
            extract_rules: vec![],
            pre_requests: vec![],
            post_requests: vec![],
            order_index: 0,
            created_at_ms: 0,
            updated_at_ms: 0,
            execution_count: 0,
            last_status_code: None,
            last_duration_ms: None,
        }
    }

    #[tokio::test]
    async fn pre_request_supplies_token_and_session_cookie() {
        let db_path = std::env::temp_dir().join(format!("pre-request-e2e-{}.db", Uuid::new_v4()));
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            for m in crate::db::migrations::MIGRATIONS {
                for statement in m.sql.split(';').map(str::trim).filter(|s| !s.is_empty()) {
                    let _ = conn.execute(statement, []);
                }
            }
        }
        save_workspace_environment_db(&db_path, Environment {
            id: "env".into(),
            workspace_id: REPEATER_WORKSPACE_ID.into(),
            name: "Default".into(),
            is_active: true,
            variables: vec![],
            created_at_ms: 0,
            updated_at_ms: 0,
        })
        .unwrap();

        let base = serve_csrf_site().await;
        let mut fetch = tab("fetch", format!("{}/form", base));
        fetch.extract_rules = vec![ExtractRuleItem {
            id: "r".into(),
            r#type: "between_string".into(),
            expression: r#"value="||""#.into(),
            target_variable: "csrf_token".into(),
            enabled: true,
        }];
        let mut submit = tab("submit", format!("{}/submit", base));
        submit.headers = vec![HeaderItem {
            id: "h".into(),
            key: "X-CSRF-Token".into(),
            value: "{{csrf_token}}".into(),
            enabled: true,
        }];
        submit.pre_requests = vec!["fetch".into()];
        create_repeater_tab_db(&db_path, &fetch).unwrap();
        create_repeater_tab_db(&db_path, &submit).unwrap();

        let result = execute_repeater_tab(&db_path, &submit).await.unwrap();
        let _ = std::fs::remove_file(&db_path);

        let echoed = result.response_body.to_lowercase();
        assert!(echoed.contains("x-csrf-token: tok123"), "{}", echoed);
        assert!(echoed.contains("cookie: sid=abc"), "{}", echoed);
    }

    #[tokio::test]
    async fn unreachable_pre_request_stops_the_main_request() {
        let db_path = std::env::temp_dir().join(format!("pre-request-fail-{}.db", Uuid::new_v4()));
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            for m in crate::db::migrations::MIGRATIONS {
                for statement in m.sql.split(';').map(str::trim).filter(|s| !s.is_empty()) {
                    let _ = conn.execute(statement, []);
                }
            }
        }
        let base = serve_csrf_site().await;
        let closed_port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        let fetch = tab("fetch", format!("http://127.0.0.1:{}/form", closed_port));
        let mut submit = tab("submit", format!("{}/submit", base));
        submit.pre_requests = vec!["fetch".into()];
        create_repeater_tab_db(&db_path, &fetch).unwrap();
        create_repeater_tab_db(&db_path, &submit).unwrap();

        let result = execute_repeater_tab(&db_path, &submit).await.unwrap();
        let history = crate::repeater::get_repeater_history_db(&db_path, "submit", 1, 10).unwrap();
        let _ = std::fs::remove_file(&db_path);

        assert_eq!(result.status_code, 0);
        assert!(result.response_body.starts_with("Pre-request \"GET http://127.0.0.1:"), "{}", result.response_body);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status_code, 0);
    }


    #[tokio::test]
    async fn post_requests_run_after_the_main_request_with_its_cookies() {
        let db_path = std::env::temp_dir().join(format!("post-request-{}.db", Uuid::new_v4()));
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            for m in crate::db::migrations::MIGRATIONS {
                for statement in m.sql.split(';').map(str::trim).filter(|s| !s.is_empty()) {
                    let _ = conn.execute(statement, []);
                }
            }
        }
        let base = serve_csrf_site().await;
        let closed_port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        let mut main = tab("main", format!("{}/form", base));
        main.post_requests = vec!["unreachable".into(), "logout".into()];
        create_repeater_tab_db(&db_path, &tab("unreachable", format!("http://127.0.0.1:{}/", closed_port))).unwrap();
        create_repeater_tab_db(&db_path, &tab("logout", format!("{}/logout", base))).unwrap();
        create_repeater_tab_db(&db_path, &main).unwrap();

        let result = execute_repeater_tab(&db_path, &main).await.unwrap();
        let logout_runs = crate::repeater::get_repeater_history_db(&db_path, "logout", 1, 10).unwrap();
        let _ = std::fs::remove_file(&db_path);

        assert_eq!(result.status_code, 200);
        assert_eq!(logout_runs.len(), 1);
        let echoed = logout_runs[0].response_body.clone().unwrap_or_default().to_lowercase();
        assert!(echoed.starts_with("get /logout") && echoed.contains("cookie: sid=abc"), "{}", echoed);
    }
}
