const MAX_PRETTY_BYTES: usize = 4 * 1024 * 1024;
const INDENT: &str = "  ";
const VOID_ELEMENTS: [&str; 14] =
    ["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr"];
/// Their content is not markup, so it is kept whole instead of parsed.
const RAW_TEXT_ELEMENTS: [&str; 4] = ["script", "style", "pre", "textarea"];
/// Their whitespace is the content.
const PREFORMATTED: [&str; 2] = ["pre", "textarea"];

/// Indents HTML or XML one element per line; None when the text isn't markup.
pub fn pretty(text: &str, content_type: &str) -> Option<String> {
    let html = is_html(text, content_type)?;
    if text.len() > MAX_PRETTY_BYTES {
        return None;
    }
    Some(Printer { html, out: String::new(), stack: Vec::new() }.print(&tokenize(text, html)))
}

fn is_html(text: &str, content_type: &str) -> Option<bool> {
    let content_type = content_type.to_ascii_lowercase();
    let start = text.trim_start();
    if content_type.contains("html") {
        return Some(true);
    }
    if content_type.contains("xml") {
        return Some(false);
    }
    if !start.starts_with('<') {
        return None;
    }
    let head = start.get(..14).unwrap_or(start).to_ascii_lowercase();
    Some(head.starts_with("<!doctype html") || head.starts_with("<html"))
}

#[derive(Debug, PartialEq)]
enum Token<'a> {
    Text(&'a str),
    Raw(&'a str),
    Open { name: String, tag: &'a str, self_closing: bool },
    Close { name: String, tag: &'a str },
    /// Comments, doctypes, CDATA and processing instructions.
    Other(&'a str),
}

fn tokenize(text: &str, html: bool) -> Vec<Token<'_>> {
    let lower = text.to_ascii_lowercase();
    let find = |from: usize, needle: &str| lower[from..].find(needle).map(|at| from + at);
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        if !rest.starts_with('<') {
            let end = find(i, "<").unwrap_or(text.len());
            tokens.push(Token::Text(&text[i..end]));
            i = end;
            continue;
        }
        let next = rest.as_bytes().get(1).copied().unwrap_or(b' ');
        let (end, token) = if rest.starts_with("<!--") {
            let end = find(i + 4, "-->").map_or(text.len(), |at| at + 3);
            (end, Token::Other(&text[i..end]))
        } else if rest.starts_with("<![CDATA[") {
            let end = find(i, "]]>").map_or(text.len(), |at| at + 3);
            (end, Token::Other(&text[i..end]))
        } else if next == b'!' || next == b'?' {
            let end = find(i, ">").map_or(text.len(), |at| at + 1);
            (end, Token::Other(&text[i..end]))
        } else if next == b'/' {
            let end = find(i, ">").map_or(text.len(), |at| at + 1);
            let tag = &text[i..end];
            (end, Token::Close { name: tag_name(&tag[2..]), tag })
        } else if next.is_ascii_alphabetic() {
            let end = tag_end(text, i);
            let tag = &text[i..end];
            let name = tag_name(&tag[1..]);
            let self_closing = tag.ends_with("/>");
            let raw_text = html && !self_closing && RAW_TEXT_ELEMENTS.contains(&name.as_str());
            let close = format!("</{name}");
            tokens.push(Token::Open { name, tag, self_closing });
            if raw_text {
                let content_end = find(end, &close).unwrap_or(text.len());
                tokens.push(Token::Raw(&text[end..content_end]));
                i = content_end;
            } else {
                i = end;
            }
            continue;
        } else {
            let end = find(i + 1, "<").unwrap_or(text.len());
            (end, Token::Text(&text[i..end]))
        };
        tokens.push(token);
        i = end;
    }
    tokens
}

/// Where a start tag ends, skipping a `>` inside a quoted attribute value.
fn tag_end(text: &str, start: usize) -> usize {
    let mut quote = None;
    for (at, byte) in text.bytes().enumerate().skip(start) {
        match (quote, byte) {
            (None, b'"' | b'\'') => quote = Some(byte),
            (Some(q), _) if q == byte => quote = None,
            (None, b'>') => return at + 1,
            _ => {}
        }
    }
    text.len()
}

fn tag_name(after_bracket: &str) -> String {
    after_bracket
        .trim_start()
        .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

struct Printer {
    html: bool,
    out: String,
    stack: Vec<String>,
}

impl Printer {
    fn print(mut self, tokens: &[Token]) -> String {
        let mut i = 0;
        while i < tokens.len() {
            i += self.token(&tokens[i], &tokens[i + 1..]);
        }
        self.out.truncate(self.out.trim_end().len());
        self.out
    }

    /// Returns how many tokens it consumed.
    fn token(&mut self, token: &Token, after: &[Token]) -> usize {
        match token {
            Token::Text(text) => {
                let text = collapse(text);
                if !text.is_empty() {
                    self.line(&text);
                }
            }
            Token::Raw(text) => self.raw_lines(text),
            Token::Other(tag) => self.line(tag.trim()),
            Token::Close { name, tag } => {
                if let Some(at) = self.stack.iter().rposition(|open| open == name) {
                    self.stack.truncate(at);
                }
                self.line(tag);
            }
            Token::Open { name, tag, self_closing } => {
                if *self_closing || (self.html && VOID_ELEMENTS.contains(&name.as_str())) {
                    self.line(tag);
                    return 1;
                }
                return 1 + self.element(name, tag, after);
            }
        }
        1
    }

    /// An element whose only content is text, or nothing, stays on one line.
    fn element(&mut self, name: &str, tag: &str, after: &[Token]) -> usize {
        let closes = |token| closing_tag(token, name);
        match after.first() {
            Some(Token::Raw(text)) if PREFORMATTED.contains(&name) => {
                let close = closes(after.get(1)).unwrap_or_default();
                self.line(&format!("{tag}{text}{close}"));
                return if close.is_empty() { 1 } else { 2 };
            }
            Some(Token::Raw(text)) | Some(Token::Text(text)) => {
                if let Some(close) = closes(after.get(1)) {
                    let text = if matches!(after[0], Token::Raw(_)) { text.trim().to_string() } else { collapse(text) };
                    if !text.contains('\n') {
                        self.line(&format!("{tag}{text}{close}"));
                        return 2;
                    }
                }
            }
            first => {
                if let Some(close) = closes(first) {
                    self.line(&format!("{tag}{close}"));
                    return 1;
                }
            }
        }
        self.line(tag);
        self.stack.push(name.to_string());
        0
    }

    /// Script and style bodies keep their own relative indentation.
    fn raw_lines(&mut self, text: &str) {
        let lines: Vec<&str> = text.lines().filter(|line| !line.trim().is_empty()).collect();
        let margin = lines.iter().map(|line| line.len() - line.trim_start().len()).min().unwrap_or(0);
        for line in lines {
            self.line(line[margin..].trim_end());
        }
    }

    fn line(&mut self, text: &str) {
        self.out.push_str(&INDENT.repeat(self.stack.len()));
        self.out.push_str(text);
        self.out.push('\n');
    }
}

fn closing_tag<'a>(token: Option<&Token<'a>>, name: &str) -> Option<&'a str> {
    match token {
        Some(Token::Close { name: closing, tag }) if closing == name => Some(tag),
        _ => None,
    }
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indents_html_and_keeps_text_only_elements_on_one_line() {
        let html = r#"<!DOCTYPE html><html lang="en"><head><meta charSet="utf-8"/><title>Hi  there</title><script>var a = "<b>";</script></head><body><div class="a>b"><p>One <b>two</b></p><br><img src="x"></div><script src="s.js"></script></body></html>"#;
        let expected = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charSet="utf-8"/>
    <title>Hi there</title>
    <script>var a = "<b>";</script>
  </head>
  <body>
    <div class="a>b">
      <p>
        One
        <b>two</b>
      </p>
      <br>
      <img src="x">
    </div>
    <script src="s.js"></script>
  </body>
</html>"#;
        assert_eq!(pretty(html, "text/html; charset=utf-8").as_deref(), Some(expected));
    }

    #[test]
    fn multi_line_scripts_keep_relative_indentation_and_pre_is_untouched() {
        let html = "<body><script>\n    if (a) {\n      b();\n    }\n</script><pre>  x\n y</pre></body>";
        let expected = "<body>\n  <script>\n    if (a) {\n      b();\n    }\n  </script>\n  <pre>  x\n y</pre>\n</body>";
        assert_eq!(pretty(html, "text/html").as_deref(), Some(expected));
    }

    #[test]
    fn xml_has_no_void_elements_and_unclosed_html_tags_do_not_drift() {
        let rss = r#"<?xml version="1.0"?><rss><channel><link>https://a.test</link><item/></channel></rss>"#;
        let expected = "<?xml version=\"1.0\"?>\n<rss>\n  <channel>\n    <link>https://a.test</link>\n    <item/>\n  </channel>\n</rss>";
        assert_eq!(pretty(rss, "application/rss+xml").as_deref(), Some(expected));

        let list = "<ul><li>a<li>b</ul><p>c</p>";
        assert_eq!(pretty(list, "text/html").as_deref(), Some("<ul>\n  <li>\n    a\n    <li>\n      b\n</ul>\n<p>c</p>"));
    }

    #[test]
    fn non_markup_is_left_alone() {
        assert_eq!(pretty("plain text", ""), None);
        assert_eq!(pretty(r#"{"a":1}"#, "application/json"), None);
    }
}
