use std::sync::Arc;
use tokio::sync::oneshot;
use tauri::AppHandle;
use tauri::Emitter;
use uuid::Uuid;

use crate::proxy::rules::should_intercept;
use crate::state::{AppState, InterceptAction, InterceptPhase, PendingFlow, PendingFlowPayload};

pub async fn handle_intercept_hook(
    app_handle: &AppHandle,
    state: &Arc<AppState>,
    phase: InterceptPhase,
    method: &str,
    url: &str,
    host: &str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
) -> Option<InterceptAction> {
    if !should_intercept(state, phase, method, url, host, &headers).await {
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

    let body_text = String::from_utf8(body.clone()).unwrap_or_else(|_| format!("<binary data {} bytes>", body.len()));

    let payload = PendingFlowPayload {
        flow_id: flow_id.clone(),
        phase: match phase {
            InterceptPhase::Request => "request".to_string(),
            InterceptPhase::Response => "response".to_string(),
        },
        method: method.to_string(),
        url: url.to_string(),
        headers,
        body,
        body_text,
    };

    let _ = app_handle.emit("intercept_triggered", &payload);

    match rx.await {
        Ok(action) => Some(action),
        Err(_) => Some(InterceptAction::Drop),
    }
}
