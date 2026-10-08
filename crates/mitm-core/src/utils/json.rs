use serde::Serialize;
use serde_json::ser::{PrettyFormatter, Serializer};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indent {
    Two,
    Four,
    Tab,
}

impl Indent {
    fn bytes(self) -> &'static [u8] {
        match self {
            Indent::Two => b"  ",
            Indent::Four => b"    ",
            Indent::Tab => b"\t",
        }
    }
}

fn parse(input: &str) -> Result<Value, String> {
    serde_json::from_str(input).map_err(|e| format!("line {}, column {}: {}", e.line(), e.column(), e))
}

/// serde_json is built with `preserve_order`, so key order is kept unless we sort it here.
fn sorted(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().map(|(k, v)| (k, sorted(v))).collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(entries.into_iter().collect())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
        other => other,
    }
}

fn to_pretty(value: &Value, indent: Indent) -> Result<String, String> {
    let mut buf = Vec::new();
    let mut ser = Serializer::with_formatter(&mut buf, PrettyFormatter::with_indent(indent.bytes()));
    value.serialize(&mut ser).map_err(|e| e.to_string())?;
    String::from_utf8(buf).map_err(|e| e.to_string())
}

pub fn format(input: &str, indent: Indent, sort_keys: bool) -> Result<String, String> {
    let mut value = parse(input)?;
    if sort_keys {
        value = sorted(value);
    }
    to_pretty(&value, indent)
}

pub fn minify(input: &str) -> Result<String, String> {
    serde_json::to_string(&parse(input)?).map_err(|e| e.to_string())
}

/// JSON-encodes the raw text as a string literal (quotes included), for embedding.
pub fn escape(input: &str) -> String {
    serde_json::to_string(input).unwrap_or_default()
}

/// Reverses `escape`. The surrounding quotes are optional, so a pasted fragment like `a\"b` works too.
pub fn unescape(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    let literal = if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed.to_string()
    } else {
        format!("\"{trimmed}\"")
    };
    serde_json::from_str(&literal).map_err(|e| e.to_string())
}

/// Sorted, pretty JSON, so two documents that differ only in key order or whitespace diff as equal.
pub fn canonicalize(input: &str) -> Result<String, String> {
    to_pretty(&sorted(parse(input)?), Indent::Two)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_indents_and_optionally_sorts() {
        let input = r#"{"b":1,"a":{"d":2,"c":3}}"#;
        let two = format(input, Indent::Two, false).unwrap();
        assert_eq!(two, "{\n  \"b\": 1,\n  \"a\": {\n    \"d\": 2,\n    \"c\": 3\n  }\n}");
        let tab_sorted = format(input, Indent::Tab, true).unwrap();
        assert_eq!(tab_sorted, "{\n\t\"a\": {\n\t\t\"c\": 3,\n\t\t\"d\": 2\n\t},\n\t\"b\": 1\n}");
    }

    #[test]
    fn minify_escape_and_unescape() {
        assert_eq!(minify("{  \"a\" : [1, 2] }").unwrap(), r#"{"a":[1,2]}"#);
        assert_eq!(escape("a\"b\nc"), r#""a\"b\nc""#);
        assert_eq!(unescape(r#""a\"b\nc""#).unwrap(), "a\"b\nc");
        assert_eq!(unescape(r#"a\"b\u00e9"#).unwrap(), "a\"bé");
        assert!(unescape(r#"bad \x"#).is_err());
        assert!(minify("{bad}").is_err());
    }

    #[test]
    fn canonicalize_ignores_key_order_and_whitespace() {
        let a = canonicalize(r#"{"b":1,"a":2}"#).unwrap();
        let b = canonicalize("{\n  \"a\": 2,\n  \"b\": 1\n}").unwrap();
        assert_eq!(a, b);
    }
}
