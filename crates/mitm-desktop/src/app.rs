use std::path::PathBuf;
use std::sync::Arc;
use crossbeam_channel::Receiver;
use eframe::egui;
use egui_dock::{DockArea, DockState};
use mitm_core::db::InterceptRule;
use mitm_core::events::CoreEvent;
use mitm_core::state::{ListenerConfig, ProxyMode};
use mitm_core::MitmCore;
use tokio::runtime::Runtime;
use crate::command::{AppCommand, Commands, RequestDraft};
use crate::dock::{self, Tab, TabViewer};
use mitm_core::collections::execute::ExecutionResult;
use mitm_core::repeater::RepeaterExecutionResult;
use crate::features::collections::CollectionsCmd;
use crate::features::fuzzer::FuzzerWorker;
use crate::features::settings::ProxyState;
use crate::format::{headers_to_text, Exchange};
use crate::jobs::Jobs;
use crate::state::AppState;
use crate::palette::{self, PaletteState};
use crate::theme::Tone;
use crate::{prefs, shortcuts, top_bar};

pub struct MitmApp {
    dock: DockState<Tab>,
    state: AppState,
    cmds: Commands,
    palette: PaletteState,
    core: Arc<MitmCore>,
    rt: Arc<Runtime>,
    core_events: Receiver<CoreEvent>,
    repeater_runs: Jobs<(String, Result<RepeaterExecutionResult, String>)>,
    collection_runs: Jobs<(String, Result<ExecutionResult, String>)>,
    ca_jobs: Jobs<Option<Result<PathBuf, String>>>,
    fuzzer_worker: FuzzerWorker,
}

impl MitmApp {
    pub fn new(ctx: &egui::Context, storage: Option<&dyn eframe::Storage>, rt: Arc<Runtime>, core: Arc<MitmCore>) -> Self {
        let mut state = AppState::load(&core.db_path);
        state.traffic.set_history_limit(rt.block_on(core.get_history_settings()));
        state.proxy.mode = rt.block_on(core.proxy_mode());
        if state.proxy.mode.is_enabled() {
            state.proxy.set_started(rt.block_on(core.start_all_listeners()));
        }
        refresh_listener_state(&rt, &core, &mut state.proxy);
        state.proxy.reload_host_ips();
        state.certificate.info = core.ca_info().ok();

        Self {
            dock: prefs::restore(storage, &mut state),
            state,
            cmds: Commands::default(),
            palette: PaletteState::default(),
            core_events: forward_core_events(&rt, &core, ctx),
            core,
            rt,
            repeater_runs: Jobs::default(),
            collection_runs: Jobs::default(),
            ca_jobs: Jobs::default(),
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
        // These reload environments from the DB, and a later flush of a stale copy would undo the switch.
        if cmd.touches_environments() {
            self.state.environments.flush(db_path);
        }
        match cmd {
            AppCommand::OpenPalette => self.palette.open(),
            AppCommand::OpenTab(tab) => dock::open_tab(&mut self.dock, tab),
            AppCommand::FocusTabAt(index) => dock::focus_tab_at(&mut self.dock, index),
            AppCommand::CycleTab { forward } => dock::cycle_tab(&mut self.dock, forward),
            AppCommand::ToggleProxy => {
                let mode = if self.state.proxy.mode.is_enabled() { ProxyMode::Off } else { ProxyMode::On };
                self.set_proxy_mode(mode);
            }
            AppCommand::SetProxyMode(mode) => self.set_proxy_mode(mode),
            AppCommand::SaveListeners(listeners) => self.save_listeners(listeners),
            AppCommand::SetListenerEnabled { id, enabled } => {
                let _ = self.rt.block_on(self.core.set_listener_enabled(id, enabled));
                self.refresh_listeners();
            }
            AppCommand::ExportCa(format) => {
                let core = Arc::clone(&self.core);
                let (file_name, filter_name, ext) = match format {
                    mitm_core::CaFormat::Pem => ("mitm-ca.crt", "PEM certificate", "crt"),
                    mitm_core::CaFormat::Der => ("mitm-ca.cer", "DER certificate", "cer"),
                };
                self.ca_jobs.spawn(&self.rt, ctx, async move {
                    let path = crate::files::pick_save(file_name, &[(filter_name, &[ext])]).await?;
                    let res = core.export_ca(&path, format).map(|_| path);
                    Some(res)
                });
            }
            AppCommand::CopyCaPem => {
                if let Ok(pem) = self.core.ca_pem() {
                    ctx.copy_text(pem);
                }
            }
            AppCommand::OpenCaFolder => {
                if let Ok(info) = self.core.ca_info() {
                    let _ = crate::files::open_external(&info.ca_dir);
                }
            }
            AppCommand::RegenerateCa => {
                match self.rt.block_on(self.core.regenerate_ca()) {
                    Ok(()) => {
                        self.state.certificate.info = self.core.ca_info().ok();
                        self.state.certificate.status = Some(("CA certificate regenerated".to_string(), Tone::Success));
                    }
                    Err(e) => {
                        self.state.certificate.status = Some((e, Tone::Danger));
                    }
                }
            }
            AppCommand::ClearDataCategory(category) => {
                match self.core.clear_data_category(category) {
                    Ok(()) => {
                        if category == mitm_core::DataCategory::Traffic {
                            self.state.traffic.clear();
                        }
                        self.state.data.counts = self.core.data_counts().ok();
                        self.state.data.status = Some((format!("{} cleared", category.display_name()), Tone::Success));
                    }
                    Err(e) => {
                        self.state.data.status = Some((e, Tone::Danger));
                    }
                }
            }
            AppCommand::RefreshDataCounts => {
                self.state.data.counts = self.core.data_counts().ok();
            }
            AppCommand::FactoryReset => {
                match self.rt.block_on(self.core.factory_reset()) {
                    Ok(()) => {
                        let event_bus = mitm_core::EventBus::default();
                        if let Ok(new_core) = MitmCore::init_with_events(event_bus) {
                            self.core = Arc::new(new_core);
                            self.core_events = forward_core_events(&self.rt, &self.core, ctx);
                        }
                        self.state = AppState::load(&self.core.db_path);
                        self.state.traffic.set_history_limit(self.rt.block_on(self.core.get_history_settings()));
                        self.state.proxy.mode = self.rt.block_on(self.core.proxy_mode());
                        if self.state.proxy.mode.is_enabled() {
                            self.state.proxy.set_started(self.rt.block_on(self.core.start_all_listeners()));
                        }
                        refresh_listener_state(&self.rt, &self.core, &mut self.state.proxy);
                        self.state.proxy.reload_host_ips();
                        self.state.certificate.info = self.core.ca_info().ok();
                        self.state.data.counts = self.core.data_counts().ok();
                        self.state.data.status = Some(("Factory reset complete".to_string(), Tone::Success));
                    }
                    Err(e) => {
                        self.state.data.status = Some((e, Tone::Danger));
                    }
                }
            }

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
                    let db_path = db_path.clone();
                    self.repeater_runs.spawn(&self.rt, ctx, async move {
                        let result = mitm_core::repeater::execute::execute_repeater_tab(&db_path, &tab).await;
                        (tab.id, result)
                    });
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
            AppCommand::ToggleInterceptor => {
                let intercept = &mut self.state.intercept;
                intercept.enabled = !intercept.enabled;
                let cmd = AppCommand::ConfigureInterceptor { enabled: intercept.enabled, mode: intercept.mode.clone() };
                self.run_command(ctx, cmd);
            }
            AppCommand::SaveInterceptRules(rules) => {
                let core = Arc::clone(&self.core);
                self.rt.spawn(async move {
                    let _ = core.update_intercept_rules(rules).await;
                });
            }

            AppCommand::Collections(CollectionsCmd::Execute(id)) => {
                if self.state.collections.start_run(&id, db_path) {
                    let db_path = db_path.clone();
                    self.collection_runs.spawn(&self.rt, ctx, async move {
                        let result = mitm_core::collections::execute::execute_collection_request_db(&db_path, &id).await;
                        (id, result)
                    });
                }
            }
            AppCommand::Collections(cmd) => self.state.collections.apply(cmd, db_path),
            AppCommand::ReloadWsConnections => self.state.websockets.load_connections(db_path),
            AppCommand::SelectWsConnection(id) => self.state.websockets.select(id, db_path),
            AppCommand::SelectWorkspace(id) => {
                let _ = mitm_core::workspace::set_active_workspace_db(db_path, &id);
                self.state.environments.active_workspace_id = id.clone();
                self.state.environments.reload_environments(db_path);
                self.state.collections.apply(CollectionsCmd::SelectWorkspace(id), db_path);
            }
            AppCommand::SelectEnvironment(env_id) => {
                let ws_id = self.state.environments.active_workspace_id.clone();
                let _ = mitm_core::workspace::set_active_environment_db(db_path, &ws_id, env_id.as_deref());
                self.state.environments.reload_environments(db_path);
            }
            AppCommand::CreateWorkspace(name) => {
                if let Ok(ws) = mitm_core::workspace::create_workspace_db(db_path, name, None) {
                    self.state.environments.workspaces.push(ws.clone());
                    self.run_command(ctx, AppCommand::SelectWorkspace(ws.id));
                }
            }
            AppCommand::RenameWorkspace { id, name } => {
                let _ = mitm_core::workspace::rename_workspace_db(db_path, &id, &name);
                if let Some(ws) = self.state.environments.workspaces.iter_mut().find(|w| w.id == id) {
                    ws.name = name;
                }
            }
            AppCommand::UpdateWorkspaceDescription { id, description } => {
                let _ = mitm_core::workspace::update_workspace_description_db(db_path, &id, description.as_deref());
                if let Some(ws) = self.state.environments.workspaces.iter_mut().find(|w| w.id == id) {
                    ws.description = description;
                }
            }
            AppCommand::DeleteWorkspace(id) => {
                let _ = mitm_core::workspace::delete_workspace_db(db_path, &id);
                self.state.environments.workspaces.retain(|w| w.id != id);
                if self.state.environments.active_workspace_id == id {
                    if let Some(first) = self.state.environments.workspaces.first().cloned() {
                        self.run_command(ctx, AppCommand::SelectWorkspace(first.id));
                    } else if let Ok(ws) = mitm_core::workspace::create_workspace_db(db_path, "Default Workspace".to_string(), None) {
                        self.state.environments.workspaces.push(ws.clone());
                        self.run_command(ctx, AppCommand::SelectWorkspace(ws.id));
                    }
                }
            }
            AppCommand::CreateEnvironment { workspace_id, name } => {
                let now = chrono::Utc::now().timestamp_millis();
                let new_env = mitm_core::workspace::Environment {
                    id: uuid::Uuid::new_v4().to_string(),
                    workspace_id,
                    name,
                    is_active: false,
                    variables: Vec::new(),
                    created_at_ms: now,
                    updated_at_ms: now,
                };
                let id = new_env.id.clone();
                let _ = mitm_core::workspace::save_workspace_environment_db(db_path, new_env);
                self.state.environments.reload_environments(db_path);
                self.state.environments.selected_environment_id = Some(id);
            }
            AppCommand::RenameEnvironment { id, name } => {
                if let Ok(Some(mut env)) = mitm_core::workspace::get_workspace_environment_db(db_path, &id) {
                    env.name = name;
                    let _ = mitm_core::workspace::save_workspace_environment_db(db_path, env);
                    self.state.environments.reload_environments(db_path);
                }
            }
            AppCommand::DuplicateEnvironment(id) => {
                if let Ok(dup) = mitm_core::workspace::duplicate_workspace_environment_db(db_path, &id, None) {
                    self.state.environments.reload_environments(db_path);
                    self.state.environments.selected_environment_id = Some(dup.id);
                }
            }
            AppCommand::DeleteEnvironment(id) => {
                let _ = mitm_core::workspace::delete_workspace_environment_db(db_path, &id);
                self.state.environments.reload_environments(db_path);
                if self.state.environments.selected_environment_id.as_deref() == Some(&id) {
                    self.state.environments.selected_environment_id = self.state.environments.environments.first().map(|e| e.id.clone());
                }
            }
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

    fn set_proxy_mode(&mut self, mode: ProxyMode) {
        let proxy = &mut self.state.proxy;
        match self.rt.block_on(self.core.set_proxy_mode(mode)) {
            Ok(started) => {
                proxy.mode = mode;
                match started {
                    Some(started) => proxy.set_started(Ok(started)),
                    None => proxy.running &= mode.is_enabled(),
                }
            }
            Err(e) => proxy.error = Some(e),
        }
        self.refresh_listeners();
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
        self.refresh_listeners();
    }

    fn refresh_listeners(&mut self) {
        refresh_listener_state(&self.rt, &self.core, &mut self.state.proxy);
    }
}

impl eframe::App for MitmApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        prefs::persist(storage, &self.dock, &self.state);
    }

    fn on_exit(&mut self) {
        self.state.repeater.flush(&self.core.db_path);
        self.state.collections.flush(&self.core.db_path);
        self.state.environments.flush(&self.core.db_path);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::widgets::template::set_active_env(self.state.environments.active_environment());
        while let Ok(event) = self.core_events.try_recv() {
            self.apply_core_event(event);
        }
        shortcuts::consume(ui.ctx(), &mut self.cmds);
        let now = ui.input(|i| i.time);
        for (id, result) in self.repeater_runs.drain() {
            self.state.repeater.finish_run(&id, result, &self.core.db_path);
        }
        self.state.repeater.flush_if_idle(&self.core.db_path, now);
        for (id, result) in self.collection_runs.drain() {
            self.state.collections.finish_run(&id, result, &self.core.db_path);
        }
        self.state.collections.flush_if_idle(&self.core.db_path, now);
        self.state.environments.flush_if_idle(&self.core.db_path, now);
        self.fuzzer_worker.poll(&mut self.state.fuzzer);
        for result in self.ca_jobs.drain().flatten() {
            self.state.certificate.status = Some(match result {
                Ok(path) => (format!("Exported to {}", path.display()), Tone::Success),
                Err(e) => (e, Tone::Danger),
            });
        }

        egui::Panel::top("top_panel").show(ui, |ui| {
            top_bar::show(ui, &mut self.state, &self.dock, &mut self.cmds);
        });

        let mut viewer = TabViewer {
            state: &mut self.state,
            cmds: &mut self.cmds,
            db_path: &self.core.db_path,
        };
        DockArea::new(&mut self.dock).show_inside(ui, &mut viewer);
        palette::show(ui.ctx(), &mut self.palette, &self.state, &mut self.cmds);

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

fn refresh_listener_state(rt: &Runtime, core: &MitmCore, proxy: &mut ProxyState) {
    let statuses = rt.block_on(core.listener_status());
    proxy.listeners = statuses.iter().map(|s| s.config.clone()).collect();
    proxy.running_listeners = statuses.iter().filter(|s| s.running).map(|s| s.config.id).collect();
    proxy.listener_errors = statuses.iter().filter_map(|s| Some((s.config.id, s.error.clone()?))).collect();
    if proxy.running {
        let running_summaries: Vec<_> = statuses
            .iter()
            .filter(|s| s.running)
            .map(|s| format!("{} ({})", s.config.label, s.config.address))
            .collect();
        proxy.active_summary = running_summaries.join("  |  ");
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
