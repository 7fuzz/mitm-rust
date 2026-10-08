use mitm_core::curl::ParsedCurl;
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
        Self::new(flow.method.clone(), flow.url.clone(), request_headers, request_body)
    }

    pub fn new(method: String, url: String, headers: &[(String, String)], body: &str) -> Self {
        let content_type = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map_or("", |(_, v)| v.as_str());
        let (body_type, body_content) = structured_from_capture(content_type, body);
        Self {
            params: parse_query(&url, &[]),
            method,
            url,
            headers: header_items(headers),
            body_type,
            body_content,
        }
    }
}

impl From<ParsedCurl> for CapturedRequest {
    fn from(curl: ParsedCurl) -> Self {
        Self::new(curl.method, curl.url, &curl.headers, curl.body.as_deref().unwrap_or_default())
    }
}

pub fn header_items(headers: &[(String, String)]) -> Vec<HeaderItem> {
    headers
        .iter()
        .map(|(key, value)| HeaderItem { id: new_id(), key: key.clone(), value: value.clone(), enabled: true })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curl_becomes_params_headers_and_typed_body() {
        let curl = mitm_core::curl::parse_curl(
            "curl 'https://a.test/x?page=2' -H 'Content-Type: application/json' --data-raw '{\"a\":1}'",
        )
        .unwrap();
        let captured = CapturedRequest::from(curl);
        assert_eq!(captured.method, "POST");
        assert_eq!(captured.params[0].value, "2");
        assert_eq!(captured.headers[0].key, "Content-Type");
        assert_eq!(captured.body_type, "json");
    }
}
