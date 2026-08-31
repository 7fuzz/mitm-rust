use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookReplayResult {
    pub success: bool,
    pub status_code: u16,
    pub response_body: String,
    pub duration_ms: u64,
}

pub async fn replay_delivery(
    headers: Vec<(String, String)>,
    payload: String,
    target_url: &str,
) -> Result<WebhookReplayResult, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client.post(target_url);

    for (k, v) in headers {
        if !k.eq_ignore_ascii_case("host") && !k.eq_ignore_ascii_case("content-length") {
            req = req.header(k, v);
        }
    }

    let start = std::time::Instant::now();
    let resp = req
        .body(payload)
        .send()
        .await
        .map_err(|e| format!("Replay network error: {}", e))?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let status_code = resp.status().as_u16();
    let response_body = resp.text().await.unwrap_or_default();

    Ok(WebhookReplayResult {
        success: status_code < 400,
        status_code,
        response_body,
        duration_ms,
    })
}
