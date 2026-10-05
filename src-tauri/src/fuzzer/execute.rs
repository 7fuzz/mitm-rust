use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream::{self, StreamExt};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::{
    apply_payloads, evaluate_matches, generate_combos, FuzzConfig, FuzzResult, MatchResult,
};
use crate::repeater::execute::send_tab_request;
use crate::repeater::RepeaterTab;
use crate::state::AppState;

/// The current run's full results, kept in memory for row detail and later saving.
pub struct FuzzRunBuffer {
    pub run_id: i64,
    pub config: FuzzConfig,
    pub results: Vec<FuzzResult>,
    pub saved: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzResultEvent {
    pub run_id: i64,
    pub idx: u32,
    pub payloads: Vec<String>,
    pub status_code: u16,
    pub response_size: u64,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub matches: Vec<MatchResult>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzDoneEvent {
    pub run_id: i64,
    pub completed: u32,
    pub total: u32,
    pub stopped: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzStartedEvent {
    pub run_id: i64,
    pub total: u32,
}

fn template_to_tab(idx: u32, t: &super::FuzzTemplate) -> RepeaterTab {
    RepeaterTab {
        id: format!("fuzz-{}", idx),
        method: t.method.clone(),
        url: t.url.clone(),
        headers: t.headers.clone(),
        params: t.params.clone(),
        body_type: t.body_type.clone(),
        body_content: t.body.clone(),
        extract_rules: Vec::new(),
        pre_requests: vec![],
        post_requests: vec![],
        order_index: 0,
        created_at_ms: 0,
        updated_at_ms: 0,
        execution_count: 0,
        last_status_code: None,
        last_duration_ms: None,
    }
}

async fn run_one(
    idx: u32,
    config: Arc<FuzzConfig>,
    names: Arc<Vec<String>>,
    payloads: Vec<String>,
) -> FuzzResult {
    let values: HashMap<String, String> = names
        .iter()
        .cloned()
        .zip(payloads.iter().cloned())
        .collect();
    let applied = apply_payloads(&config.template, &values);
    let tab = template_to_tab(idx, &applied);
    match send_tab_request(&tab).await {
        Ok(sent) => {
            let body = sent.response_body.clone();
            let matches = evaluate_matches(&config.match_rules, &body);
            FuzzResult {
                idx,
                payloads,
                method: tab.method,
                url: sent.final_url,
                request_headers: sent.request_headers,
                request_body: sent.request_body,
                status_code: sent.status_code,
                response_headers: sent.response_headers,
                response_body: Some(body),
                response_size: sent.response_size,
                duration_ms: sent.duration_ms,
                error: if sent.status_code == 0 {
                    Some(sent.status_text)
                } else {
                    None
                },
                matches,
            }
        }
        Err(e) => FuzzResult {
            idx,
            payloads,
            method: applied.method.clone(),
            url: applied.url.clone(),
            request_headers: applied.headers.clone(),
            request_body: applied.body.clone(),
            status_code: 0,
            response_headers: Vec::new(),
            response_body: None,
            response_size: 0,
            duration_ms: 0,
            error: Some(e),
            matches: config
                .match_rules
                .iter()
                .map(|r| MatchResult {
                    name: r.name.clone(),
                    value: String::new(),
                })
                .collect(),
        },
    }
}

/// Runs a whole attack: generates requests, sends them with the configured
/// concurrency, streams a summary event per response, and buffers full results.
pub async fn run_attack(
    app: AppHandle,
    state: AppState,
    run_id: i64,
    name: String,
    config: FuzzConfig,
    save: bool,
) -> Result<(), String> {
    let (names, combos) = generate_combos(&config)?;
    let names = Arc::new(names);
    let total = combos.len() as u32;
    let concurrency = config.concurrency.clamp(1, 200) as usize;
    let delay_ms = config.delay_ms;

    state.fuzz_cancel.store(false, Ordering::SeqCst);
    state.fuzz_running.store(true, Ordering::SeqCst);
    {
        let mut buf = state.fuzz_buffer.write().await;
        *buf = Some(FuzzRunBuffer {
            run_id,
            config: config.clone(),
            results: Vec::with_capacity(total as usize),
            saved: false,
        });
    }

    let _ = app.emit(
        "fuzz-started",
        FuzzStartedEvent {
            run_id,
            total,
        },
    );

    let config = Arc::new(config);
    let cancel = state.fuzz_cancel.clone();

    let mut stream = stream::iter(combos.into_iter().enumerate())
        .map(|(i, payloads)| {
            let config = config.clone();
            let cancel = cancel.clone();
            let names = names.clone();
            async move {
                if cancel.load(Ordering::SeqCst) {
                    return None;
                }
                if delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                Some(run_one(i as u32, config, names, payloads).await)
            }
        })
        .buffer_unordered(concurrency);

    let mut completed = 0u32;
    while let Some(result) = stream.next().await {
        let Some(result) = result else { continue };
        completed += 1;

        let _ = app.emit(
            "fuzz-result",
            FuzzResultEvent {
                run_id,
                idx: result.idx,
                payloads: result.payloads.clone(),
                status_code: result.status_code,
                response_size: result.response_size,
                duration_ms: result.duration_ms,
                error: result.error.clone(),
                matches: result.matches.clone(),
            },
        );

        if let Some(buf) = state.fuzz_buffer.write().await.as_mut() {
            buf.results.push(result);
        }
    }

    let stopped = cancel.load(Ordering::SeqCst);
    state.fuzz_running.store(false, Ordering::SeqCst);

    if save && !stopped {
        if let Some(buf) = state.fuzz_buffer.read().await.as_ref() {
            let mut sorted = buf.results.clone();
            sorted.sort_by_key(|r| r.idx);
            super::save_run_db(&state.db_path, run_id, &name, &config, &sorted)?;
        }
        if let Some(buf) = state.fuzz_buffer.write().await.as_mut() {
            buf.saved = true;
        }
    }

    let _ = app.emit(
        "fuzz-done",
        FuzzDoneEvent {
            run_id,
            completed,
            total,
            stopped,
        },
    );
    Ok(())
}
