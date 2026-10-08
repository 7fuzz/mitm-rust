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
use tokio::sync::broadcast::error::RecvError;
use crate::command::{AppCommand, Commands};
use crate::dock::{self, Tab, TabViewer};
use mitm_core::collections::execute::ExecutionResult;
use mitm_core::repeater::RepeaterExecutionResult;
use crate::features::collections::CollectionsCmd;
use crate::features::request_editor::captured::CapturedRequest;
use crate::features::environments::{self, QuickVarScope};
use crate::features::settings::ProxyState;
use crate::format::Exchange;
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
    import_files: Jobs<Option<Result<(String, String), String>>>,
    file_saves: Jobs<Option<Result<String, String>>>,
    fuzz_search: Jobs<(i64, String, Vec<u32>)>,
    ws_connects: Jobs<Result<mitm_core::state::WebSocketConn, String>>,
    ws_actions: Jobs<Result<(), String>>,
}

impl MitmApp {
    pub fn new(ctx: &egui::Context, storage: Option<&dyn eframe::Storage>, rt: Arc<Runtime>, core: Arc<MitmCore>) -> Self {
        let mut state = startup_state(&rt, &core);
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
            import_files: Jobs::default(),
            file_saves: Jobs::default(),
            fuzz_search: Jobs::default(),
            ws_connects: Jobs::default(),
            ws_actions: Jobs::default(),
        }
    }

    fn apply_core_event(&mut self, ctx: &egui::Context, event: CoreEvent) {
        let db_path = &self.core.db_path;
        let traffic = &mut self.state.traffic;
        match event {
            CoreEvent::FuzzResult { run_id, row } => self.state.fuzzer.push_row(run_id, row),
            CoreEvent::FuzzFinished { run_id, completed, stopped, .. } => {
                let core = &self.core;
                self.state.fuzzer.finished(run_id, completed, stopped, || core.fuzz_rows(run_id).unwrap_or_default());
            }
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
            CoreEvent::InterceptPrompt(payload) => {
                self.state.intercept.add_pending(payload);
                if self.state.intercept.prefs.raise_window {
                    dock::open_tab(&mut self.dock, Tab::Intercept);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                }
            }
            CoreEvent::InterceptResolved { flow_id } => self.state.intercept.remove_pending(&flow_id),
            CoreEvent::WsFrameReceived { frame, .. } => self.state.websockets.apply_frame(frame),
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
            AppCommand::ToggleQuickVariables => {
                let scope = match self.dock.find_active_focused() {
                    Some((_, Tab::Repeater)) => QuickVarScope::Repeater,
                    _ => QuickVarScope::Workspace,
                };
                let environments = &mut self.state.environments;
                if scope == QuickVarScope::Workspace && environments.active_environment().is_none() {
                    dock::open_tab(&mut self.dock, Tab::Environments);
                } else {
                    environments.quick.toggle(scope);
                }
            }
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
                if let Err(e) = self.rt.block_on(self.core.set_listener_enabled(id, enabled)) {
                    self.state.proxy.error = Some(e);
                }
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
                self.refresh_listeners();
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
                        let mut state = startup_state(&self.rt, &self.core);
                        prefs::carry_over(&mut self.state, &mut state);
                        self.state = state;
                        self.state.data.status = Some(("Factory reset complete".to_string(), Tone::Success));
                    }
                    Err(e) => {
                        self.state.data.status = Some((e, Tone::Danger));
                    }
                }
            }

            AppCommand::LoadOlderFlows => self.state.traffic.load_older(db_path),
            AppCommand::SaveFlowToCollection { flow_id, target } => {
                if let Some(flow) = self.state.traffic.flow(flow_id) {
                    let detail = mitm_core::db::get_history_detail(db_path, flow_id).ok().flatten();
                    let captured = CapturedRequest::from_flow(flow, detail.as_ref());
                    let name = format!("{} {}", flow.method, if flow.path.is_empty() { &flow.url } else { &flow.path });
                    let _ = self.state.collections.save_captured(db_path, target, name, captured);
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
            AppCommand::CopyFlowCustom(id, options) => {
                if let Some(flow) = self.state.traffic.flow(id) {
                    let detail = mitm_core::db::get_history_detail(db_path, id).ok().flatten();
                    ctx.copy_text(Exchange::from_flow(flow, detail.as_ref()).custom_text(&options));
                }
            }
            AppCommand::SaveBody(body) => {
                self.file_saves.spawn(&self.rt, ctx, async move {
                    let path = crate::files::pick_save(&body.file_name, &[]).await?;
                    let result = std::fs::write(&path, &body.bytes).map_err(|e| e.to_string());
                    Some(result.map(|_| format!("Saved {}", crate::files::file_name(&path))))
                });
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
                if let Some(flow) = self.state.traffic.flow(id).cloned() {
                    let detail = mitm_core::db::get_history_detail(db_path, id).ok().flatten();
                    let captured = CapturedRequest::from_flow(&flow, detail.as_ref());
                    self.state.fuzzer.load_capture(captured);
                    dock::open_tab(&mut self.dock, Tab::Fuzzer);
                }
            }

            AppCommand::RepeaterNew => self.state.repeater.create_blank(db_path),
            AppCommand::OpenCurlImport => self.state.repeater.curl_import.open(),
            AppCommand::RepeaterImportCurl(curl) => {
                self.state.repeater.create_from_curl(curl, db_path);
                dock::open_tab(&mut self.dock, Tab::Repeater);
            }
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
            AppCommand::StartFuzzer(config) => {
                let start = self.rt.block_on(self.core.start_fuzz(config));
                self.state.fuzzer.started(start);
            }
            AppCommand::StopFuzzer => self.core.stop_fuzz(),
            AppCommand::SelectFuzzResult { run_id, idx } => {
                self.state.fuzzer.select_result(run_id, idx, db_path);
            }
            AppCommand::SendFuzzResultToRepeater { run_id, idx } => {
                let result = if self.state.fuzzer.selected_result.as_ref().is_some_and(|r| r.idx == idx && self.state.fuzzer.run_id == Some(run_id)) {
                    self.state.fuzzer.selected_result.clone()
                } else {
                    mitm_core::fuzzer::get_result_db(db_path, run_id, idx).ok().flatten()
                };
                if let Some(result) = result {
                    self.state.repeater.create_from_fuzz_result(&result, db_path);
                    dock::open_tab(&mut self.dock, Tab::Repeater);
                }
            }
            AppCommand::CopyFuzzCurl { run_id, idx } => {
                let result = if self.state.fuzzer.selected_result.as_ref().is_some_and(|r| r.idx == idx && self.state.fuzzer.run_id == Some(run_id)) {
                    self.state.fuzzer.selected_result.clone()
                } else {
                    mitm_core::fuzzer::get_result_db(db_path, run_id, idx).ok().flatten()
                };
                if let Some(result) = result {
                    ctx.copy_text(crate::format::Exchange::from_fuzz_result(&result).curl());
                }
            }
            AppCommand::SaveFuzzRun { run_id, name } => {
                match self.core.save_fuzz_run(run_id, &name) {
                    Ok(()) => {
                        if self.state.fuzzer.run_id == Some(run_id) {
                            self.state.fuzzer.is_saved = true;
                            self.state.fuzzer.run_name = Some(name);
                        }
                        self.state.fuzzer.load_saved_runs(db_path);
                    }
                    Err(e) => {
                        self.state.fuzzer.error = Some(e);
                    }
                }
            }
            AppCommand::LoadFuzzRun(id) => {
                match self.core.load_fuzz_run(id) {
                    Ok(loaded) => {
                        self.state.fuzzer.load_run(loaded);
                    }
                    Err(e) => {
                        self.state.fuzzer.error = Some(e);
                    }
                }
            }
            AppCommand::DeleteFuzzRun(id) => {
                let _ = self.core.delete_fuzz_run(id);
                if self.state.fuzzer.run_id == Some(id) {
                    self.state.fuzzer.is_saved = false;
                    self.state.fuzzer.run_name = None;
                }
                self.state.fuzzer.load_saved_runs(db_path);
            }
            AppCommand::ReloadSavedFuzzRuns => {
                self.state.fuzzer.load_saved_runs(db_path);
            }

            AppCommand::ForwardIntercept { flow_id, url, method, headers, body } => {
                let _ = self.core.forward_intercepted_flow(&flow_id, url, method, headers, body);
                self.state.intercept.remove_pending(&flow_id);
            }
            AppCommand::DropIntercept { flow_id } => {
                let _ = self.core.drop_intercepted_flow(&flow_id);
                self.state.intercept.remove_pending(&flow_id);
            }
            AppCommand::ForwardActiveIntercept { modified } => {
                let intercept = &self.state.intercept;
                if let Some(flow_id) = intercept.active_id.clone() {
                    let edit = &intercept.edit;
                    let cmd = AppCommand::ForwardIntercept {
                        flow_id,
                        url: modified.then(|| edit.url.clone()),
                        method: modified.then(|| edit.method.clone()),
                        headers: modified.then(|| intercept.edited_headers()),
                        body: modified.then(|| edit.body.as_bytes().to_vec()),
                    };
                    self.run_command(ctx, cmd);
                }
            }
            AppCommand::DropActiveIntercept => {
                if let Some(flow_id) = self.state.intercept.active_id.clone() {
                    self.run_command(ctx, AppCommand::DropIntercept { flow_id });
                }
            }
            AppCommand::ForwardAllIntercepts => {
                self.core.forward_all_intercepted();
                self.state.intercept.clear_pending();
            }
            AppCommand::DropAllIntercepts => {
                self.core.drop_all_intercepted();
                self.state.intercept.clear_pending();
            }
            AppCommand::ConfigureInterceptor { enabled, mode } => {
                let core = Arc::clone(&self.core);
                self.rt.spawn(async move {
                    let _ = core.toggle_interceptor(enabled, mode).await;
                });
            }
            AppCommand::SetRewriteEnabled(enabled) => {
                let _ = self.core.set_rewrite_enabled(enabled);
            }
            AppCommand::ShowRewriteFor(flow_id) => {
                if let Some(flow) = self.state.traffic.flow(flow_id).cloned() {
                    let rewrite = &mut self.state.rewrite;
                    rewrite.show_log_entry_for(&flow, db_path);
                    dock::open_tab(&mut self.dock, Tab::Rewrite);
                }
            }
            AppCommand::ClearRewriteHistory => {
                if self.core.clear_rewrite_history().is_ok() {
                    self.state.rewrite.clear_log();
                }
            }
            AppCommand::SetRewriteScope(scope) => {
                let _ = self.rt.block_on(self.core.set_rewrite_source_scope(scope));
            }
            AppCommand::SetInterceptScope(scope) => {
                let core = Arc::clone(&self.core);
                self.rt.spawn(async move {
                    let _ = core.set_intercept_source_scope(scope).await;
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
            AppCommand::SelectWsConnection(id) => self.state.websockets.select(id, db_path),
            AppCommand::ConnectWs { url, headers } => {
                let core = Arc::clone(&self.core);
                let headers = (!headers.is_empty()).then_some(headers);
                self.ws_connects.spawn(&self.rt, ctx, async move { core.connect_ws_client(url, headers).await });
            }
            AppCommand::SendWsFrame { connection_id, msg_type, payload, to_server } => {
                let core = Arc::clone(&self.core);
                self.ws_actions.spawn(&self.rt, ctx, async move {
                    core.send_ws_frame(connection_id, msg_type, payload, to_server).await
                });
            }
            AppCommand::DisconnectWs(id) => {
                let core = Arc::clone(&self.core);
                self.ws_actions.spawn(&self.rt, ctx, async move { core.disconnect_ws(id).await });
            }
            AppCommand::ClearWsSession(id) => match self.core.clear_ws_session(id) {
                Ok(()) => self.state.websockets.cleared(id),
                Err(e) => self.ws_actions.reply(ctx).send(Err(e)),
            },
            AppCommand::DeleteWsSession(id) => match self.core.delete_ws_connection(id) {
                Ok(()) => self.state.websockets.removed(id),
                Err(e) => self.ws_actions.reply(ctx).send(Err(e)),
            },
            AppCommand::SetWsMitm(enabled) => {
                if let Err(e) = self.rt.block_on(self.core.set_ws_mitm_enabled(enabled)) {
                    self.state.websockets.mitm_enabled = !enabled;
                    self.ws_actions.reply(ctx).send(Err(e));
                }
            }
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
            AppCommand::OpenImport => self.state.environments.import.open_with(None, String::new()),
            AppCommand::PickImportFile => {
                self.import_files.spawn(&self.rt, ctx, async {
                    let path = crate::files::pick_open(&[("JSON", &["json"])]).await?;
                    let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                    Some(std::fs::read_to_string(&path).map(|json| (name, json)).map_err(|e| e.to_string()))
                });
            }
            AppCommand::ExportWorkspace => {
                let Some(workspace) = self.state.environments.active_workspace().cloned() else { return };
                let db_path = db_path.clone();
                self.file_saves.spawn(&self.rt, ctx, async move {
                    let path = crate::files::pick_save(&format!("{}.json", workspace.name), &[("JSON", &["json"])]).await?;
                    let result = mitm_core::workspace::export::export_workspace_file_db(&db_path, &workspace.id, &path.to_string_lossy());
                    Some(result.map(|_| format!("Exported {}", crate::files::file_name(&path))))
                });
            }
            AppCommand::ImportWorkspace { json, target, name } => {
                match mitm_core::workspace::import::import_workspace_json_db(db_path, &json, target.as_deref(), name.as_deref()) {
                    Ok(_) if target.is_some() => {
                        self.state.environments.import = Default::default();
                        self.state.environments.refresh(db_path);
                        self.state.collections.reload_tree(db_path);
                    }
                    Ok(summary) => {
                        self.state.environments.import = Default::default();
                        self.state.environments.load(db_path);
                        self.run_command(ctx, AppCommand::SelectWorkspace(summary.workspace_id));
                    }
                    Err(e) => self.state.environments.import.error = Some(e),
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

    fn save_rewrite_rules(&mut self, now: f64, force: bool) {
        if let Some(rules) = self.state.rewrite.take_save(now, force) {
            let _ = self.rt.block_on(self.core.save_rewrite_rules(rules));
        }
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
        self.save_rewrite_rules(0.0, true);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::widgets::template::set_active_env(self.state.environments.active_environment());
        while let Ok(event) = self.core_events.try_recv() {
            self.apply_core_event(ui.ctx(), event);
        }
        let focused_tab = self.dock.find_active_focused().map(|(_, tab)| *tab);
        shortcuts::consume(ui.ctx(), focused_tab, &mut self.cmds);
        let now = ui.input(|i| i.time);
        for (id, result) in self.repeater_runs.drain() {
            self.state.repeater.finish_run(&id, result, &self.core.db_path);
            self.state.environments.refresh(&self.core.db_path);
        }
        self.state.repeater.flush_if_idle(&self.core.db_path, now);
        for (id, result) in self.collection_runs.drain() {
            self.state.collections.finish_run(&id, result, &self.core.db_path);
            self.state.environments.refresh(&self.core.db_path);
        }
        self.state.collections.flush_if_idle(&self.core.db_path, now);
        self.state.environments.flush_if_idle(&self.core.db_path, now);
        self.save_rewrite_rules(now, false);
        self.state.traffic.sync_header_search(&self.core.db_path);
        for (run_id, query, matches) in self.fuzz_search.drain() {
            self.state.fuzzer.set_response_search_results(run_id, &query, matches);
        }
        self.state.fuzzer.sync_response_search(&self.rt, ui.ctx(), &self.core, &self.fuzz_search);
        for result in self.import_files.drain().flatten() {
            let import = &mut self.state.environments.import;
            match result {
                Ok((name, json)) => import.open_with(Some(environments::file_label(&name, json.len())), json),
                Err(e) => import.error = Some(e),
            }
        }
        if let Some((name, json)) = dropped_json(ui.ctx()) {
            self.state.environments.import.open_with(Some(environments::file_label(&name, json.len())), json);
        }
        for result in self.file_saves.drain().flatten() {
            top_bar::flash(ui, result);
        }
        for result in self.ws_connects.drain() {
            match result {
                Ok(conn) => self.state.websockets.connected(conn, &self.core.db_path),
                Err(e) => top_bar::flash(ui, Err(e)),
            }
        }
        for error in self.ws_actions.drain().filter_map(Result::err) {
            top_bar::flash(ui, Err(error));
        }
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
        crate::features::repeater::show_curl_import(ui.ctx(), &mut self.state.repeater.curl_import, &mut self.cmds);
        crate::features::inspector::show_window(ui.ctx(), &mut self.state.inspector, &self.state.traffic, &mut self.cmds);
        let now = ui.input(|i| i.time);
        environments::show_quick_vars(ui.ctx(), &mut self.state.environments, now);
        let workspace = self.state.environments.active_workspace().map(|w| (w.id.clone(), w.name.clone())).unwrap_or_default();
        environments::show_import(
            ui.ctx(),
            &mut self.state.environments.import,
            (&workspace.0, &workspace.1),
            &self.core.db_path,
            &mut self.cmds,
        );

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

fn dropped_json(ctx: &egui::Context) -> Option<(String, String)> {
    let file = ctx.input(|i| i.raw.dropped_files.first().cloned())?;
    let name = file.path().file_name()?.to_string_lossy().into_owned();
    if !environments::is_json_file(&name) {
        return None;
    }
    let bytes = file.bytes().ok()?;
    Some((name, String::from_utf8_lossy(&bytes).into_owned()))
}

fn startup_state(rt: &Runtime, core: &MitmCore) -> AppState {
    let mut state = AppState::load(&core.db_path);
    state.traffic.set_history_limit(rt.block_on(core.get_history_settings()));
    state.proxy.mode = rt.block_on(core.proxy_mode());
    if state.proxy.mode.is_enabled() {
        state.proxy.set_started(rt.block_on(core.start_all_listeners()));
    }
    refresh_listener_state(rt, core, &mut state.proxy);
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
    state
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
