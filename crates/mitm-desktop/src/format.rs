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
            response_body: detail.map_or(&flow.response_body, |d| &d.response_body).clone(),
            content_type: flow.content_type.clone(),
        }
    }

    pub fn curl(&self) -> String {
        let mut curl = format!("curl -X {} '{}'", self.method, self.url);
        for (k, v) in &self.request_headers {
            if !k.eq_ignore_ascii_case("content-length") {
                curl.push_str(&format!(" \\\n  -H '{}: {}'", k, shell_quote(v)));
            }
        }
        if !self.request_body.is_empty() {
            curl.push_str(&format!(" \\\n  --data-raw '{}'", shell_quote(&self.request_body)));
        }
        curl
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

pub fn local_clock(rfc3339: Option<&str>) -> Option<(String, String)> {
    let at = chrono::DateTime::parse_from_rfc3339(rfc3339?).ok()?.with_timezone(&chrono::Local);
    Some((at.format("%H:%M:%S%.3f").to_string(), at.format("%Y-%m-%d %H:%M:%S%.3f").to_string()))
}
