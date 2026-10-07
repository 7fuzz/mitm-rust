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

pub fn curl_command(flow: &HistoryEntry, detail: Option<&HistoryDetail>) -> String {
    let mut curl = format!("curl -X {} '{}'", flow.method, flow.url);
    if let Some(detail) = detail {
        for (k, v) in &detail.request_headers {
            if !k.eq_ignore_ascii_case("content-length") {
                curl.push_str(&format!(" \\\n  -H '{}: {}'", k, v.replace('\'', "'\\''")));
            }
        }
        if !detail.request_body.is_empty() {
            curl.push_str(&format!(" \\\n  --data-raw '{}'", detail.request_body.replace('\'', "'\\''")));
        }
    }
    curl
}
