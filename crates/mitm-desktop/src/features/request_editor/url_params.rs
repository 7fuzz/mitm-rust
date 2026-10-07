use mitm_core::repeater::ParamItem;

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn parse_query(url: &str, existing: &[ParamItem]) -> Vec<ParamItem> {
    let disabled = existing.iter().filter(|p| !p.enabled);
    let Some((_, query)) = url.split_once('?') else {
        return disabled.cloned().collect();
    };

    let mut parsed: Vec<ParamItem> = Vec::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let (key, value) = (decode(key), decode(value));
        let reused = existing
            .iter()
            .find(|p| p.key == key && p.value == value && !parsed.iter().any(|n| n.id == p.id))
            .map(|p| p.id.clone());
        parsed.push(ParamItem { id: reused.unwrap_or_else(new_id), key, value, enabled: true });
    }

    let kept: Vec<ParamItem> = disabled
        .filter(|d| !parsed.iter().any(|p| !p.key.trim().is_empty() && p.key == d.key))
        .cloned()
        .collect();
    parsed.extend(kept);
    parsed
}

pub fn build_url(url: &str, params: &[ParamItem]) -> String {
    let base = url.split_once('?').map_or(url, |(base, _)| base);
    let query: Vec<String> = params
        .iter()
        .filter(|p| p.enabled && (!p.key.trim().is_empty() || !p.value.trim().is_empty()))
        .map(|p| format!("{}={}", encode_keeping_placeholders(&p.key), encode_keeping_placeholders(&p.value)))
        .collect();
    if query.is_empty() {
        base.to_string()
    } else {
        format!("{}?{}", base, query.join("&"))
    }
}

fn encode_keeping_placeholders(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let Some(len) = rest[start..].find("}}") else { break };
        out.push_str(&encode_component(&rest[..start]));
        out.push_str(&rest[start..start + len + 2]);
        rest = &rest[start + len + 2..];
    }
    out.push_str(&encode_component(rest));
    out
}

fn encode_component(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

pub fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = bytes
            .get(i + 1..i + 3)
            .filter(|hex| bytes[i] == b'%' && hex.iter().all(u8::is_ascii_hexdigit))
            .and_then(|hex| u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok());
        match (escaped, bytes[i]) {
            (Some(b), _) => {
                out.push(b);
                i += 3;
            }
            (None, b'+') => {
                out.push(b' ');
                i += 1;
            }
            (None, b) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_params_and_keeps_placeholders() {
        let params = parse_query("https://x.io/a?q=hello%20world&t={{TOKEN}}&flag", &[]);
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].value, "hello world");
        assert_eq!(params[1].value, "{{TOKEN}}");
        assert_eq!(params[2].key, "flag");
        assert_eq!(build_url("https://x.io/a?old=1", &params), "https://x.io/a?q=hello%20world&t={{TOKEN}}&flag=");
    }

    #[test]
    fn keeps_disabled_params_not_in_url() {
        let existing = vec![ParamItem { id: "d".into(), key: "debug".into(), value: "1".into(), enabled: false }];
        let params = parse_query("https://x.io/?a=1", &existing);
        assert_eq!(params.len(), 2);
        assert!(!params[1].enabled);
        assert_eq!(build_url("https://x.io/", &params), "https://x.io/?a=1");
    }
}
