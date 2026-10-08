use mitm_core::state::{HistoryDetail, HistoryEntry};

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn format_duration(ms: Option<u64>) -> String {
    match ms {
        Some(m) if m < 1000 => format!("{}ms", m),
        Some(m) => format!("{:.2}s", m as f64 / 1000.0),
        None => "-".to_string(),
    }
}

pub fn headers_to_text(headers: &[(String, String)]) -> String {
    headers
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse_header_lines(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            let key = key.trim();
            (!key.is_empty()).then(|| (key.to_string(), value.trim().to_string()))
        })
        .collect()
}

pub fn parse_header_items(text: &str) -> Vec<mitm_core::repeater::HeaderItem> {
    parse_header_lines(text)
        .into_iter()
        .map(|(key, value)| mitm_core::repeater::HeaderItem {
            id: uuid::Uuid::new_v4().to_string(),
            key,
            value,
            enabled: true,
        })
        .collect()
}

pub fn pretty_json(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| serde_json::to_string_pretty(&v).ok())
}

pub fn hex_dump(bytes: &[u8]) -> String {
    let mut output = String::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        output.push_str(&format!("{:08x}:  ", i * 16));

        for j in 0..16 {
            if j == 8 {
                output.push(' ');
            }
            if j < chunk.len() {
                output.push_str(&format!("{:02x} ", chunk[j]));
            } else {
                output.push_str("   ");
            }
        }

        output.push_str(" |");
        for &b in chunk {
            if b.is_ascii_graphic() || b == b' ' {
                output.push(b as char);
            } else {
                output.push('.');
            }
        }
        output.push_str("|\n");
    }
    output
}

pub struct Exchange {
    pub method: String,
    pub url: String,
    pub request_headers: Vec<(String, String)>,
    pub request_body: String,
    pub status_code: u16,
    pub response_headers: Vec<(String, String)>,
    pub response_body: String,
    pub content_type: String,
}

impl Exchange {
    pub fn from_flow(flow: &HistoryEntry, detail: Option<&HistoryDetail>) -> Self {
        Self {
            method: flow.method.clone(),
            url: flow.url.clone(),
            request_headers: detail.map_or(&flow.request_headers, |d| &d.request_headers).clone(),
            request_body: detail.map_or(&flow.request_body, |d| &d.request_body).clone(),
            status_code: flow.status_code,
            response_headers: detail.map_or(&flow.response_headers, |d| &d.response_headers).clone(),
            response_body: detail.map_or(&flow.response_body, |d| &d.response_body).clone(),
            content_type: flow.content_type.clone(),
        }
    }

    pub fn curl(&self) -> String {
        self.curl_with(&self.request_headers, &self.request_body)
    }

    fn curl_with(&self, headers: &[(String, String)], body: &str) -> String {
        let mut curl = format!("curl -X {} '{}'", self.method, self.url);
        for (k, v) in headers {
            if !k.eq_ignore_ascii_case("content-length") {
                curl.push_str(&format!(" \\\n  -H '{}: {}'", k, shell_quote(v)));
            }
        }
        if !body.is_empty() {
            curl.push_str(&format!(" \\\n  --data-raw '{}'", shell_quote(body)));
        }
        curl
    }

    pub fn custom_text(&self, options: &CopyOptions) -> String {
        let pretty = |body: &str, content_type: &str| {
            options.pretty_json.then(|| pretty_json(body)).flatten().unwrap_or_else(|| display_body(body, content_type))
        };
        let headers = if options.request_headers { self.request_headers.as_slice() } else { &[] };
        let body = if options.request_body { pretty(&self.request_body, "") } else { String::new() };
        let mut text = self.curl_with(headers, body.trim());

        let mut response = Vec::new();
        let reason = || http::StatusCode::from_u16(self.status_code).ok().and_then(|s| s.canonical_reason()).unwrap_or("");
        match (options.response_status, options.status_text) {
            (true, true) => response.push(format!("HTTP/1.1 {} {}", self.status_code, reason()).trim_end().to_string()),
            (true, false) => response.push(format!("HTTP/1.1 {}", self.status_code)),
            (false, true) => response.push(reason().to_string()),
            (false, false) => {}
        }
        if options.response_headers {
            response.extend(self.response_headers.iter().map(|(k, v)| format!("{k}: {v}")));
        }
        if options.response_body && !self.response_body.trim().is_empty() {
            if !response.is_empty() {
                response.push(String::new());
            }
            response.push(pretty(&self.response_body, &self.content_type));
        }
        if !response.is_empty() {
            text.push_str("\n\n");
            text.push_str(&response.join("\n"));
        }
        text
    }

    pub fn copy_text(&self, format: CopyFormat) -> String {
        match format {
            CopyFormat::Curl => self.curl(),
            CopyFormat::CurlAndResponse => format!("{}\n\n{}", self.curl(), self.response_section()),
            CopyFormat::UrlBodyAndResponse => {
                let mut request = format!("{} {}", self.method, self.url);
                if !self.request_body.trim().is_empty() {
                    request.push('\n');
                    request.push_str(&display_body(&self.request_body, ""));
                }
                format!("{}\n\n{}", request, self.response_section())
            }
        }
    }

    fn response_section(&self) -> String {
        let code = if self.status_code == 0 { 200 } else { self.status_code };
        if self.response_body.trim().is_empty() {
            code.to_string()
        } else {
            format!("{}\n{}", code, display_body(&self.response_body, &self.content_type))
        }
    }
}

fn shell_quote(text: &str) -> String {
    text.replace('\'', "'\\''")
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CopyOptions {
    pub request_headers: bool,
    pub request_body: bool,
    pub response_status: bool,
    pub status_text: bool,
    pub response_headers: bool,
    pub response_body: bool,
    pub pretty_json: bool,
}

impl Default for CopyOptions {
    fn default() -> Self {
        Self {
            request_headers: true,
            request_body: true,
            response_status: true,
            status_text: false,
            response_headers: false,
            response_body: true,
            pretty_json: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CopyFormat {
    Curl,
    CurlAndResponse,
    UrlBodyAndResponse,
}

pub fn display_body(body: &str, content_type: &str) -> String {
    if let Some(b64) = body.strip_prefix("base64:") {
        let mime = if content_type.is_empty() { "application/octet-stream" } else { content_type };
        return format!("[Binary Content ({}): {} chars base64]", mime, b64.len());
    }
    pretty_json(body).unwrap_or_else(|| body.to_string())
}

pub fn body_bytes(body: &str) -> Vec<u8> {
    use base64::Engine;
    body.strip_prefix("base64:")
        .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok())
        .unwrap_or_else(|| body.as_bytes().to_vec())
}

pub fn looks_like_base64(text: &str) -> bool {
    let text = strip_data_url(text.trim());
    text.len() >= 4 && text.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'-' | b'_' | b'='))
}

pub fn decode_base64(text: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
    let text = strip_data_url(text.trim());
    [STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD]
        .iter()
        .find_map(|engine| engine.decode(text).ok())
        .filter(|bytes| !bytes.is_empty())
}

fn strip_data_url(text: &str) -> &str {
    text.strip_prefix("data:")
        .and_then(|rest| rest.split_once(";base64,"))
        .map_or(text, |(_, data)| data)
}

pub fn local_clock(rfc3339: Option<&str>) -> Option<(String, String)> {
    let at = chrono::DateTime::parse_from_rfc3339(rfc3339?).ok()?.with_timezone(&chrono::Local);
    Some((at.format("%H:%M:%S%.3f").to_string(), at.format("%Y-%m-%d %H:%M:%S%.3f").to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_padded_unpadded_url_safe_and_data_urls() {
        assert_eq!(decode_base64("aGk=").as_deref(), Some(&b"hi"[..]));
        assert_eq!(decode_base64("aGk").as_deref(), Some(&b"hi"[..]));
        assert_eq!(decode_base64("_-8").as_deref(), Some(&[0xff, 0xef][..]));
        assert_eq!(decode_base64("data:text/plain;base64,aGk=").as_deref(), Some(&b"hi"[..]));
        assert_eq!(decode_base64("not base64!"), None);
    }

    #[test]
    fn base64_shape_check_rejects_prose() {
        assert!(looks_like_base64("data:image/png;base64,iVBORw0KGgo="));
        assert!(!looks_like_base64("hello world"));
        assert!(!looks_like_base64("ab"));
    }

    #[test]
    fn custom_copy_includes_only_the_chosen_parts() {
        let exchange = Exchange {
            method: "POST".to_string(),
            url: "https://api.test/u".to_string(),
            request_headers: vec![("X-A".to_string(), "1".to_string())],
            request_body: r#"{"a":1}"#.to_string(),
            status_code: 404,
            response_headers: vec![("Server".to_string(), "t".to_string())],
            response_body: "missing".to_string(),
            content_type: "text/plain".to_string(),
        };
        let all = CopyOptions { status_text: true, response_headers: true, ..CopyOptions::default() };
        let text = exchange.custom_text(&all);
        assert!(text.contains("-H 'X-A: 1'"));
        assert!(text.contains("\"a\": 1"));
        assert!(text.ends_with("HTTP/1.1 404 Not Found\nServer: t\n\nmissing"));

        let bare = CopyOptions {
            request_headers: false,
            request_body: false,
            response_status: false,
            response_body: false,
            ..CopyOptions::default()
        };
        assert_eq!(exchange.custom_text(&bare), "curl -X POST 'https://api.test/u'");
    }
}
