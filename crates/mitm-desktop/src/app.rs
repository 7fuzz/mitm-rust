use std::sync::Arc;
use crossbeam_channel::Receiver;
use eframe::egui;
use egui_dock::{DockArea, DockState};
use mitm_core::db::InterceptRule;
use mitm_core::events::CoreEvent;
use mitm_core::state::ListenerConfig;
use mitm_core::MitmCore;
use tokio::runtime::Runtime;
use crate::command::{AppCommand, Commands, RequestDraft};
use crate::dock::{self, Tab, TabViewer};
use crate::features::collections::{CollectionsCmd, CollectionsWorker};
use crate::features::fuzzer::FuzzerWorker;
use crate::features::repeater::RepeaterWorker;
use crate::format::{headers_to_text, Exchange};
use crate::state::AppState;
use crate::top_bar;

pub struct MitmApp {
    dock: DockState<Tab>,
    state: AppState,
    cmds: Commands,
    core: Arc<MitmCore>,
    rt: Arc<Runtime>,
    core_events: Receiver<CoreEvent>,
    repeater_worker: RepeaterWorker,
    collections_worker: CollectionsWorker,
    fuzzer_worker: FuzzerWorker,
}

impl MitmApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let rt = Arc::new(Runtime::new().expect("Failed to initialize Tokio runtime"));
        let core = {
            let _guard = rt.enter();
            Arc::new(MitmCore::init().expect("Failed to initialize mitm-core"))
        };

        let mut state = AppState::load(&core.db_path);
        state.proxy.listeners = rt.block_on(core.get_listener_configs());
        state.traffic.set_history_limit(rt.block_on(core.get_history_settings()));
        state.proxy.set_started(rt.block_on(core.start_all_listeners()));

        Self {
            dock: dock::initial_layout(),
            state,
            cmds: Commands::default(),
            core_events: forward_core_events(&rt, &core, &cc.egui_ctx),
            core,
            rt,
            repeater_worker: RepeaterWorker::default(),
            collections_worker: CollectionsWorker::default(),
            fuzzer_worker: FuzzerWorker::default(),
        }
    }

    fn apply_core_event(&mut self, event: CoreEvent) {
        let db_path = &self.core.db_path;
        let traffic = &mut self.state.traffic;
        match event {
            CoreEvent::TrafficCaptured(ev) => {
                let id = ev.entry.id;
                traffic.upsert(ev.entry);
                traffic.refresh_detail_if_selected(id, db_path);
            }
            CoreEvent::TrafficUpdated { id, status, size, duration_ms } => {
                traffic.update_response(id, status, size, duration_ms);
                traffic.refresh_detail_if_selected(id, db_path);
            }
            CoreEvent::TrafficCleared => traffic.clear(),
            CoreEvent::InterceptPrompt(payload) => self.state.intercept.add_pending(payload),
            CoreEvent::InterceptResolved { flow_id } => self.state.intercept.remove_pending(&flow_id),
            _ => {}
        }
    }

    fn run_command(&mut self, ctx: &egui::Context, cmd: AppCommand) {
        let db_path = &self.core.db_path;
        match cmd {
            AppCommand::OpenTab(tab) => dock::open_tab(&mut self.dock, tab),
            AppCommand::ToggleProxy => self.toggle_proxy(),
            AppCommand::SaveListeners(listeners) => self.save_listeners(listeners),

            AppCommand::SelectFlow(id) => {
                self.state.traffic.select(id, db_path);
                self.state.inspector.prefs.open = true;
            }
            AppCommand::ClearTraffic => {
                let _ = self.core.clear_all_traffic();
                self.state.traffic.clear();
            }
            AppCommand::DeleteFlow(id) => {
                let _ = mitm_core::db::delete_history_entry(db_path, id);
                self.state.traffic.remove(id);
            }
            AppCommand::CopyFlow(id, format) => {
                if let Some(flow) = self.state.traffic.flow(id) {
                    let detail = mitm_core::db::get_history_detail(db_path, id).ok().flatten();
                    ctx.copy_text(Exchange::from_flow(flow, detail.as_ref()).copy_text(format));
                }
            }
            AppCommand::SetHistoryLimit(settings) => {
                if self.rt.block_on(self.core.set_history_settings(settings.clone())).is_ok() {
                    self.state.traffic.set_history_limit(settings);
                    self.state.traffic.load_from_db(db_path);
                }
            }
            AppCommand::AddInterceptRuleForHost(host) => {
                let rules = &mut self.state.intercept.rules;
                rules.push(InterceptRule {
                    id: uuid::Uuid::new_v4().to_string(),
                    is_enabled: true,
                    target_phase: "request".to_string(),
                    match_field: "host".to_string(),
                    operator: "equals".to_string(),
                    match_value: host,
                    order_index: rules.len() as i32,
                    created_at_ms: chrono::Utc::now().timestamp_millis(),
                    action: "intercept".to_string(),
                });
                let rules = rules.clone();
                let core = Arc::clone(&self.core);
                self.rt.spawn(async move {
                    let _ = core.update_intercept_rules(rules).await;
                });
                dock::open_tab(&mut self.dock, Tab::Intercept);
            }
            AppCommand::SendFlowToRepeater(id) => {
                if let Some(flow) = self.state.traffic.flow(id).cloned() {
                    let detail = mitm_core::db::get_history_detail(db_path, id).ok().flatten();
                    self.state.repeater.create_from_flow(&flow, detail.as_ref(), db_path);
                    dock::open_tab(&mut self.dock, Tab::Repeater);
                }
            }
            AppCommand::SendFlowToFuzzer(id) => {
                if let Some(draft) = self.flow_draft(id) {
                    self.state.fuzzer.load_draft(draft);
                    dock::open_tab(&mut self.dock, Tab::Fuzzer);
                }
            }

            AppCommand::RepeaterNew => self.state.repeater.create_blank(db_path),
            AppCommand::RepeaterSelect(id) => self.state.repeater.select(id, db_path),
            AppCommand::RepeaterDelete(id) => self.state.repeater.delete(&id, db_path),
            AppCommand::RepeaterSend(id) => {
                let repeater = &mut self.state.repeater;
                repeater.flush(db_path);
                if let Some(tab) = repeater.tabs.iter().find(|t| t.id == id).cloned() {
                    repeater.executing.insert(id);
                    self.repeater_worker.send(&self.rt, ctx, db_path.clone(), tab);
                }
            }
            AppCommand::StartFuzzer { template, payloads, concurrency } => {
                self.fuzzer_worker.start(&self.rt, ctx, template, payloads, concurrency);
            }

            AppCommand::ForwardIntercept { flow_id, url, method, headers, body } => {
                let _ = self.core.forward_intercepted_flow(&flow_id, url, method, headers, body);
                self.state.intercept.remove_pending(&flow_id);
            }
            AppCommand::DropIntercept { flow_id } => {
                let _ = self.core.drop_intercepted_flow(&flow_id);
                self.state.intercept.remove_pending(&flow_id);
            }
            AppCommand::ConfigureInterceptor { enabled, mode } => {
                let core = Arc::clone(&self.core);
                self.rt.spawn(async move {
                    let _ = core.toggle_interceptor(enabled, mode).await;
                });
            }
            AppCommand::SaveInterceptRules(rules) => {
                let core = Arc::clone(&self.core);
                self.rt.spawn(async move {
                    let _ = core.update_intercept_rules(rules).await;
                });
            }

            AppCommand::Collections(CollectionsCmd::Execute(id)) => {
                if self.state.collections.start_run(&id, db_path) {
                    self.collections_worker.execute(&self.rt, ctx, db_path.clone(), id);
                }
            }
            AppCommand::Collections(cmd) => self.state.collections.apply(cmd, db_path),
            AppCommand::ReloadWsConnections => self.state.websockets.load_connections(db_path),
            AppCommand::SelectWsConnection(id) => self.state.websockets.select(id, db_path),
        }
    }

    fn flow_draft(&self, id: i64) -> Option<RequestDraft> {
        let flow = self.state.traffic.flow(id)?;
        let detail = mitm_core::db::get_history_detail(&self.core.db_path, id).ok().flatten();
        let (headers, body) = match &detail {
            Some(d) => (&d.request_headers, &d.request_body),
            None => (&flow.request_headers, &flow.request_body),
        };
        Some(RequestDraft {
            method: flow.method.clone(),
            url: flow.url.clone(),
            headers: headers_to_text(headers),
            body: body.clone(),
        })
    }

    fn toggle_proxy(&mut self) {
        if self.state.proxy.running {
            self.rt.block_on(self.core.stop_all_listeners());
            self.state.proxy.running = false;
        } else {
            self.state.proxy.set_started(self.rt.block_on(self.core.start_all_listeners()));
        }
    }

    fn save_listeners(&mut self, listeners: Vec<ListenerConfig>) {
        let was_running = self.state.proxy.running;
        let core = &self.core;
        let result = self.rt.block_on(async {
            // Stopped first so set_listener_configs doesn't restart them itself; the restart below reports what bound.
            core.stop_all_listeners().await;
            core.set_listener_configs(listeners).await?;
            if was_running {
                core.start_all_listeners().await.map(Some)
            } else {
                Ok(None)
            }
        });
        match result {
            Ok(Some(started)) => self.state.proxy.set_started(Ok(started)),
            Ok(None) => {}
            Err(e) => self.state.proxy.error = Some(e),
        }
    }
}

impl eframe::App for MitmApp {
    fn on_exit(&mut self) {
        self.state.repeater.flush(&self.core.db_path);
        self.state.collections.flush(&self.core.db_path);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok(event) = self.core_events.try_recv() {
            self.apply_core_event(event);
        }
        let now = ui.input(|i| i.time);
        self.repeater_worker.poll(&mut self.state.repeater, &self.core.db_path);
        self.state.repeater.flush_if_idle(&self.core.db_path, now);
        self.collections_worker.poll(&mut self.state.collections, &self.core.db_path);
        self.state.collections.flush_if_idle(&self.core.db_path, now);
        self.fuzzer_worker.poll(&mut self.state.fuzzer);

        egui::Panel::top("top_panel").show(ui, |ui| {
            top_bar::show(ui, &self.state, &self.dock, &mut self.cmds);
        });

        let mut viewer = TabViewer {
            state: &mut self.state,
            cmds: &mut self.cmds,
            db_path: &self.core.db_path,
        };
        DockArea::new(&mut self.dock).show_inside(ui, &mut viewer);

        let cmds = self.cmds.take();
        if !cmds.is_empty() {
            let ctx = ui.ctx().clone();
            for cmd in cmds {
                self.run_command(&ctx, cmd);
            }
            ctx.request_repaint();
        }
    }
}

fn forward_core_events(rt: &Runtime, core: &MitmCore, ctx: &egui::Context) -> Receiver<CoreEvent> {
    let (tx, rx) = crossbeam_channel::unbounded();
    let mut core_rx = core.event_bus.subscribe();
    let ctx = ctx.clone();
    rt.spawn(async move {
        while let Ok(event) = core_rx.recv().await {
            let _ = tx.send(event);
            ctx.request_repaint();
        }
    });
    rx
}
