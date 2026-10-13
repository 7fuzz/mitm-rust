pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// 12345 → "12,345"
pub fn format_count(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
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

pub fn display_body(body: &str, content_type: &str) -> String {
    if let Some(b64) = body.strip_prefix("base64:") {
        let mime = if content_type.is_empty() { "application/octet-stream" } else { content_type };
        return format!("[Binary Content ({}): {} chars base64]", mime, b64.len());
    }
    pretty_json(body)
        .or_else(|| super::markup::pretty(body, content_type))
        .unwrap_or_else(|| body.to_string())
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
    fn counts_group_thousands() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1000), "1,000");
        assert_eq!(format_count(1234567), "1,234,567");
    }

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
}
