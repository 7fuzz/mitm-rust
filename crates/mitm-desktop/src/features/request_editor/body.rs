use base64::Engine;
use mitm_core::encoding::{MultipartBodySpec, MultipartFieldSpec, UrlEncodedParamSpec};
use crate::format::pretty_json;
use super::url_params::{decode, new_id};

pub const REPEATER_BODY_TYPES: [(&str, &str); 6] = [
    ("none", "None"),
    ("json", "JSON"),
    ("raw", "Raw"),
    ("form", "Multipart"),
    ("urlencoded", "URL-Encoded"),
    ("binary", "Binary"),
];

pub const COLLECTION_BODY_TYPES: [(&str, &str); 5] = [
    ("none", "none"),
    ("json", "json"),
    ("raw", "raw"),
    ("form-data", "form-data"),
    ("urlencoded", "x-www-form-urlencoded"),
];

const BINARY_PREFIX: &str = "base64:";

pub fn is_urlencoded(body_type: &str) -> bool {
    matches!(body_type, "urlencoded" | "x-www-form-urlencoded")
}

pub fn is_multipart(body_type: &str) -> bool {
    matches!(body_type, "form" | "form-data" | "multipart")
}

pub fn content_type_for(body_type: &str) -> Option<&'static str> {
    match body_type {
        "none" => None,
        t if is_urlencoded(t) => Some("application/x-www-form-urlencoded"),
        t if is_multipart(t) => Some("multipart/form-data"),
        "raw" => Some("text/plain"),
        "binary" => Some("application/octet-stream"),
        _ => Some("application/json"),
    }
}

pub fn read_urlencoded(content: &str) -> Vec<UrlEncodedParamSpec> {
    serde_json::from_str(content).unwrap_or_default()
}

pub fn store_urlencoded(rows: &[UrlEncodedParamSpec]) -> String {
    serde_json::to_string_pretty(rows).unwrap_or_else(|_| "[]".to_string())
}

pub fn read_multipart(content: &str) -> Vec<MultipartFieldSpec> {
    serde_json::from_str::<MultipartBodySpec>(content)
        .map(|spec| spec.__form_data)
        .or_else(|_| serde_json::from_str::<Vec<MultipartFieldSpec>>(content))
        .unwrap_or_default()
}

pub fn store_multipart(fields: &[MultipartFieldSpec]) -> String {
    let spec = MultipartBodySpec { __form_data: fields.to_vec() };
    serde_json::to_string_pretty(&spec).unwrap_or_else(|_| "{\"__form_data\":[]}".to_string())
}

pub fn urlencoded_row(key: String, value: String) -> UrlEncodedParamSpec {
    UrlEncodedParamSpec { id: Some(new_id()), enabled: true, key, value }
}

pub fn text_field(key: String, value: String) -> MultipartFieldSpec {
    MultipartFieldSpec {
        id: Some(new_id()),
        enabled: true,
        key,
        value,
        r#type: "text".to_string(),
        file_name: None,
        content_type: None,
    }
}

pub fn structured_from_capture(content_type: &str, body: &str) -> (String, String) {
    let ct = content_type.to_lowercase();
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return ("none".into(), String::new());
    }
    if let Some(fields) = multipart_boundary(content_type, body).and_then(|b| parse_multipart(body, &b)) {
        return ("form".into(), store_multipart(&fields));
    }
    if body.starts_with(BINARY_PREFIX) {
        return ("binary".into(), body.to_string());
    }
    if ct.contains("x-www-form-urlencoded") {
        return ("urlencoded".into(), store_urlencoded(&parse_urlencoded(trimmed)));
    }
    let json_shaped = ct.contains("json") || trimmed.starts_with('{') || trimmed.starts_with('[');
    if let Some(pretty) = pretty_json(trimmed).filter(|_| json_shaped) {
        return ("json".into(), pretty);
    }
    if !ct.contains("xml") && !trimmed.starts_with('<') && looks_urlencoded(trimmed) {
        return ("urlencoded".into(), store_urlencoded(&parse_urlencoded(trimmed)));
    }
    ("raw".into(), body.to_string())
}

pub fn convert(from: &str, to: &str, content: &str) -> String {
    if from == to || content.trim().is_empty() {
        return content.to_string();
    }
    if is_urlencoded(to) {
        if !read_urlencoded(content).is_empty() {
            return content.to_string();
        }
        let rows = if is_multipart(from) {
            read_multipart(content)
                .into_iter()
                .filter(|f| f.r#type == "text")
                .map(|f| urlencoded_row(f.key, f.value))
                .collect()
        } else if let Some(pairs) = json_object_pairs(content) {
            pairs.into_iter().map(|(k, v)| urlencoded_row(k, v)).collect()
        } else if looks_urlencoded(content.trim()) {
            parse_urlencoded(content.trim())
        } else {
            Vec::new()
        };
        return store_urlencoded(&rows);
    }
    if is_multipart(to) {
        if !read_multipart(content).is_empty() {
            return content.to_string();
        }
        let fields = if is_urlencoded(from) {
            read_urlencoded(content).into_iter().map(|r| text_field(r.key, r.value)).collect()
        } else if let Some(pairs) = json_object_pairs(content) {
            pairs.into_iter().map(|(k, v)| text_field(k, v)).collect()
        } else {
            Vec::new()
        };
        return store_multipart(&fields);
    }
    if is_urlencoded(from) {
        let rows: Vec<_> = read_urlencoded(content).into_iter().filter(|r| r.enabled && !r.key.trim().is_empty()).collect();
        return if to == "json" {
            let object: serde_json::Map<String, serde_json::Value> =
                rows.into_iter().map(|r| (r.key, serde_json::Value::String(r.value))).collect();
            serde_json::to_string_pretty(&object).unwrap_or_default()
        } else {
            rows.iter().map(|r| format!("{}={}", r.key, r.value)).collect::<Vec<_>>().join("&")
        };
    }
    if is_multipart(from) {
        return content.to_string();
    }
    if to == "json" {
        return pretty_json(content).unwrap_or_else(|| content.to_string());
    }
    content.to_string()
}

fn json_object_pairs(content: &str) -> Option<Vec<(String, String)>> {
    let serde_json::Value::Object(map) = serde_json::from_str(content).ok()? else {
        return None;
    };
    Some(
        map.into_iter()
            .map(|(k, v)| match v {
                serde_json::Value::String(s) => (k, s),
                other => (k, other.to_string()),
            })
            .collect(),
    )
}

fn looks_urlencoded(text: &str) -> bool {
    !text.is_empty()
        && !text.contains(char::is_whitespace)
        && text.split('&').all(|pair| pair.split_once('=').is_some_and(|(k, _)| !k.is_empty()))
}

fn parse_urlencoded(text: &str) -> Vec<UrlEncodedParamSpec> {
    text.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            urlencoded_row(decode(key), decode(value))
        })
        .collect()
}

fn body_bytes(body: &str) -> Vec<u8> {
    match body.strip_prefix(BINARY_PREFIX) {
        Some(b64) => {
            let compact: String = b64.split_whitespace().collect();
            base64::engine::general_purpose::STANDARD.decode(compact).unwrap_or_default()
        }
        None => body.as_bytes().to_vec(),
    }
}

fn multipart_boundary(content_type: &str, body: &str) -> Option<String> {
    if let Some(pos) = content_type.to_lowercase().find("boundary=") {
        let raw = content_type[pos + 9..].split(';').next().unwrap_or_default();
        return Some(raw.trim().trim_matches('"').to_string()).filter(|b| !b.is_empty());
    }
    let bytes = body_bytes(body);
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]).into_owned();
    let first = head.lines().next()?.strip_prefix("--")?.trim().to_string();
    (!first.is_empty() && head.to_lowercase().contains("content-disposition")).then_some(first)
}

fn parse_multipart(body: &str, boundary: &str) -> Option<Vec<MultipartFieldSpec>> {
    let bytes = body_bytes(body);
    let delimiter = format!("--{}", boundary).into_bytes();
    let mut fields = Vec::new();

    for section in split_bytes(&bytes, &delimiter).into_iter().skip(1) {
        if section.starts_with(b"--") {
            break;
        }
        let part = trim_crlf(section);
        let (raw_headers, content) = match find_bytes(part, b"\r\n\r\n").map(|i| (i, 4)).or_else(|| find_bytes(part, b"\n\n").map(|i| (i, 2))) {
            Some((i, sep)) => (&part[..i], &part[i + sep..]),
            None => (part, &[][..]),
        };
        let headers = String::from_utf8_lossy(raw_headers);
        let name = disposition_param(&headers, "name").unwrap_or_default();
        let file_name = disposition_param(&headers, "filename");
        let content_type = headers
            .lines()
            .find_map(|l| l.split_once(':').filter(|(k, _)| k.trim().eq_ignore_ascii_case("content-type")))
            .map(|(_, v)| v.trim().to_string());

        let field = match (file_name, std::str::from_utf8(content)) {
            (None, Ok(text)) => MultipartFieldSpec { content_type, ..text_field(name, text.to_string()) },
            (file_name, _) => {
                let mime = content_type.unwrap_or_else(|| "application/octet-stream".to_string());
                MultipartFieldSpec {
                    id: Some(new_id()),
                    enabled: true,
                    key: name,
                    value: format!("data:{};base64,{}", mime, base64::engine::general_purpose::STANDARD.encode(content)),
                    r#type: "base64".to_string(),
                    file_name: Some(file_name.unwrap_or_else(|| "file".to_string())),
                    content_type: Some(mime),
                }
            }
        };
        fields.push(field);
    }
    (!fields.is_empty()).then_some(fields)
}

fn disposition_param(headers: &str, name: &str) -> Option<String> {
    let line = headers.lines().find(|l| l.to_lowercase().starts_with("content-disposition"))?;
    line.split(';').skip(1).find_map(|segment| {
        let (key, value) = segment.trim().split_once('=')?;
        (key.trim().eq_ignore_ascii_case(name)).then(|| value.trim().trim_matches('"').to_string())
    })
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn split_bytes<'a>(bytes: &'a [u8], delimiter: &[u8]) -> Vec<&'a [u8]> {
    let mut parts = Vec::new();
    let mut rest = bytes;
    while let Some(i) = find_bytes(rest, delimiter) {
        parts.push(&rest[..i]);
        rest = &rest[i + delimiter.len()..];
    }
    parts.push(rest);
    parts
}

fn trim_crlf(mut part: &[u8]) -> &[u8] {
    for prefix in [&b"\r\n"[..], b"\n"] {
        if let Some(stripped) = part.strip_prefix(prefix) {
            part = stripped;
            break;
        }
    }
    for suffix in [&b"\r\n"[..], b"\n"] {
        if let Some(stripped) = part.strip_suffix(suffix) {
            part = stripped;
            break;
        }
    }
    part
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_captured_bodies() {
        assert_eq!(structured_from_capture("", "").0, "none");
        assert_eq!(structured_from_capture("application/json", "{\"a\":1}").0, "json");
        assert_eq!(structured_from_capture("text/plain", "hello world").0, "raw");
        assert_eq!(structured_from_capture("", "base64:AAEC").0, "binary");

        let (kind, content) = structured_from_capture("application/x-www-form-urlencoded", "a=1&b=hello+world");
        assert_eq!(kind, "urlencoded");
        let rows = read_urlencoded(&content);
        assert_eq!(rows[1].value, "hello world");
    }

    #[test]
    fn parses_multipart_text_and_file_fields() {
        let body = "--XYZ\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nhello\r\n--XYZ\r\nContent-Disposition: form-data; name=\"doc\"; filename=\"a.txt\"\r\nContent-Type: text/plain\r\n\r\nfile body\r\n--XYZ--\r\n";
        let (kind, content) = structured_from_capture("multipart/form-data; boundary=XYZ", body);
        assert_eq!(kind, "form");
        let fields = read_multipart(&content);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].value, "hello");
        assert_eq!(fields[1].r#type, "base64");
        assert_eq!(fields[1].file_name.as_deref(), Some("a.txt"));
    }

    #[test]
    fn converts_between_body_types() {
        let rows = convert("json", "urlencoded", "{\"a\":\"1\",\"n\":2}");
        let parsed = read_urlencoded(&rows);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[1].value, "2");

        let json = convert("urlencoded", "json", &rows);
        assert!(json.contains("\"a\": \"1\""));

        let form = convert("urlencoded", "form", &rows);
        assert_eq!(read_multipart(&form).len(), 2);
    }
}
