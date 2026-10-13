use std::borrow::Cow;
use mitm_core::proxy::history::{HistoryDetail, HistoryEntry};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use super::format::{format_bytes, headers_to_text};

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

#[derive(Debug, Clone)]
pub enum CopyFormat {
    CurlAndResponse,
    UrlBodyAndResponse,
    Url,
    UrlAndBody,
    RequestHeaders,
    RequestBody,
    Curl,
    Request,
    ResponseBody,
    ResponseHeaders,
    Response,
    Custom(CopyParts),
}

/// What a Custom copy includes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CopyParts {
    pub request_headers: bool,
    pub request_body: bool,
    pub response_status: bool,
    pub status_text: bool,
    pub response_headers: bool,
    pub response_body: bool,
}

impl Default for CopyParts {
    fn default() -> Self {
        Self {
            request_headers: true,
            request_body: true,
            response_status: true,
            status_text: false,
            response_headers: false,
            response_body: true,
        }
    }
}

/// How every copy is formatted, whatever is copied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CopySettings {
    pub markdown_fence: bool,
    pub pretty_json: bool,
    pub truncate: bool,
    pub max_chars: usize,
    pub show_truncated_size: bool,
}

impl Default for CopySettings {
    fn default() -> Self {
        Self { markdown_fence: false, pretty_json: true, truncate: false, max_chars: 200, show_truncated_size: true }
    }
}

#[derive(Debug, Clone)]
pub struct CopyAction {
    pub format: CopyFormat,
    pub settings: CopySettings,
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

    pub fn from_fuzz_result(result: &mitm_core::fuzzer::FuzzResult) -> Self {
        let content_type = result
            .response_headers
            .iter()
            .find(|h| h.key.eq_ignore_ascii_case("content-type"))
            .map_or(String::new(), |h| h.value.clone());
        Self {
            method: result.method.clone(),
            url: result.url.clone(),
            request_headers: result
                .request_headers
                .iter()
                .filter(|h| h.enabled)
                .map(|h| (h.key.clone(), h.value.clone()))
                .collect(),
            request_body: result.request_body.clone().unwrap_or_default(),
            status_code: result.status_code,
            response_headers: result
                .response_headers
                .iter()
                .map(|h| (h.key.clone(), h.value.clone()))
                .collect(),
            response_body: result.response_body.clone().unwrap_or_default(),
            content_type,
        }
    }

    /// A request with no response yet, e.g. as it stands in an editor.
    pub fn request_only(method: String, url: String, request_headers: Vec<(String, String)>, request_body: String) -> Self {
        Self {
            method,
            url,
            request_headers,
            request_body,
            status_code: 0,
            response_headers: Vec::new(),
            response_body: String::new(),
            content_type: String::new(),
        }
    }

    pub fn copy_text(&self, action: &CopyAction) -> String {
        let text = Copier { exchange: self, settings: &action.settings }.text(&action.format);
        if action.settings.markdown_fence {
            format!("```\n{text}\n```")
        } else {
            text
        }
    }
}

struct Copier<'a> {
    exchange: &'a Exchange,
    settings: &'a CopySettings,
}

impl Copier<'_> {
    fn text(&self, format: &CopyFormat) -> String {
        match format {
            CopyFormat::Url => self.exchange.url.clone(),
            CopyFormat::UrlAndBody => self.url_and_body(),
            CopyFormat::RequestHeaders => headers_to_text(&self.request_headers()),
            CopyFormat::RequestBody => self.request_body(),
            CopyFormat::Curl => self.curl(&self.request_headers(), &self.request_body()),
            CopyFormat::Request => raw_message(self.request_line(), &self.request_headers(), &self.request_body()),
            CopyFormat::ResponseBody => self.response_body(),
            CopyFormat::ResponseHeaders => raw_message(self.status_line(), &self.response_headers(), ""),
            CopyFormat::Response => raw_message(self.status_line(), &self.response_headers(), &self.response_body()),
            CopyFormat::CurlAndResponse => {
                format!("{}\n\n{}", self.curl(&self.request_headers(), &self.request_body()), self.response_section())
            }
            CopyFormat::UrlBodyAndResponse => format!("{}\n\n{}", self.url_and_body(), self.response_section()),
            CopyFormat::Custom(parts) => self.custom(parts),
        }
    }

    fn request_line(&self) -> String {
        format!("{} {}", self.exchange.method, self.exchange.url)
    }

    fn reason(&self) -> &'static str {
        http::StatusCode::from_u16(self.exchange.status_code).ok().and_then(|s| s.canonical_reason()).unwrap_or("")
    }

    fn status_line(&self) -> String {
        format!("HTTP/1.1 {} {}", self.exchange.status_code, self.reason()).trim_end().to_string()
    }

    fn request_headers(&self) -> Vec<(String, String)> {
        self.headers(&self.exchange.request_headers)
    }

    fn response_headers(&self) -> Vec<(String, String)> {
        self.headers(&self.exchange.response_headers)
    }

    fn headers(&self, headers: &[(String, String)]) -> Vec<(String, String)> {
        headers.iter().map(|(k, v)| (k.clone(), self.shorten(v).into_owned())).collect()
    }

    fn request_body(&self) -> String {
        let content_type = self
            .exchange
            .request_headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map_or("", |(_, v)| v.as_str());
        self.body(&self.exchange.request_body, content_type)
    }

    fn response_body(&self) -> String {
        self.body(&self.exchange.response_body, &self.exchange.content_type)
    }

    fn url_and_body(&self) -> String {
        let mut text = self.request_line();
        let body = self.request_body();
        if !body.trim().is_empty() {
            text.push('\n');
            text.push_str(&body);
        }
        text
    }

    fn response_section(&self) -> String {
        let code = if self.exchange.status_code == 0 { 200 } else { self.exchange.status_code };
        let body = self.response_body();
        if body.trim().is_empty() {
            code.to_string()
        } else {
            format!("{code}\n{body}")
        }
    }

    fn curl(&self, headers: &[(String, String)], body: &str) -> String {
        let mut curl = format!("curl -X {} '{}'", self.exchange.method, self.exchange.url);
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

    fn custom(&self, parts: &CopyParts) -> String {
        let headers = if parts.request_headers { self.request_headers() } else { Vec::new() };
        let body = if parts.request_body { self.request_body() } else { String::new() };
        let mut text = self.curl(&headers, body.trim());

        let mut response = Vec::new();
        let code = self.exchange.status_code;
        match (parts.response_status, parts.status_text) {
            (true, true) => response.push(self.status_line()),
            (true, false) => response.push(format!("HTTP/1.1 {code}")),
            (false, true) => response.push(self.reason().to_string()),
            (false, false) => {}
        }
        if parts.response_headers {
            response.push(headers_to_text(&self.response_headers()));
        }
        let body = if parts.response_body { self.response_body() } else { String::new() };
        if !body.trim().is_empty() {
            if !response.is_empty() {
                response.push(String::new());
            }
            response.push(body);
        }
        if !response.is_empty() {
            text.push_str("\n\n");
            text.push_str(&response.join("\n"));
        }
        text
    }

    /// Captured binary bodies are stored as `base64:<data>`.
    fn body(&self, body: &str, content_type: &str) -> String {
        let truncate = self.settings.truncate;
        let boundary = multipart_boundary(content_type).filter(|_| truncate);
        match body.strip_prefix("base64:") {
            Some(b64) => {
                let bytes = boundary.as_ref().and_then(|_| super::format::decode_base64(b64));
                match (boundary, bytes) {
                    (Some(boundary), Some(bytes)) => self.multipart(&bytes, &boundary),
                    _ if truncate => match split_at_chars(b64, self.settings.max_chars) {
                        Some((kept, rest)) => format!("{kept}{}", self.marker(rest.len() as u64 * 3 / 4)),
                        None => b64.to_string(),
                    },
                    _ => b64.to_string(),
                }
            }
            None => match boundary {
                Some(boundary) => self.multipart(body.as_bytes(), &boundary),
                None => self.text_body(body),
            },
        }
    }

    fn text_body(&self, body: &str) -> String {
        let settings = self.settings;
        if settings.pretty_json || settings.truncate {
            if let Ok(mut value) = serde_json::from_str::<Value>(body) {
                if settings.truncate {
                    self.shorten_json(&mut value);
                }
                let text = if settings.pretty_json { serde_json::to_string_pretty(&value) } else { serde_json::to_string(&value) };
                if let Ok(text) = text {
                    return text;
                }
            }
        }
        if settings.truncate {
            body.lines().map(|line| self.shorten(line)).collect::<Vec<_>>().join("\n")
        } else {
            body.to_string()
        }
    }

    fn shorten_json(&self, value: &mut Value) {
        match value {
            Value::String(text) => {
                if let Some(short) = self.cut(text) {
                    *text = short;
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|item| self.shorten_json(item)),
            Value::Object(map) => map.values_mut().for_each(|item| self.shorten_json(item)),
            _ => {}
        }
    }

    /// Each part keeps its own headers; a long text value or any binary file is cut.
    fn multipart(&self, bytes: &[u8], boundary: &str) -> String {
        let delimiter = format!("--{boundary}");
        let parts: Vec<String> = split_bytes(bytes, delimiter.as_bytes()).into_iter().map(|part| self.multipart_part(part)).collect();
        parts.join(&delimiter).replace("\r\n", "\n")
    }

    fn multipart_part(&self, part: &[u8]) -> String {
        let Some(split) = find_bytes(part, b"\r\n\r\n") else {
            return String::from_utf8_lossy(part).into_owned();
        };
        let head = String::from_utf8_lossy(&part[..split]);
        let content = &part[split + 4..];
        let (content, tail) = match content.strip_suffix(b"\r\n") {
            Some(content) => (content, "\r\n"),
            None => (content, ""),
        };
        let content = match std::str::from_utf8(content) {
            Ok(text) => self.shorten(text).into_owned(),
            Err(_) => self.marker(content.len() as u64),
        };
        format!("{head}\r\n\r\n{content}{tail}")
    }

    fn shorten<'t>(&self, text: &'t str) -> Cow<'t, str> {
        self.cut(text).map_or(Cow::Borrowed(text), Cow::Owned)
    }

    fn cut(&self, text: &str) -> Option<String> {
        if !self.settings.truncate {
            return None;
        }
        let (kept, rest) = split_at_chars(text, self.settings.max_chars)?;
        Some(format!("{kept}{}", self.marker(rest.len() as u64)))
    }

    fn marker(&self, cut_bytes: u64) -> String {
        if self.settings.show_truncated_size {
            format!("…({})", format_bytes(cut_bytes))
        } else {
            "…".to_string()
        }
    }
}

fn raw_message(first_line: String, headers: &[(String, String)], body: &str) -> String {
    let mut text = first_line;
    if !headers.is_empty() {
        text.push('\n');
        text.push_str(&headers_to_text(headers));
    }
    if !body.trim().is_empty() {
        text.push_str("\n\n");
        text.push_str(body);
    }
    text
}

fn shell_quote(text: &str) -> String {
    text.replace('\'', "'\\''")
}

fn split_at_chars(text: &str, max_chars: usize) -> Option<(&str, &str)> {
    text.char_indices().nth(max_chars).map(|(at, _)| text.split_at(at))
}

fn multipart_boundary(content_type: &str) -> Option<String> {
    if !content_type.trim_start().to_ascii_lowercase().starts_with("multipart/") {
        return None;
    }
    content_type
        .split(';')
        .filter_map(|param| param.trim().split_once('='))
        .find(|(key, _)| key.trim().eq_ignore_ascii_case("boundary"))
        .map(|(_, value)| value.trim().trim_matches('"').to_string())
        .filter(|boundary| !boundary.is_empty())
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn split_bytes<'b>(mut bytes: &'b [u8], delimiter: &[u8]) -> Vec<&'b [u8]> {
    let mut parts = Vec::new();
    while let Some(at) = find_bytes(bytes, delimiter) {
        parts.push(&bytes[..at]);
        bytes = &bytes[at + delimiter.len()..];
    }
    parts.push(bytes);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Exchange {
        Exchange {
            method: "POST".to_string(),
            url: "https://api.test/u".to_string(),
            request_headers: vec![("X-A".to_string(), "1".to_string())],
            request_body: r#"{"a":1}"#.to_string(),
            status_code: 404,
            response_headers: vec![("Server".to_string(), "t".to_string())],
            response_body: String::new(),
            content_type: "text/plain".to_string(),
        }
    }

    fn copy(exchange: &Exchange, format: CopyFormat, settings: CopySettings) -> String {
        exchange.copy_text(&CopyAction { format, settings })
    }

    fn truncating(max_chars: usize) -> CopySettings {
        CopySettings { truncate: true, max_chars, ..CopySettings::default() }
    }

    #[test]
    fn custom_copy_includes_only_the_chosen_parts() {
        let exchange = Exchange { response_body: "missing".to_string(), ..sample() };
        let all = CopyParts { status_text: true, response_headers: true, ..CopyParts::default() };
        let text = copy(&exchange, CopyFormat::Custom(all), CopySettings::default());
        assert!(text.contains("-H 'X-A: 1'"));
        assert!(text.contains("\"a\": 1"));
        assert!(text.ends_with("HTTP/1.1 404 Not Found\nServer: t\n\nmissing"));

        let bare = CopyParts {
            request_headers: false,
            request_body: false,
            response_status: false,
            response_body: false,
            ..CopyParts::default()
        };
        assert_eq!(copy(&exchange, CopyFormat::Custom(bare), CopySettings::default()), "curl -X POST 'https://api.test/u'");
    }

    #[test]
    fn request_and_response_copy_as_raw_messages() {
        let exchange = sample();
        let settings = CopySettings::default;
        assert_eq!(copy(&exchange, CopyFormat::Request, settings()), "POST https://api.test/u\nX-A: 1\n\n{\n  \"a\": 1\n}");
        assert_eq!(copy(&exchange, CopyFormat::Response, settings()), "HTTP/1.1 404 Not Found\nServer: t");
        assert_eq!(copy(&exchange, CopyFormat::UrlAndBody, settings()), "POST https://api.test/u\n{\n  \"a\": 1\n}");
        assert_eq!(copy(&exchange, CopyFormat::RequestHeaders, settings()), "X-A: 1");
    }

    #[test]
    fn markdown_fence_and_raw_json_apply_to_every_format() {
        let exchange = sample();
        let settings = CopySettings { markdown_fence: true, pretty_json: false, ..CopySettings::default() };
        assert_eq!(copy(&exchange, CopyFormat::RequestBody, settings.clone()), "```\n{\"a\":1}\n```");
        assert_eq!(copy(&exchange, CopyFormat::Url, settings), "```\nhttps://api.test/u\n```");
    }

    #[test]
    fn truncation_cuts_long_header_values_and_json_strings_with_the_cut_size() {
        let token = "x".repeat(1034);
        let exchange = Exchange {
            request_headers: vec![("Authorization".to_string(), token.clone())],
            response_body: format!(r#"{{"token":"{token}","n":1}}"#),
            status_code: 200,
            ..sample()
        };
        assert_eq!(copy(&exchange, CopyFormat::RequestHeaders, truncating(10)), "Authorization: xxxxxxxxxx…(1.0 KB)");
        let body = copy(&exchange, CopyFormat::ResponseBody, CopySettings { pretty_json: false, ..truncating(4) });
        assert_eq!(body, r#"{"token":"xxxx…(1.0 KB)","n":1}"#);
        let quiet = CopySettings { show_truncated_size: false, ..truncating(4) };
        assert_eq!(copy(&exchange, CopyFormat::RequestHeaders, quiet), "Authorization: xxxx…");
        assert!(copy(&exchange, CopyFormat::RequestHeaders, CopySettings::default()).ends_with(&token));
    }

    #[test]
    fn truncation_cuts_binary_bodies_and_multipart_files() {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD;
        let exchange = Exchange { response_body: format!("base64:{}", b64.encode([0u8; 300])), ..sample() };
        assert_eq!(copy(&exchange, CopyFormat::ResponseBody, truncating(8)), "AAAAAAAA…(294 B)");

        let mut form = b"--XB\r\nContent-Disposition: form-data; name=\"note\"\r\n\r\nhello\r\n".to_vec();
        form.extend_from_slice(b"--XB\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.png\"\r\n\r\n");
        form.extend_from_slice(&[0xff; 2048]);
        form.extend_from_slice(b"\r\n--XB--\r\n");
        let exchange = Exchange {
            request_headers: vec![("Content-Type".to_string(), "multipart/form-data; boundary=XB".to_string())],
            request_body: format!("base64:{}", b64.encode(&form)),
            ..sample()
        };
        assert_eq!(
            copy(&exchange, CopyFormat::RequestBody, truncating(100)),
            "--XB\nContent-Disposition: form-data; name=\"note\"\n\nhello\n\
             --XB\nContent-Disposition: form-data; name=\"file\"; filename=\"a.png\"\n\n…(2.0 KB)\n--XB--\n"
        );
    }

    #[test]
    fn fuzz_result_formats_as_curl() {
        use mitm_core::fuzzer::FuzzResult;
        use mitm_core::repeater::HeaderItem;

        let header = |id: &str, key: &str, value: &str, enabled| HeaderItem {
            id: id.to_string(),
            key: key.to_string(),
            value: value.to_string(),
            enabled,
        };
        let result = FuzzResult {
            idx: 0,
            payloads: vec!["admin".to_string()],
            method: "POST".to_string(),
            url: "https://example.com/login".to_string(),
            request_headers: vec![header("1", "Content-Type", "application/json", true), header("2", "Disabled", "val", false)],
            request_body: Some(r#"{"user":"admin"}"#.to_string()),
            status_code: 200,
            response_headers: vec![header("3", "Content-Type", "application/json", true)],
            response_body: Some(r#"{"token":"xyz"}"#.to_string()),
            response_size: 15,
            duration_ms: 42,
            error: None,
            matches: vec![],
        };
        let exchange = Exchange::from_fuzz_result(&result);
        let curl = copy(&exchange, CopyFormat::Curl, CopySettings { pretty_json: false, ..CopySettings::default() });
        assert!(curl.starts_with("curl -X POST 'https://example.com/login'"));
        assert!(curl.contains("-H 'Content-Type: application/json'"));
        assert!(!curl.contains("Disabled"));
        assert!(curl.contains(r#"--data-raw '{"user":"admin"}'"#));
    }
}
