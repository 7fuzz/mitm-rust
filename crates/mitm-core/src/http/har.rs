//! HAR 1.2 (http://www.softwareishard.com/blog/har-12-spec/) built from traffic history.

use base64::Engine;
use serde_json::{json, Value};
use crate::proxy::history::HistoryEntry;

const BASE64_PREFIX: &str = "base64:";

pub fn build_har(entries: &[HistoryEntry]) -> Value {
    json!({
        "log": {
            "version": "1.2",
            "creator": { "name": "mitm", "version": env!("CARGO_PKG_VERSION") },
            "entries": entries.iter().map(har_entry).collect::<Vec<_>>(),
        }
    })
}

fn har_entry(entry: &HistoryEntry) -> Value {
    let wait = entry.duration_ms.unwrap_or(0);
    json!({
        "startedDateTime": started_at(entry),
        "time": wait,
        "request": request(entry),
        "response": response(entry),
        "cache": {},
        "timings": { "blocked": -1, "dns": -1, "connect": -1, "ssl": -1, "send": 0, "wait": wait, "receive": 0 },
    })
}

fn request(entry: &HistoryEntry) -> Value {
    let headers = &entry.request_headers;
    let mut request = json!({
        "method": entry.method,
        "url": entry.url,
        "httpVersion": "HTTP/1.1",
        "cookies": header_values(headers, "cookie").flat_map(request_cookies).collect::<Vec<_>>(),
        "headers": har_headers(headers),
        "queryString": query_string(&entry.url),
        "headersSize": -1,
        "bodySize": body_size(&entry.request_body),
    });
    if !entry.request_body.is_empty() {
        let mime_type = header_values(headers, "content-type").next().unwrap_or_default();
        let mut post_data = json!({ "mimeType": mime_type, "params": [] });
        match entry.request_body.strip_prefix(BASE64_PREFIX) {
            // postData has no encoding field, so a binary body is marked with a custom one.
            Some(encoded) => {
                post_data["text"] = json!(encoded);
                post_data["_encoding"] = json!("base64");
            }
            None => post_data["text"] = json!(entry.request_body),
        }
        request["postData"] = post_data;
    }
    request
}

fn response(entry: &HistoryEntry) -> Value {
    let headers = &entry.response_headers;
    let mime_type = header_values(headers, "content-type")
        .next()
        .or_else(|| Some(entry.content_type.as_str()).filter(|t| *t != "-"))
        .unwrap_or_default();
    let mut content = json!({ "size": body_size(&entry.response_body).max(0), "mimeType": mime_type });
    match entry.response_body.strip_prefix(BASE64_PREFIX) {
        Some(encoded) => {
            content["text"] = json!(encoded);
            content["encoding"] = json!("base64");
        }
        None if !entry.response_body.is_empty() => content["text"] = json!(entry.response_body),
        None => {}
    }
    json!({
        "status": entry.status_code,
        "statusText": status_text(entry.status_code),
        "httpVersion": "HTTP/1.1",
        "cookies": header_values(headers, "set-cookie").map(response_cookie).collect::<Vec<_>>(),
        "headers": har_headers(headers),
        "content": content,
        "redirectURL": header_values(headers, "location").next().unwrap_or_default(),
        "headersSize": -1,
        "bodySize": if entry.status_code == 0 { -1 } else { entry.response_size as i64 },
    })
}

fn started_at(entry: &HistoryEntry) -> String {
    if let Some(at) = entry.request_at.as_deref().filter(|at| !at.is_empty()) {
        return at.to_string();
    }
    // `created_at` is SQLite's CURRENT_TIMESTAMP: UTC without a zone.
    chrono::NaiveDateTime::parse_from_str(&entry.created_at, "%Y-%m-%d %H:%M:%S")
        .map(|t| t.and_utc().to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_else(|_| entry.created_at.clone())
}

fn status_text(code: u16) -> &'static str {
    hyper::StatusCode::from_u16(code).ok().and_then(|s| s.canonical_reason()).unwrap_or_default()
}

fn body_size(body: &str) -> i64 {
    match body.strip_prefix(BASE64_PREFIX) {
        Some(encoded) => base64::engine::general_purpose::STANDARD.decode(encoded).map_or(-1, |b| b.len() as i64),
        None => body.len() as i64,
    }
}

fn har_headers(headers: &[(String, String)]) -> Vec<Value> {
    headers.iter().map(|(name, value)| json!({ "name": name, "value": value })).collect()
}

fn header_values<'a>(headers: &'a [(String, String)], name: &'a str) -> impl Iterator<Item = &'a str> {
    headers.iter().filter(move |(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
}

fn query_string(url: &str) -> Vec<Value> {
    let Ok(url) = url::Url::parse(url) else { return Vec::new() };
    url.query_pairs().map(|(name, value)| json!({ "name": name, "value": value })).collect()
}

fn request_cookies(header: &str) -> impl Iterator<Item = Value> + '_ {
    header.split(';').filter_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        Some(json!({ "name": name, "value": value }))
    })
}

fn response_cookie(header: &str) -> Value {
    let mut parts = header.split(';').map(str::trim);
    let (name, value) = parts.next().and_then(|p| p.split_once('=')).unwrap_or((header, ""));
    let mut cookie = json!({ "name": name, "value": value });
    for attr in parts {
        let (key, val) = attr.split_once('=').unwrap_or((attr, ""));
        match key.to_ascii_lowercase().as_str() {
            "path" => cookie["path"] = json!(val),
            "domain" => cookie["domain"] = json!(val),
            "expires" => cookie["expires"] = json!(val),
            "httponly" => cookie["httpOnly"] = json!(true),
            "secure" => cookie["secure"] = json!(true),
            _ => {}
        }
    }
    cookie
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> HistoryEntry {
        HistoryEntry {
            id: 1,
            method: "POST".into(),
            url: "https://example.test/login?next=%2Fhome&x=1".into(),
            host: "example.test".into(),
            path: "/login".into(),
            content_type: "application/json".into(),
            response_size: 12,
            status_code: 302,
            request_headers: vec![
                ("Content-Type".into(), "application/json".into()),
                ("Cookie".into(), "a=1; b=two".into()),
            ],
            response_headers: vec![
                ("Location".into(), "/home".into()),
                ("Set-Cookie".into(), "sid=abc; Path=/; HttpOnly; Secure".into()),
            ],
            request_body: r#"{"u":"me"}"#.into(),
            response_body: "redirecting!".into(),
            phase: "response".into(),
            duration_ms: Some(42),
            created_at: "2026-10-11 08:30:05".into(),
            is_intercepted: false,
            is_rewritten: false,
            is_failed: false,
            listener_label: String::new(),
            container_label: String::new(),
            run_id: 0,
            request_at: None,
            response_at: None,
        }
    }

    fn first(har: &Value) -> &Value {
        &har["log"]["entries"][0]
    }

    #[test]
    fn maps_request_and_response_parts() {
        let har = build_har(&[entry()]);
        let e = first(&har);
        assert_eq!(har["log"]["version"], "1.2");
        assert_eq!(e["startedDateTime"], "2026-10-11T08:30:05.000Z");
        assert_eq!(e["time"], 42);
        assert_eq!(e["request"]["queryString"], json!([{"name": "next", "value": "/home"}, {"name": "x", "value": "1"}]));
        assert_eq!(e["request"]["cookies"], json!([{"name": "a", "value": "1"}, {"name": "b", "value": "two"}]));
        assert_eq!(e["request"]["postData"]["text"], r#"{"u":"me"}"#);
        assert_eq!(e["request"]["postData"]["mimeType"], "application/json");
        assert_eq!(e["response"]["statusText"], "Found");
        assert_eq!(e["response"]["redirectURL"], "/home");
        assert_eq!(
            e["response"]["cookies"],
            json!([{"name": "sid", "value": "abc", "path": "/", "httpOnly": true, "secure": true}])
        );
        assert_eq!(e["response"]["content"]["text"], "redirecting!");
        assert_eq!(e["response"]["content"]["size"], 12);
    }

    #[test]
    fn binary_bodies_stay_base64() {
        let mut flow = entry();
        flow.request_body = "base64:AAEC".into();
        flow.response_body = "base64:/9j/".into();
        let har = build_har(&[flow]);
        let e = first(&har);
        assert_eq!(e["request"]["postData"]["text"], "AAEC");
        assert_eq!(e["request"]["postData"]["_encoding"], "base64");
        assert_eq!(e["request"]["bodySize"], 3);
        assert_eq!(e["response"]["content"]["encoding"], "base64");
        assert_eq!(e["response"]["content"]["size"], 3);
    }

    #[test]
    fn failed_flows_have_status_zero_and_request_at_wins() {
        let mut flow = entry();
        flow.status_code = 0;
        flow.request_body.clear();
        flow.response_body.clear();
        flow.response_headers.clear();
        flow.request_at = Some("2026-10-11T08:30:05.123Z".into());
        let har = build_har(&[flow]);
        let e = first(&har);
        assert_eq!(e["startedDateTime"], "2026-10-11T08:30:05.123Z");
        assert_eq!(e["response"]["status"], 0);
        assert_eq!(e["response"]["statusText"], "");
        assert_eq!(e["response"]["bodySize"], -1);
        assert!(e["request"].get("postData").is_none());
        assert!(e["response"]["content"].get("text").is_none());
    }
}
