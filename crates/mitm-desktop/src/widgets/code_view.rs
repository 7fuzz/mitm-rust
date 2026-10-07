use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{Color32, FontId, Style, TextStyle};
use crate::theme::Tone;

const MAX_HIGHLIGHT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    Json,
    Markup,
    Plain,
    Sql,
}

pub fn detect(content_type: &str, text: &str) -> Lang {
    let content_type = content_type.to_ascii_lowercase();
    let start = text.trim_start();
    if content_type.contains("json") || start.starts_with('{') || start.starts_with('[') {
        Lang::Json
    } else if content_type.contains("html") || content_type.contains("xml") || start.starts_with('<') {
        Lang::Markup
    } else {
        Lang::Plain
    }
}

struct Palette {
    font: FontId,
    text: Color32,
    punct: Color32,
    key: Color32,
    string: Color32,
    number: Color32,
    keyword: Color32,
}

impl Palette {
    fn new(style: &Style) -> Self {
        let visuals = &style.visuals;
        Self {
            font: TextStyle::Monospace.resolve(style),
            text: visuals.text_color(),
            punct: visuals.weak_text_color(),
            key: Tone::Info.color(visuals),
            string: Tone::Success.color(visuals),
            number: Tone::Caution.color(visuals),
            keyword: Tone::Accent.color(visuals),
        }
    }

    fn push(&self, job: &mut LayoutJob, text: &str, color: Color32) {
        if !text.is_empty() {
            job.append(text, 0.0, TextFormat::simple(self.font.clone(), color));
        }
    }
}

pub fn layout_job(style: &Style, text: &str, lang: Lang) -> LayoutJob {
    let palette = Palette::new(style);
    let mut job = LayoutJob::default();
    match lang {
        _ if text.len() > MAX_HIGHLIGHT_BYTES => palette.push(&mut job, text, palette.text),
        Lang::Json => json(&mut job, &palette, text),
        Lang::Markup => markup(&mut job, &palette, text),
        Lang::Plain => palette.push(&mut job, text, palette.text),
        Lang::Sql => sql(&mut job, &palette, text),
    }
    job
}

pub fn sql_layouter() -> impl FnMut(&eframe::egui::Ui, &dyn eframe::egui::TextBuffer, f32) -> std::sync::Arc<eframe::egui::Galley> {
    |ui, buf, wrap_width| {
        let mut job = layout_job(ui.style(), buf.as_str(), Lang::Sql);
        job.wrap.max_width = wrap_width;
        ui.fonts_mut(|f| f.layout_job(job))
    }
}

fn sql(job: &mut LayoutJob, p: &Palette, text: &str) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        match bytes[i] {
            b'\'' | b'"' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i < bytes.len() {
                    i += 1;
                }
                p.push(job, &text[start..i], p.string);
            }
            b'-' if i + 1 < bytes.len() && bytes[i + 1] == b'-' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                p.push(job, &text[start..i], p.punct);
            }
            b'0'..=b'9' => {
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                p.push(job, &text[start..i], p.number);
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &text[start..i];
                let is_kw = matches!(
                    word.to_ascii_uppercase().as_str(),
                    "SELECT" | "FROM" | "WHERE" | "INSERT" | "INTO" | "UPDATE" | "DELETE" |
                    "ORDER" | "BY" | "GROUP" | "HAVING" | "LIMIT" | "OFFSET" | "JOIN" |
                    "LEFT" | "RIGHT" | "INNER" | "OUTER" | "ON" | "AS" | "AND" | "OR" |
                    "NOT" | "IN" | "IS" | "NULL" | "LIKE" | "CAST" | "PRAGMA" | "TABLE" |
                    "CREATE" | "DROP" | "ALTER" | "INDEX" | "ASC" | "DESC" | "DISTINCT" |
                    "UNION" | "ALL" | "EXISTS" | "BETWEEN" | "CASE" | "WHEN" | "THEN" | "ELSE" | "END"
                );
                let color = if is_kw { p.keyword } else { p.text };
                p.push(job, word, color);
            }
            b' ' | b'\t' | b'\r' | b'\n' => {
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                p.push(job, &text[start..i], p.text);
            }
            _ => {
                p.push(job, &text[start..=start], p.punct);
                i += 1;
            }
        }
    }
}

fn json(job: &mut LayoutJob, p: &Palette, text: &str) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        match bytes[i] {
            b'"' => {
                i = string_end(bytes, i);
                let rest = text[i..].trim_start();
                let color = if rest.starts_with(':') { p.key } else { p.string };
                p.push(job, &text[start..i], color);
            }
            b'-' | b'0'..=b'9' => {
                i += 1;
                while i < bytes.len() && matches!(bytes[i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-') {
                    i += 1;
                }
                p.push(job, &text[start..i], p.number);
            }
            b'{' | b'}' | b'[' | b']' | b',' | b':' => {
                i += 1;
                p.push(job, &text[start..i], p.punct);
            }
            b if b.is_ascii_alphabetic() => {
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word = &text[start..i];
                let color = if matches!(word, "true" | "false" | "null") { p.keyword } else { p.text };
                p.push(job, word, color);
            }
            _ => {
                i = next_char(text, i);
                while i < bytes.len() && !matches!(bytes[i], b'"' | b'-' | b'0'..=b'9' | b'{' | b'}' | b'[' | b']' | b',' | b':')
                    && !bytes[i].is_ascii_alphabetic()
                {
                    i = next_char(text, i);
                }
                p.push(job, &text[start..i], p.text);
            }
        }
    }
}

fn markup(job: &mut LayoutJob, p: &Palette, text: &str) {
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(comment) = rest.strip_prefix("<!--") {
            let end = comment.find("-->").map_or(rest.len(), |e| e + 7);
            p.push(job, &rest[..end], p.punct);
            rest = &rest[end..];
        } else if rest.starts_with('<') {
            let end = tag_end(rest);
            tag(job, p, &rest[..end]);
            rest = &rest[end..];
        } else {
            let end = rest.find('<').unwrap_or(rest.len());
            p.push(job, &rest[..end], p.text);
            rest = &rest[end..];
        }
    }
}

fn tag(job: &mut LayoutJob, p: &Palette, tag: &str) {
    let bytes = tag.as_bytes();
    let mut i = 0;
    let mut named = false;
    while i < bytes.len() {
        let start = i;
        match bytes[i] {
            b'"' | b'\'' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                i = (i + 1).min(bytes.len());
                p.push(job, &tag[start..i], p.string);
            }
            b'<' | b'>' | b'/' | b'=' | b'?' | b'!' => {
                i += 1;
                p.push(job, &tag[start..i], p.punct);
            }
            b if b.is_ascii_whitespace() => {
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                p.push(job, &tag[start..i], p.text);
            }
            _ => {
                while i < bytes.len() && !matches!(bytes[i], b'"' | b'\'' | b'<' | b'>' | b'/' | b'=' | b'?' | b'!')
                    && !bytes[i].is_ascii_whitespace()
                {
                    i = next_char(tag, i);
                }
                p.push(job, &tag[start..i], if named { p.number } else { p.key });
                named = true;
            }
        }
    }
}

fn tag_end(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut quote = None;
    for (i, &b) in bytes.iter().enumerate().skip(1) {
        match (quote, b) {
            (None, b'"' | b'\'') => quote = Some(b),
            (Some(q), b) if b == q => quote = None,
            (None, b'>') => return i + 1,
            (None, b'<') => return i,
            _ => {}
        }
    }
    text.len()
}

fn string_end(bytes: &[u8], open: usize) -> usize {
    let mut i = open + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

fn next_char(text: &str, i: usize) -> usize {
    i + text[i..].chars().next().map_or(1, char::len_utf8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(text: &str, lang: Lang) -> Vec<String> {
        let job = layout_job(&Style::default(), text, lang);
        job.sections.iter().map(|s| job.text[s.byte_range.start.0..s.byte_range.end.0].to_string()).collect()
    }

    #[test]
    fn json_keeps_every_byte_and_splits_tokens() {
        let text = "{\n  \"név\": \"a\\\"b\", \"n\": -1.5e3, \"ok\": true }";
        let parts = spans(text, Lang::Json);
        assert_eq!(parts.concat(), text);
        assert!(parts.contains(&"\"név\"".to_string()));
        assert!(parts.contains(&"\"a\\\"b\"".to_string()));
        assert!(parts.contains(&"-1.5e3".to_string()));
        assert!(parts.contains(&"true".to_string()));
    }

    #[test]
    fn markup_keeps_every_byte_and_splits_attributes() {
        let text = "<!-- x --><a href=\"/p?a>b\">héllo</a>";
        let parts = spans(text, Lang::Markup);
        assert_eq!(parts.concat(), text);
        assert!(parts.contains(&"\"/p?a>b\"".to_string()));
        assert!(parts.contains(&"href".to_string()));
    }

    #[test]
    fn detects_language() {
        assert_eq!(detect("application/json; charset=utf-8", ""), Lang::Json);
        assert_eq!(detect("", "  [1]"), Lang::Json);
        assert_eq!(detect("text/html", ""), Lang::Markup);
        assert_eq!(detect("text/plain", "hello"), Lang::Plain);
    }

    #[test]
    fn sql_keeps_every_byte_and_splits_tokens() {
        let text = "SELECT id, name FROM users WHERE id = 42 AND active = 'yes'; -- comment";
        let parts = spans(text, Lang::Sql);
        assert_eq!(parts.concat(), text);
        assert!(parts.contains(&"SELECT".to_string()));
        assert!(parts.contains(&"'yes'".to_string()));
        assert!(parts.contains(&"42".to_string()));
    }
}
