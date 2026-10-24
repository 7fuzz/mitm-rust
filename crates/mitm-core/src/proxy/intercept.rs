use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::oneshot;
use uuid::Uuid;
use crate::events::EventBus;
use crate::proxy::containers::ContainerTag;

use crate::proxy::rules::should_intercept;
use crate::state::AppState;
use crate::proxy::history::{HistoryEntry, TrafficCapturedEvent};

#[allow(clippy::too_many_arguments)]
pub async fn handle_intercept_hook(
    entry_id: Option<i64>,
    request_at: &str,
    event_bus: Option<&EventBus>,
    state: &Arc<AppState>,
    phase: InterceptPhase,
    method: &str,
    url: &str,
    host: &str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    request_headers: &[(String, String)],
    listener_label: &str,
    container: &ContainerTag,
) -> Option<InterceptAction> {
    if !should_intercept(state, phase, method, url, host, &headers, request_headers, listener_label).await {
        return None;
    }

    let flow_id = Uuid::new_v4().to_string();
    let (tx, rx) = oneshot::channel::<InterceptAction>();

    let pending = PendingFlow {
        flow_id: flow_id.clone(),
        phase,
        method: method.to_string(),
        url: url.to_string(),
        headers: headers.clone(),
        body: body.clone(),
        tx,
    };

    state.pending_flows.insert(flow_id.clone(), pending);

    let content_encoding = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-encoding"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");

    let decompressed_body = crate::http::body::decompress_body(&body, content_encoding);
    let body_text = String::from_utf8(decompressed_body).unwrap_or_else(|e| {
        use base64::Engine;
        format!("base64:{}", base64::engine::general_purpose::STANDARD.encode(e.into_bytes()))
    });

    let payload = PendingFlowPayload {
        flow_id: flow_id.clone(),
        phase: match phase {
            InterceptPhase::Request => "request".to_string(),
            InterceptPhase::Response => "response".to_string(),
        },
        method: method.to_string(),
        url: url.to_string(),
        headers: headers.clone(),
        body,
        body_text: body_text.clone(),
    };

    if let Some(bus) = event_bus {
        bus.emit(crate::events::CoreEvent::InterceptPrompt(payload.clone()));
    }

    if let Some(eid) = entry_id {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        let phase_str = match phase {
            InterceptPhase::Request => "intercepted_request",
            InterceptPhase::Response => "intercepted_response",
        };
        let intercepted_entry = HistoryEntry {
            id: eid,
            method: method.to_string(),
            url: url.to_string(),
            host: host.to_string(),
            path: crate::proxy::parse_path_from_url(url),
            content_type: "-".to_string(),
            response_size: 0,
            status_code: 0,
            request_headers: headers,
            response_headers: vec![],
            request_body: body_text,
            response_body: "".to_string(),
            phase: phase_str.to_string(),
            duration_ms: None,
            created_at: now,
            is_intercepted: true,
            is_rewritten: false,
            is_failed: false,
            listener_label: listener_label.to_string(),
            container_label: container.label.clone(),
            run_id: container.run_id,
            request_at: Some(request_at.to_string()),
            response_at: None,
        };
        if let Some(bus) = event_bus {
            bus.emit(crate::events::CoreEvent::TrafficCaptured(TrafficCapturedEvent { entry: intercepted_entry }));
        }
    }

    match rx.await {
        Ok(action) => Some(action),
        Err(_) => Some(InterceptAction::Drop),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InterceptPhase {
    Request,
    Response,
}

#[derive(Debug)]
pub enum InterceptAction {
    Forward {
        modified_url: Option<String>,
        modified_method: Option<String>,
        modified_headers: Option<Vec<(String, String)>>,
        modified_body: Option<Vec<u8>>,
    },
    Drop,
}

pub struct PendingFlow {
    pub flow_id: String,
    pub phase: InterceptPhase,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub tx: oneshot::Sender<InterceptAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingFlowPayload {
    pub flow_id: String,
    pub phase: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub body_text: String,
}
