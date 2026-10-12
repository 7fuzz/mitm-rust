use std::sync::Arc;
use mitm_core::webhook::WebhookReplayResult;
use mitm_core::MitmCore;
use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use crate::app::jobs::Jobs;
use super::WebhooksState;

#[derive(Clone)]
pub enum Cmd {
    StartReceiver,
    StopReceiver,
    SetPort(u16),
    CreateEndpoint { name: String, path: String, secret: String },
    DeleteEndpoint(String),
    CopyTestCurl(String),
    ClearDeliveries,
    Replay { id: i64, url: String },
}

impl From<Cmd> for AppCommand {
    fn from(cmd: Cmd) -> Self {
        AppCommand::Webhooks(cmd)
    }
}

#[derive(Default)]
pub struct WebhookJobs {
    /// The switch result and whether the receiver runs afterwards.
    receiver: Jobs<(Result<(), String>, bool)>,
    replays: Jobs<(i64, Result<WebhookReplayResult, String>)>,
}

pub fn handle(state: &mut WebhooksState, cx: &Cx, cmd: Cmd) {
    match cmd {
        Cmd::StartReceiver | Cmd::StopReceiver if state.switching => {}
        Cmd::StartReceiver => {
            let port = state.port;
            switch_receiver(state, cx, move |core| async move { core.start_webhook_receiver(port).await });
        }
        Cmd::StopReceiver => switch_receiver(state, cx, |core| async move { core.stop_webhook_receiver().await }),
        // Only stores the port while stopped, and the drag control sends every step, so it stays in order here.
        Cmd::SetPort(port) => {
            cx.report(cx.rt.block_on(cx.core.set_webhook_port(port)));
        }
        Cmd::CreateEndpoint { name, path, secret } => {
            if let Some(endpoint) = cx.report(cx.core.create_webhook_endpoint(name, path, secret)) {
                state.created(endpoint);
            }
        }
        Cmd::DeleteEndpoint(id) => {
            if cx.report(cx.core.delete_webhook_endpoint(&id)).is_some() {
                state.removed(&id);
            }
        }
        Cmd::ClearDeliveries => {
            if cx.report(cx.core.clear_webhook_deliveries()).is_some() {
                state.deliveries_cleared();
            }
        }
        Cmd::CopyTestCurl(id) => {
            if let Some(curl) = test_curl(state, cx.core, &id) {
                cx.ctx.copy_text(curl);
            }
        }
        Cmd::Replay { id, url } => {
            let Some(delivery) = state.deliveries.iter().find(|d| d.id == id).cloned() else { return };
            let core = Arc::clone(cx.core);
            state.jobs.replays.spawn(cx.rt, cx.ctx, async move { (id, core.replay_webhook(delivery, &url).await) });
        }
    }
}

pub fn poll(state: &mut WebhooksState, cx: &Cx) {
    for (result, running) in state.jobs.receiver.drain() {
        state.switching = false;
        state.running = running;
        cx.report(result);
    }
    let replays: Vec<_> = state.jobs.replays.drain().collect();
    for (id, result) in replays {
        state.replay_done(id, result);
    }
}

fn switch_receiver<F, Fut>(state: &mut WebhooksState, cx: &Cx, switch: F)
where
    F: FnOnce(Arc<MitmCore>) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
{
    state.switching = true;
    let core = Arc::clone(cx.core);
    let job = switch(Arc::clone(&core));
    state.jobs.receiver.spawn(cx.rt, cx.ctx, async move {
        let result = job.await;
        (result, core.webhook_status().await.is_running)
    });
}

fn test_curl(state: &WebhooksState, core: &MitmCore, id: &str) -> Option<String> {
    let endpoint = state.endpoints.iter().find(|e| e.id == id)?;
    let body = r#"{"event":"test","message":"hello"}"#;
    let mut curl = format!("curl -X POST '{}' \\\n  -H 'Content-Type: application/json'", state.endpoint_url(&endpoint.path));
    if !endpoint.secret_key.trim().is_empty() {
        let sig = core.calculate_webhook_signature(&endpoint.secret_key, body, "github");
        curl.push_str(&format!(" \\\n  -H '{}: {}'", sig.header_name, sig.header_value));
    }
    curl.push_str(&format!(" \\\n  -d '{}'", body));
    Some(curl)
}
