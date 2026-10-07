use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use mitm_core::fuzzer::FuzzTemplate;
use mitm_core::repeater::{HeaderItem, RepeaterTab};
use super::{FuzzResultRow, FuzzerState};

enum FuzzMsg {
    Result(FuzzResultRow),
    Done,
}

pub struct FuzzerWorker {
    tx: Sender<FuzzMsg>,
    rx: Receiver<FuzzMsg>,
}

impl Default for FuzzerWorker {
    fn default() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self { tx, rx }
    }
}

impl FuzzerWorker {
    pub fn start(
        &self,
        rt: &tokio::runtime::Runtime,
        ctx: &egui::Context,
        template: FuzzTemplate,
        payloads: Vec<String>,
        concurrency: usize,
    ) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        rt.spawn(async move {
            let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
            let handles: Vec<_> = payloads
                .into_iter()
                .enumerate()
                .map(|(idx, payload)| {
                    let semaphore = Arc::clone(&semaphore);
                    let template = template.clone();
                    let tx = tx.clone();
                    let ctx = ctx.clone();
                    tokio::spawn(async move {
                        let _permit = semaphore.acquire().await.ok();
                        let row = run_payload(&template, idx, payload).await;
                        let _ = tx.send(FuzzMsg::Result(row));
                        ctx.request_repaint();
                    })
                })
                .collect();

            for handle in handles {
                let _ = handle.await;
            }
            let _ = tx.send(FuzzMsg::Done);
            ctx.request_repaint();
        });
    }

    pub fn poll(&self, state: &mut FuzzerState) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                FuzzMsg::Result(row) => {
                    state.progress_completed += 1;
                    state.results.push(row);
                }
                FuzzMsg::Done => state.is_running = false,
            }
        }
    }
}

async fn run_payload(template: &FuzzTemplate, idx: usize, payload: String) -> FuzzResultRow {
    let request = substitute(template, idx, &payload);
    match mitm_core::repeater::execute::send_tab_request(&request).await {
        Ok(res) => FuzzResultRow {
            index: idx + 1,
            payload,
            status_code: res.status_code,
            size: res.response_size,
            duration_ms: res.duration_ms,
            error: None,
        },
        Err(e) => FuzzResultRow {
            index: idx + 1,
            payload,
            status_code: 0,
            size: 0,
            duration_ms: 0,
            error: Some(e),
        },
    }
}

fn substitute(template: &FuzzTemplate, idx: usize, payload: &str) -> RepeaterTab {
    let re = mitm_core::fuzzer::marker_re();
    let now = chrono::Utc::now().timestamp_millis();
    RepeaterTab {
        id: format!("fuzz-{}", idx),
        method: template.method.clone(),
        url: re.replace_all(&template.url, payload).to_string(),
        headers: template
            .headers
            .iter()
            .map(|h| HeaderItem {
                id: h.id.clone(),
                key: h.key.clone(),
                value: re.replace_all(&h.value, payload).to_string(),
                enabled: h.enabled,
            })
            .collect(),
        params: Vec::new(),
        body_type: template.body_type.clone(),
        body_content: template.body.as_ref().map(|b| re.replace_all(b, payload).to_string()),
        extract_rules: Vec::new(),
        pre_requests: Vec::new(),
        post_requests: Vec::new(),
        order_index: idx as i32,
        created_at_ms: now,
        updated_at_ms: now,
        execution_count: 0,
        last_status_code: None,
        last_duration_ms: None,
    }
}
