use serde::Deserialize;

use crate::workspace::interpolate_variables;
use crate::db::Db;

#[derive(Debug, Clone)]
pub struct RequestPreview {
    pub full_url_request: String,
    pub full_request: String,
    pub curl_command: String,
}

#[derive(Deserialize)]
struct Row {
    key: String,
    value: String,
    #[serde(default)]
    enabled: bool,
}

struct StoredRequest {
    method: String,
    url: String,
    headers_json: String,
    params_json: String,
    body_type: String,
    body: Option<String>,
    workspace_id: String,
}

fn load(db: &Db, request_id: &str) -> Result<StoredRequest, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT r.method, r.url, r.headers_json, r.params_json, r.body_type,
                r.body_json, r.body_raw, r.body_form_data, r.body_urlencoded, c.workspace_id
         FROM requests r JOIN collections c ON r.collection_id = c.id
         WHERE r.id = ?",
        [request_id],
        |row| {
            let body_type: String = row.get(4)?;
            let body = match body_type.as_str() {
                "json" => row.get(5)?,
                "raw" => row.get(6)?,
                "form-data" | "multipart" => row.get(7)?,
                "urlencoded" | "x-www-form-urlencoded" => row.get(8)?,
                _ => None,
            };
            Ok(StoredRequest {
                method: row.get(0)?,
                url: row.get(1)?,
                headers_json: row.get(2)?,
                params_json: row.get(3)?,
                body_type,
                body,
                workspace_id: row.get(9)?,
            })
        },
    )
    .map_err(|e| format!("Request not found: {}", e))
}

fn url_encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect()
}

fn active_rows(json: &str) -> Vec<Row> {
    serde_json::from_str::<Vec<Row>>(json)
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.enabled && !r.key.trim().is_empty())
        .collect()
}

pub fn preview_request_db(db: &Db, request_id: &str) -> Result<RequestPreview, String> {
    let stored = load(db, request_id)?;
    let fill = |text: &str| interpolate_variables(db, &stored.workspace_id, text);
    let method = stored.method.to_uppercase();

    let query: Vec<String> = active_rows(&stored.params_json)
        .iter()
        .map(|p| format!("{}={}", url_encode(&fill(&p.key)), url_encode(&fill(&p.value))))
        .collect();
    let mut url = fill(&stored.url);
    if !query.is_empty() {
        let joiner = if url.contains('?') { "&" } else { "?" };
        url = format!("{}{}{}", url, joiner, query.join("&"));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        url = format!("https://{}", url);
    }

    let mut headers: Vec<(String, String)> = active_rows(&stored.headers_json)
        .iter()
        .map(|h| (fill(&h.key), fill(&h.value)))
        .collect();
    let has_content_type = headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type"));
    let implied_content_type = match stored.body_type.as_str() {
        "json" => Some("application/json"),
        "urlencoded" | "x-www-form-urlencoded" => Some("application/x-www-form-urlencoded"),
        _ => None,
    };
    if let (false, Some(ct)) = (has_content_type, implied_content_type) {
        headers.push(("Content-Type".to_string(), ct.to_string()));
    }
    let body = stored.body.as_deref().map(fill);

    let without_scheme = url.trim_start_matches("https://").trim_start_matches("http://");
    let (host, path) = without_scheme.find('/').map_or((without_scheme, "/"), |i| (&without_scheme[..i], &without_scheme[i..]));

    let header_block: String = headers.iter().map(|(k, v)| format!("{}: {}\r\n", k, v)).collect();
    let body_text = body.clone().unwrap_or_default();
    let full_url_request = format!("{} {} HTTP/1.1\r\n{}\r\n{}", method, url, header_block, body_text);
    let full_request = format!("{} {} HTTP/1.1\r\nHost: {}\r\n{}\r\n{}", method, path, host, header_block, body_text);

    let mut curl = vec![format!("curl -X {}", method), format!("  '{}'", url)];
    curl.extend(headers.iter().map(|(k, v)| format!("  -H '{}: {}'", k, v)));
    if let Some(body) = &body {
        let escaped = body.replace('\'', "'\\''");
        let flag = match stored.body_type.as_str() {
            "urlencoded" | "x-www-form-urlencoded" => "--data-urlencode",
            _ => "-d",
        };
        curl.push(format!("  {} '{}'", flag, escaped));
    }

    Ok(RequestPreview {
        full_url_request,
        full_request,
        curl_command: curl.join(" \\\n"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_request_with_params_and_implied_content_type() {
        let dir = std::env::temp_dir().join(format!("mitm-preview-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = crate::db::Db::new(dir.join("test.db"));
        let conn = crate::db::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE collections (id TEXT PRIMARY KEY, workspace_id TEXT);
             CREATE TABLE requests (id TEXT PRIMARY KEY, collection_id TEXT, method TEXT, url TEXT,
                headers_json TEXT, params_json TEXT, body_type TEXT, body_json TEXT, body_raw TEXT,
                body_form_data TEXT, body_urlencoded TEXT);
             INSERT INTO collections VALUES ('c1', 'w1');
             INSERT INTO requests VALUES ('r1', 'c1', 'post', 'api.example.com/users',
                '[{\"key\":\"X-Trace\",\"value\":\"abc\",\"enabled\":true},{\"key\":\"X-Off\",\"value\":\"no\",\"enabled\":false}]',
                '[{\"key\":\"q\",\"value\":\"a b\",\"enabled\":true}]',
                'json', '{\"name\":\"x\"}', NULL, NULL, NULL);",
        )
        .unwrap();
        drop(conn);

        let preview = preview_request_db(&db, "r1").unwrap();
        assert!(preview.full_url_request.starts_with("POST https://api.example.com/users?q=a%20b HTTP/1.1\r\n"));
        assert!(preview.full_request.starts_with("POST /users?q=a%20b HTTP/1.1\r\nHost: api.example.com\r\n"));
        assert!(preview.full_request.contains("X-Trace: abc\r\nContent-Type: application/json\r\n"));
        assert!(!preview.full_request.contains("X-Off"));
        assert!(preview.curl_command.ends_with("-d '{\"name\":\"x\"}'"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
