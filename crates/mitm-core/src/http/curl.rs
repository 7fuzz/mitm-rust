use base64::Engine;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCurl {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

// Flags whose next token is their value; any other flag is a switch like -k or --compressed.
const VALUE_FLAGS: &[&str] = &[
    "-X", "--request", "-H", "--header", "-d", "--data", "--data-raw", "--data-binary", "--data-ascii",
    "--data-urlencode", "-b", "--cookie", "-u", "--user", "-A", "--user-agent", "-e", "--referer", "--url",
    "-F", "--form", "-o", "--output", "-x", "--proxy", "-m", "--max-time", "--connect-timeout", "-w",
    "--write-out", "--cacert", "--cert", "--key", "-T", "--upload-file", "-c", "--cookie-jar",
];

pub fn parse_curl(command: &str) -> Result<ParsedCurl, String> {
    let tokens = tokenize(command)?;
    let mut tokens = tokens.into_iter();
    if tokens.next().as_deref() != Some("curl") {
        return Err("Not a curl command".to_string());
    }

    let mut method = None;
    let mut url = None;
    let mut headers = Vec::new();
    let mut data: Vec<String> = Vec::new();
    let mut get = false;
    let mut head = false;

    while let Some(token) = tokens.next() {
        let (flag, inline) = split_flag(&token);
        let Some(flag) = flag else {
            url.get_or_insert(token);
            continue;
        };
        let value = if VALUE_FLAGS.contains(&flag) {
            match inline {
                Some(v) => v,
                None => tokens.next().ok_or_else(|| format!("{flag} needs a value"))?,
            }
        } else {
            String::new()
        };
        match flag {
            "-X" | "--request" => method = Some(value.to_uppercase()),
            "-H" | "--header" => {
                if let Some((k, v)) = value.split_once(':') {
                    headers.push((k.trim().to_string(), v.trim().to_string()));
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" | "--data-urlencode" => data.push(value),
            "-b" | "--cookie" => headers.push(("Cookie".to_string(), value)),
            "-A" | "--user-agent" => headers.push(("User-Agent".to_string(), value)),
            "-e" | "--referer" => headers.push(("Referer".to_string(), value)),
            "-u" | "--user" => {
                let encoded = base64::engine::general_purpose::STANDARD.encode(value);
                headers.push(("Authorization".to_string(), format!("Basic {encoded}")));
            }
            "--url" => url = Some(value),
            "-G" | "--get" => get = true,
            "-I" | "--head" => head = true,
            _ => {}
        }
    }

    let mut url = url.ok_or("No URL in the curl command")?;
    let mut body = (!data.is_empty()).then(|| data.join("&"));
    if get {
        if let Some(query) = body.take() {
            url.push(if url.contains('?') { '&' } else { '?' });
            url.push_str(&query);
        }
    }
    let method = method.unwrap_or_else(|| match (head, body.is_some()) {
        (true, _) => "HEAD".to_string(),
        (_, true) => "POST".to_string(),
        _ => "GET".to_string(),
    });
    Ok(ParsedCurl { method, url, headers, body })
}

/// `-XPOST` and `--request=POST` carry their value in the same token.
fn split_flag(token: &str) -> (Option<&str>, Option<String>) {
    if let Some(long) = token.strip_prefix("--") {
        return match long.split_once('=') {
            Some((name, value)) => (Some(&token[..name.len() + 2]), Some(value.to_string())),
            None => (Some(token), None),
        };
    }
    if token.len() > 2 && token.starts_with('-') && VALUE_FLAGS.contains(&&token[..2]) {
        return (Some(&token[..2]), Some(token[2..].to_string()));
    }
    if token.starts_with('-') && token.len() > 1 {
        return (Some(token), None);
    }
    (None, None)
}

/// Splits like a POSIX shell: quotes, backslash escapes and line continuations.
fn tokenize(command: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_token = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => current.push(c),
                        None => return Err("Unclosed single quote".to_string()),
                    }
                }
            }
            '"' => {
                in_token = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(c @ ('"' | '\\' | '$' | '`')) => current.push(c),
                            Some('\n') => {}
                            Some(c) => {
                                current.push('\\');
                                current.push(c);
                            }
                            None => return Err("Unclosed double quote".to_string()),
                        },
                        Some(c) => current.push(c),
                        None => return Err("Unclosed double quote".to_string()),
                    }
                }
            }
            '\\' => match chars.next() {
                Some('\n') | Some('\r') => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                }
                Some(c) => {
                    in_token = true;
                    current.push(c);
                }
                None => {}
            },
            c if c.is_whitespace() => {
                if in_token {
                    tokens.push(std::mem::take(&mut current));
                    in_token = false;
                }
            }
            c => {
                in_token = true;
                current.push(c);
            }
        }
    }
    if in_token {
        tokens.push(current);
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_method_headers_and_body_across_continuations() {
        let command = "curl -X PUT 'https://api.test/users/1' \\\n  -H 'Content-Type: application/json' \\\n  -H \"Authorization: Bearer abc\" \\\n  --data-raw '{\"name\":\"a b\"}' --compressed -k";
        let parsed = parse_curl(command).unwrap();
        assert_eq!(parsed.method, "PUT");
        assert_eq!(parsed.url, "https://api.test/users/1");
        assert_eq!(parsed.headers[1], ("Authorization".to_string(), "Bearer abc".to_string()));
        assert_eq!(parsed.body.as_deref(), Some("{\"name\":\"a b\"}"));
    }

    #[test]
    fn data_implies_post_and_get_moves_it_to_the_query() {
        assert_eq!(parse_curl("curl https://a.test -d a=1 -d b=2").unwrap().method, "POST");
        let get = parse_curl("curl -G https://a.test?x=0 --data-urlencode q=hi").unwrap();
        assert_eq!((get.method.as_str(), get.url.as_str(), get.body), ("GET", "https://a.test?x=0&q=hi", None));
    }

    #[test]
    fn inline_values_users_and_errors() {
        let parsed = parse_curl("curl -XDELETE --url=https://a.test -u me:pw").unwrap();
        assert_eq!(parsed.method, "DELETE");
        assert_eq!(parsed.url, "https://a.test");
        assert_eq!(parsed.headers[0].1, "Basic bWU6cHc=");
        assert!(parse_curl("wget https://a.test").is_err());
        assert!(parse_curl("curl -H 'x: y'").is_err());
        assert!(parse_curl("curl 'https://a.test").is_err());
    }
}
