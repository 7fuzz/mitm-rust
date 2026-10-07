mod view;

use crate::features::request_editor::url_params::decode;
use crate::widgets::message_pane::PaneState;

pub use view::show;

pub struct InspectorState {
    pub open: bool,
    pub table_fraction: f32,
    pub request: PaneState,
    pub response: PaneState,
    pub split: f32,
}

impl Default for InspectorState {
    fn default() -> Self {
        Self {
            open: false,
            table_fraction: 0.5,
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
            (decode(key), decode(value))
        })
        .collect()
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
