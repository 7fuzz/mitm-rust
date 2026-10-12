use std::sync::Arc;
use crossbeam_channel::Receiver;
use eframe::egui;
use mitm_core::events::CoreEvent;
use mitm_core::MitmCore;
use tokio::runtime::Runtime;
use tokio::sync::broadcast::error::RecvError;
use crate::ui::theme::Tone;
use super::prefs;
use super::state::AppState;
use super::MitmApp;

impl MitmApp {
    pub(super) fn finish_factory_reset(&mut self, ctx: &egui::Context, result: Result<Option<MitmCore>, String>) {
        let fresh = match result {
            Ok(fresh) => fresh,
            Err(e) => {
                self.state.data.resetting = false;
                self.state.data.status = Some((e, Tone::Danger));
                return;
            }
        };
        if let Some(core) = fresh {
            self.core = Arc::new(core);
            self.core_events = forward_core_events(&self.rt, &self.core, ctx);
        }
        let mut state = startup_state(&self.rt, &self.core);
        prefs::carry_over(&mut self.state, &mut state);
        self.state = state;
        self.state.data.status = Some(("Factory reset complete".to_string(), Tone::Success));
    }
}

pub(super) fn startup_state(rt: &Runtime, core: &MitmCore) -> AppState {
    let mut state = AppState::load(&core.db);
    state.traffic.set_history_limit(rt.block_on(core.get_history_settings()));
    state.proxy.mode = rt.block_on(core.proxy_mode());
    if state.proxy.mode.is_enabled() {
        state.proxy.set_started(rt.block_on(core.start_all_listeners()));
    }
    state.proxy.apply_statuses(&rt.block_on(core.listener_status()));
    state.proxy.reload_host_ips();
    state.certificate.info = core.ca_info().ok();
    let (enabled, mode, scope) = rt.block_on(core.intercept_config());
    state.intercept.enabled = enabled;
    state.intercept.mode = mode;
    state.intercept.source_scope = scope;
    state.rewrite.rules = rt.block_on(core.rewrite_rules());
    state.rewrite.enabled = core.rewrite_enabled();
    state.rewrite.scope = rt.block_on(core.rewrite_source_scope());
    state.websockets.mitm_enabled = rt.block_on(core.ws_mitm_enabled());
    let webhook = rt.block_on(core.webhook_status());
    state.webhooks.running = webhook.is_running;
    state.webhooks.port = webhook.port;
    state
}

pub(super) fn forward_core_events(rt: &Runtime, core: &MitmCore, ctx: &egui::Context) -> Receiver<CoreEvent> {
    let (tx, rx) = crossbeam_channel::unbounded();
    let mut core_rx = core.event_bus.subscribe();
    let ctx = ctx.clone();
    rt.spawn(async move {
        loop {
            match core_rx.recv().await {
                Ok(event) => {
                    let _ = tx.send(event);
                    ctx.request_repaint();
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });
    rx
}
