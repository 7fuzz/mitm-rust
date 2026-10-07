mod pane;
mod view;

pub use view::show;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneTab {
    Body,
    Headers,
    Params,
    Cookies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyFormat {
    Pretty,
    Raw,
    Hex,
}

pub struct PaneState {
    pub tab: PaneTab,
    pub format: BodyFormat,
    rendered: Option<((i64, usize, BodyFormat), String)>,
}

impl Default for PaneState {
    fn default() -> Self {
        Self {
            tab: PaneTab::Body,
            format: BodyFormat::Pretty,
            rendered: None,
        }
    }
}

pub struct InspectorState {
    pub request: PaneState,
    pub response: PaneState,
    pub split: f32,
}

impl Default for InspectorState {
    fn default() -> Self {
        Self {
            request: PaneState::default(),
            response: PaneState::default(),
            split: 0.5,
        }
    }
}

fn query_params(url: &str) -> Vec<(String, String)> {
    let Some((_, query)) = url.split_once('?') else {
        return Vec::new();
    };
    let query = query.split('#').next().unwrap_or_default();
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(key), percent_decode(value))
        })
        .collect()
}

fn percent_decode(text: &str) -> String {
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

fn header_values<'a>(headers: &'a [(String, String)], name: &'a str) -> impl Iterator<Item = &'a str> {
    headers
        .iter()
        .filter(move |(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

fn split_cookie(pair: &str) -> (String, String) {
    let (key, value) = pair.trim().split_once('=').unwrap_or((pair.trim(), ""));
    (key.to_string(), value.to_string())
}

fn request_cookies(headers: &[(String, String)]) -> Vec<(String, String)> {
    header_values(headers, "cookie")
        .flat_map(|v| v.split(';'))
        .filter(|pair| !pair.trim().is_empty())
        .map(split_cookie)
        .collect()
}

fn response_cookies(headers: &[(String, String)]) -> Vec<(String, String)> {
    header_values(headers, "set-cookie")
        .map(|v| split_cookie(v.split(';').next().unwrap_or_default()))
        .collect()
}
