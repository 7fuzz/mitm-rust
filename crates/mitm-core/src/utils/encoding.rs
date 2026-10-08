use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HexFormat {
    /// `4d49`
    Plain,
    /// `4d 49`
    Spaced,
    /// `4d:49`
    Colon,
    /// `0x4d 0x49`
    ZeroX,
    /// `\x4d\x49`
    BackslashX,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JwtParts {
    pub header: String,
    pub payload: String,
    pub signature: String,
}

pub fn base64_encode(input: &str, url_safe: bool) -> String {
    if url_safe {
        URL_SAFE_NO_PAD.encode(input.as_bytes())
    } else {
        STANDARD.encode(input.as_bytes())
    }
}

pub fn base64_decode(input: &str, url_safe: bool) -> Result<String, String> {
    let mut cleaned: String = input.split_whitespace().collect();
    if url_safe {
        cleaned = cleaned.replace('-', "+").replace('_', "/");
    }
    while cleaned.len() % 4 != 0 {
        cleaned.push('=');
    }
    let bytes = STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|_| "Contains characters outside the Base64 alphabet".to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn url_encode(input: &str, full: bool) -> String {
    if full {
        input
            .bytes()
            .map(|b| format!("%{:02X}", b))
            .collect()
    } else {
        urlencoding::encode(input).into_owned()
    }
}

pub fn url_decode(input: &str) -> Result<String, String> {
    let prepared = input.replace('+', "%20");
    urlencoding::decode(&prepared)
        .map(|s| s.into_owned())
        .map_err(|e| e.to_string())
}

pub fn html_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

pub fn html_decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        match after.find(';') {
            Some(end) if end <= 10 => {
                let entity = &after[..end];
                if let Some(ch) = decode_entity(entity) {
                    out.push(ch);
                } else {
                    out.push('&');
                    out.push_str(entity);
                    out.push(';');
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn decode_entity(entity: &str) -> Option<char> {
    let code = if let Some(hex) = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X")) {
        u32::from_str_radix(hex, 16).ok()?
    } else if let Some(dec) = entity.strip_prefix('#') {
        dec.parse::<u32>().ok()?
    } else {
        return match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some('\u{00a0}'),
            _ => None,
        };
    };
    char::from_u32(code)
}

pub fn hex_encode(input: &str, format: HexFormat, upper: bool) -> String {
    let byte = |b: u8| if upper { format!("{:02X}", b) } else { format!("{:02x}", b) };
    let parts: Vec<String> = input.bytes().map(byte).collect();
    match format {
        HexFormat::Plain => parts.join(""),
        HexFormat::Spaced => parts.join(" "),
        HexFormat::Colon => parts.join(":"),
        HexFormat::ZeroX => parts.iter().map(|p| format!("0x{}", p)).collect::<Vec<_>>().join(" "),
        HexFormat::BackslashX => parts.iter().map(|p| format!("\\x{}", p)).collect::<Vec<_>>().join(""),
    }
}

pub fn hex_decode(input: &str) -> Result<String, String> {
    let lowered = input.replace("0x", "").replace("0X", "").replace("\\x", "").replace("\\X", "");
    let cleaned: String = lowered
        .chars()
        .filter(|c| !matches!(c, ':' | ',' | '-' | '_') && !c.is_whitespace())
        .collect();
    if cleaned.is_empty() {
        return Ok(String::new());
    }
    if cleaned.len() % 2 != 0 {
        return Err(format!("Odd number of hex digits ({})", cleaned.len()));
    }
    if !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Contains non-hex characters".to_string());
    }
    let bytes: Vec<u8> = (0..cleaned.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16).unwrap())
        .collect();
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Escapes non-ASCII characters as `\uXXXX` (astral code points as a surrogate pair), leaving ASCII intact.
pub fn unicode_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        let code = ch as u32;
        if code < 0x80 {
            out.push(ch);
        } else if code <= 0xFFFF {
            out.push_str(&format!("\\u{:04x}", code));
        } else {
            let v = code - 0x10000;
            let high = 0xD800 + (v >> 10);
            let low = 0xDC00 + (v & 0x3FF);
            out.push_str(&format!("\\u{:04x}\\u{:04x}", high, low));
        }
    }
    out
}

pub fn unicode_unescape(input: &str) -> Result<String, String> {
    let mut units: Vec<u16> = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && (bytes[i + 1] == b'u' || bytes[i + 1] == b'U') {
            let hex = input
                .get(i + 2..i + 6)
                .ok_or_else(|| "Incomplete \\u escape".to_string())?;
            let unit = u16::from_str_radix(hex, 16).map_err(|_| format!("Invalid \\u escape: \\u{}", hex))?;
            units.push(unit);
            i += 6;
        } else {
            let ch = input[i..].chars().next().unwrap();
            let mut buf = [0u16; 2];
            for u in ch.encode_utf16(&mut buf) {
                units.push(*u);
            }
            i += ch.len_utf8();
        }
    }
    String::from_utf16(&units).map_err(|_| "Unpaired surrogate in \\u escapes".to_string())
}

pub fn jwt_decode(input: &str) -> Result<JwtParts, String> {
    let parts: Vec<&str> = input.trim().split('.').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return Err("Expected header.payload.signature".to_string());
    }
    let header = pretty_json(&base64_decode(parts[0], true)?);
    let payload = pretty_json(&base64_decode(parts[1], true)?);
    let signature = parts.get(2).copied().unwrap_or("").to_string();
    Ok(JwtParts { header, payload, signature })
}

fn pretty_json(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| serde_json::to_string_pretty(&v).ok())
        .unwrap_or_else(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips_both_alphabets() {
        let text = "MITM-Developer-Security-Suite-2.0";
        assert_eq!(base64_decode(&base64_encode(text, false), false).unwrap(), text);
        assert_eq!(base64_decode(&base64_encode(text, true), true).unwrap(), text);
        // url-safe output has no + / or padding
        let encoded = base64_encode("??>>??", true);
        assert!(!encoded.contains('+') && !encoded.contains('/') && !encoded.contains('='));
        assert!(base64_decode("not valid base64!", false).is_err());
    }

    #[test]
    fn url_component_and_full_differ_on_reserved() {
        assert_eq!(url_encode("a b/c", false), "a%20b%2Fc");
        assert_eq!(url_encode("a b/c", true), "%61%20%62%2F%63");
        assert_eq!(url_decode("a%20b%2Fc").unwrap(), "a b/c");
        assert_eq!(url_decode("a+b").unwrap(), "a b");
    }

    #[test]
    fn html_entities_round_trip_named_and_numeric() {
        let raw = r#"<div class="a" x='b'>&</div>"#;
        let encoded = html_encode(raw);
        assert_eq!(encoded, "&lt;div class=&quot;a&quot; x=&#39;b&#39;&gt;&amp;&lt;/div&gt;");
        assert_eq!(html_decode(&encoded), raw);
        assert_eq!(html_decode("&#65;&#x42;&nbsp;C"), "AB\u{00a0}C");
        assert_eq!(html_decode("plain & loose"), "plain & loose");
    }

    #[test]
    fn hex_formats_encode_and_decode_tolerantly() {
        assert_eq!(hex_encode("MI", HexFormat::Plain, false), "4d49");
        assert_eq!(hex_encode("MI", HexFormat::Spaced, false), "4d 49");
        assert_eq!(hex_encode("MI", HexFormat::Colon, true), "4D:49");
        assert_eq!(hex_encode("MI", HexFormat::ZeroX, false), "0x4d 0x49");
        assert_eq!(hex_encode("MI", HexFormat::BackslashX, false), "\\x4d\\x49");
        for s in ["4d49", "4d 49", "4d:49", "0x4d 0x49", "\\x4d\\x49", "4D-49"] {
            assert_eq!(hex_decode(s).unwrap(), "MI", "decoding {s}");
        }
        assert!(hex_decode("4d4").is_err());
        assert!(hex_decode("zz").is_err());
        assert_eq!(hex_decode("   ").unwrap(), "");
    }

    #[test]
    fn unicode_escape_round_trips_and_keeps_ascii() {
        assert_eq!(unicode_escape("aé😀"), "a\\u00e9\\ud83d\\ude00");
        assert_eq!(unicode_unescape("a\\u00e9\\ud83d\\ude00").unwrap(), "aé😀");
        assert_eq!(unicode_unescape("plain text").unwrap(), "plain text");
        assert!(unicode_unescape("\\u00g1").is_err());
    }

    #[test]
    fn jwt_decode_splits_and_pretties() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjMifQ.sig";
        let parts = jwt_decode(token).unwrap();
        assert!(parts.header.contains("\"alg\": \"HS256\""));
        assert!(parts.payload.contains("\"sub\": \"123\""));
        assert_eq!(parts.signature, "sig");
        assert!(jwt_decode("only-one-part").is_err());
    }
}
