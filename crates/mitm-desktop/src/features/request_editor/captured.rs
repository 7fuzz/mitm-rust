use mitm_core::repeater::{HeaderItem, ParamItem};
use mitm_core::state::{HistoryDetail, HistoryEntry};
use super::body::structured_from_capture;
use super::url_params::{new_id, parse_query};

/// A captured flow's request, in the shape the request editors store.
pub struct CapturedRequest {
    pub method: String,
    pub url: String,
    pub params: Vec<ParamItem>,
    pub headers: Vec<HeaderItem>,
    pub body_type: String,
    pub body_content: String,
}

impl CapturedRequest {
    /// `detail` carries the full headers and body; a list row may have neither.
    pub fn from_flow(flow: &HistoryEntry, detail: Option<&HistoryDetail>) -> Self {
        let request_headers = detail.map_or(&flow.request_headers, |d| &d.request_headers);
        let request_body = detail.map_or(&flow.request_body, |d| &d.request_body);
        let content_type = request_headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map_or("", |(_, v)| v.as_str());
        let (body_type, body_content) = structured_from_capture(content_type, request_body);
        Self {
            method: flow.method.clone(),
            url: flow.url.clone(),
            params: parse_query(&flow.url, &[]),
            headers: header_items(request_headers),
            body_type,
            body_content,
        }
    }
}

pub fn header_items(headers: &[(String, String)]) -> Vec<HeaderItem> {
    headers
        .iter()
        .map(|(key, value)| HeaderItem { id: new_id(), key: key.clone(), value: value.clone(), enabled: true })
        .collect()
}
