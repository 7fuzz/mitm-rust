use std::sync::Arc;
use tokio::sync::oneshot;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::proxy::rules::should_intercept;
use crate::state::{AppState, HistoryEntry, InterceptAction, InterceptPhase, PendingFlow, PendingFlowPayload, TrafficCapturedEvent};

pub async fn handle_intercept_hook(
    entry_id: Option<&str>,
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

    let content_encoding = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-encoding"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");

    let decompressed_body = crate::encoding::decompress_body(&body, content_encoding);
    let body_text = String::from_utf8(decompressed_body)
        .unwrap_or_else(|_| format!("<binary data {} bytes>", body.len()));

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

    let _ = app_handle.emit("intercept_triggered", &payload);

    let focus_pref = crate::db::get_preference(&state.db_path, "focus_on_intercepted");
    if focus_pref.as_deref() != Some("false") {
        for (_, window) in app_handle.webview_windows() {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.set_always_on_top(true);
            let _ = window.set_always_on_top(false);
            let _ = window.request_user_attention(Some(tauri::UserAttentionType::Critical));
        }
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
            id: eid.to_string(),
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
        };
        let _ = app_handle.emit("traffic_captured", &TrafficCapturedEvent { entry: intercepted_entry });
    }

    match rx.await {
        Ok(action) => Some(action),
        Err(_) => Some(InterceptAction::Drop),
    }
}
