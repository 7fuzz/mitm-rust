pub mod command;
pub mod cx;
pub mod jobs;
pub mod prefs;
pub mod state;

use std::path::PathBuf;
use std::sync::Arc;
use crossbeam_channel::Receiver;
use eframe::egui;
use egui_dock::{DockArea, DockState};
use mitm_core::events::CoreEvent;
use mitm_core::MitmCore;
use tokio::runtime::Runtime;
use tokio::sync::broadcast::error::RecvError;
use command::{AppCommand, Commands};
use cx::Cx;
use crate::ui::dock::{self, Tab, TabViewer};
use mitm_core::collections::execute::ExecutionResult;
use mitm_core::repeater::RepeaterExecutionResult;
use crate::features::collections::CollectionsCmd;
use crate::features::request_editor::captured::CapturedRequest;
use crate::features::environments::{self, QuickVarScope};
use crate::util::format::Exchange;
use jobs::Jobs;
use state::AppState;
use crate::ui::palette::{self, PaletteState};
use crate::ui::theme::Tone;
use crate::ui::{shortcuts, top_bar};

pub struct MitmApp {
    dock: DockState<Tab>,
    state: AppState,
    cmds: Commands,
    palette: PaletteState,
    focus_search: Option<Tab>,
    core: Arc<MitmCore>,
    rt: Arc<Runtime>,
    core_events: Receiver<CoreEvent>,
    repeater_runs: Jobs<(String, Result<RepeaterExecutionResult, String>)>,
    collection_runs: Jobs<(String, Result<ExecutionResult, String>)>,
    import_files: Jobs<Option<Result<(String, String), String>>>,
    credential_files: Jobs<Option<Result<String, String>>>,
    qr_images: Jobs<Option<(crate::app::command::QrTarget, Result<String, String>)>>,
    file_saves: Jobs<Option<Result<String, String>>>,
    har_index: Jobs<Result<Vec<mitm_core::HistoryEntry>, String>>,
    factory_resets: Jobs<Result<Option<MitmCore>, String>>,
    history_limits: Jobs<(mitm_core::state::HistorySettings, Result<(), String>)>,
}

impl MitmApp {
    pub fn new(ctx: &egui::Context, storage: Option<&dyn eframe::Storage>, rt: Arc<Runtime>, core: Arc<MitmCore>) -> Self {
        crate::ui::theme::install_fonts(ctx);
        crate::ui::theme::apply_style(ctx);
        let mut state = startup_state(&rt, &core);
        Self {
            dock: prefs::restore(storage, &mut state),
            state,
            cmds: Commands::default(),
            palette: PaletteState::default(),
            focus_search: None,
            core_events: forward_core_events(&rt, &core, ctx),
            core,
            rt,
            repeater_runs: Jobs::default(),
            collection_runs: Jobs::default(),
            import_files: Jobs::default(),
            credential_files: Jobs::default(),
            qr_images: Jobs::default(),
            file_saves: Jobs::default(),
            har_index: Jobs::default(),
            factory_resets: Jobs::default(),
            history_limits: Jobs::default(),
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
            CoreEvent::WebhookDelivery(event) => self.state.webhooks.apply_delivery(event),
            _ => {}
        }
    }

    fn run_command(&mut self, ctx: &egui::Context, cmd: AppCommand) {
        let db_path = &self.core.db_path;
        // These reload environments from the DB, and a later flush of a stale copy would undo the switch.
        if cmd.touches_environments() {
            self.state.environments.flush(db_path);
            self.state.credentials.flush(db_path);
            self.state.credentials.mark_stale();
        }
        match cmd {
            AppCommand::OpenPalette => self.palette.open(),
            AppCommand::FocusSearch => self.focus_search = self.dock.find_active_focused().map(|(_, tab)| *tab),
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
            AppCommand::OpenSettings(section) => {
                self.state.settings.section = section;
                dock::open_tab(&mut self.dock, Tab::Settings);
            }
            AppCommand::FocusTabAt(index) => dock::focus_tab_at(&mut self.dock, index),
            AppCommand::CycleTab { forward } => dock::cycle_tab(&mut self.dock, forward),
            AppCommand::Settings(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::settings::handle(self.state.settings(), &cx, cmd);
            }
            AppCommand::FactoryReset if self.state.data.resetting => {}
            AppCommand::FactoryReset => {
                self.state.data.resetting = true;
                let core = Arc::clone(&self.core);
                self.factory_resets.spawn(&self.rt, ctx, async move {
                    core.factory_reset().await?;
                    let fresh = tokio::task::spawn_blocking(|| MitmCore::init_with_events(mitm_core::EventBus::default()).ok());
                    Ok(fresh.await.ok().flatten())
                });
            }

            AppCommand::LoadOlderFlows => self.state.traffic.load_older(db_path),
            AppCommand::SaveFlowToCollection { flow_id, target } => {
                if let Some(flow) = self.state.traffic.flow(flow_id) {
                    let detail = mitm_core::db::get_history_detail(db_path, flow_id).ok().flatten();
                    let captured = CapturedRequest::from_flow(flow, detail.as_ref());
                    let name = format!("{} {}", flow.method, if flow.path.is_empty() { &flow.url } else { &flow.path });
                    let saved = self.state.collections.save_captured(db_path, target, name, captured);
                    crate::widgets::toast::push(ctx, saved.map(|_| "Saved to collection".to_string()));
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
            AppCommand::OpenHarExport => {
                self.state.traffic.har_export.open();
                let db_path = db_path.clone();
                self.har_index.spawn(&self.rt, ctx, async move { mitm_core::db::get_history_summaries(&db_path) });
            }
            AppCommand::ExportHar(ids) => {
                let db_path = db_path.clone();
                self.file_saves.spawn(&self.rt, ctx, async move {
                    let path = crate::util::files::pick_save("traffic.har", &[("HAR", &["har"])]).await?;
                    Some(write_har(&db_path, &ids, &path).map(|count| {
                        format!("Exported {} requests to {}", count, crate::util::files::file_name(&path))
                    }))
                });
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
                    let path = crate::util::files::pick_save(&body.file_name, &[]).await?;
                    let result = std::fs::write(&path, &body.bytes).map_err(|e| e.to_string());
                    Some(result.map(|_| format!("Saved {}", crate::util::files::file_name(&path))))
                });
            }
            AppCommand::SetHistoryLimit(settings) => {
                let core = Arc::clone(&self.core);
                self.history_limits.spawn(&self.rt, ctx, async move {
                    let result = core.set_history_settings(settings.clone()).await;
                    (settings, result)
                });
            }
            AppCommand::AddInterceptRuleForHost(host) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::intercept::handle(&mut self.state.intercept, &cx, crate::features::intercept::Cmd::AddRuleForHost(host));
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

            AppCommand::OpenCurlImport => self.state.repeater.curl_import.open(),
            AppCommand::RepeaterImportCurl(curl) => {
                self.state.repeater.create_from_curl(curl, db_path);
                dock::open_tab(&mut self.dock, Tab::Repeater);
            }
            AppCommand::Repeater(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::repeater::handle(&mut self.state.repeater, &cx, cmd);
            }
            AppCommand::LockCredentials => self.state.credentials.lock(db_path),
            AppCommand::CopyTotp(id) => {
                if let Some(code) = self.state.credentials.current_code(&id) {
                    crate::util::clipboard::copy_secret(ctx, &code);
                }
            }
            AppCommand::RepeaterSend(id) => {
                self.state.credentials.touch(ctx.input(|i| i.time));
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
            AppCommand::Fuzzer(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::fuzzer::handle(&mut self.state.fuzzer, &cx, cmd);
            }
            AppCommand::SendFuzzResultToRepeater { run_id, idx } => {
                if let Some(result) = self.state.fuzzer.result(run_id, idx, db_path) {
                    self.state.repeater.create_from_fuzz_result(&result, db_path);
                    dock::open_tab(&mut self.dock, Tab::Repeater);
                }
            }

            AppCommand::Intercept(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::intercept::handle(&mut self.state.intercept, &cx, cmd);
            }
            AppCommand::Rewrite(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::rewrite::handle(&mut self.state.rewrite, &cx, cmd);
            }
            AppCommand::ShowRewriteFor(flow_id) => {
                if let Some(flow) = self.state.traffic.flow(flow_id).cloned() {
                    let rewrite = &mut self.state.rewrite;
                    rewrite.show_log_entry_for(&flow, db_path);
                    dock::open_tab(&mut self.dock, Tab::Rewrite);
                }
            }

            AppCommand::Collections(CollectionsCmd::Execute(id)) => {
                self.state.credentials.touch(ctx.input(|i| i.time));
                if self.state.collections.start_run(&id, db_path) {
                    let db_path = db_path.clone();
                    self.collection_runs.spawn(&self.rt, ctx, async move {
                        let result = mitm_core::collections::execute::execute_collection_request_db(&db_path, &id).await;
                        (id, result)
                    });
                }
            }
            AppCommand::Collections(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                self.state.collections.apply(cmd, &cx);
            }
            AppCommand::WebSockets(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::websockets::handle(&mut self.state.websockets, &cx, cmd);
            }
            AppCommand::Webhooks(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::webhooks::handle(&mut self.state.webhooks, &cx, cmd);
            }
            AppCommand::DecodeInEncodingTool(text) => {
                self.state.encoding.load_for_decode(text);
                dock::open_tab(&mut self.dock, Tab::Encoding);
            }
            AppCommand::RecomputeHmac { secret, payload } => {
                self.state.hmac_tool.prefill(secret, payload);
                dock::open_tab(&mut self.dock, Tab::HmacTool);
            }
            AppCommand::Hashing(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::utilities::hashing::handle(&mut self.state.hashing, &cx, cmd);
            }
            AppCommand::SelectWorkspace(id) => {
                let _ = mitm_core::workspace::set_active_workspace_db(db_path, &id);
                self.state.environments.active_workspace_id = id.clone();
                self.state.environments.reload_environments(db_path);
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                self.state.collections.apply(CollectionsCmd::SelectWorkspace(id), &cx);
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
            AppCommand::PickQrImage(target) => {
                self.qr_images.spawn(&self.rt, ctx, async move {
                    let path = crate::util::files::pick_open(&[("Image", &["png", "jpg", "jpeg", "gif", "webp"])]).await?;
                    let decoded = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|b| crate::util::qr::decode_file(&b));
                    Some((target, decoded))
                });
            }
            AppCommand::PickCredentialsJson => {
                self.credential_files.spawn(&self.rt, ctx, async {
                    let path = crate::util::files::pick_open(&[("JSON", &["json"])]).await?;
                    Some(std::fs::read_to_string(&path).map_err(|e| e.to_string()))
                });
            }
            AppCommand::PickImportFile => {
                self.import_files.spawn(&self.rt, ctx, async {
                    let path = crate::util::files::pick_open(&[("JSON", &["json"])]).await?;
                    let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                    Some(std::fs::read_to_string(&path).map(|json| (name, json)).map_err(|e| e.to_string()))
                });
            }
            AppCommand::ExportWorkspace => {
                let environments = &mut self.state.environments;
                if let Some(options) = environments.export.ask(&environments.environments, &self.state.credentials.entries) {
                    self.run_command(ctx, AppCommand::ExportWorkspaceFile { options, passphrase: None });
                }
            }
            AppCommand::ExportWorkspaceFile { options, passphrase } => {
                if passphrase.is_some() && mitm_core::credentials::is_locked(db_path) {
                    self.state.credentials.ask_unlock(Some(AppCommand::ExportWorkspaceFile { options, passphrase }));
                    return;
                }
                let Some(workspace) = self.state.environments.active_workspace().cloned() else { return };
                // Export reads the database, so edits still waiting for their debounced save go first.
                self.state.environments.flush(db_path);
                self.state.collections.flush(db_path);
                self.state.credentials.flush(db_path);
                let db_path = db_path.clone();
                self.file_saves.spawn(&self.rt, ctx, async move {
                    let name = format!("{}.mitm.json", workspace.name);
                    let path = crate::util::files::pick_save(&name, &[("Workspace file", &["json"])]).await?;
                    let result = mitm_core::workspace::export::export_workspace_file_db(
                        &db_path,
                        &workspace.id,
                        &path.to_string_lossy(),
                        options,
                        passphrase.as_deref(),
                    );
                    Some(result.map(|_| format!("Exported {}", crate::util::files::file_name(&path))))
                });
            }
            AppCommand::ImportWorkspace { json, target, name, passphrase } => {
                let imported = mitm_core::workspace::import::import_workspace_json_db(
                    db_path,
                    &json,
                    target.as_deref(),
                    name.as_deref(),
                    passphrase.as_deref(),
                );
                match imported {
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
                    Err(e) if e == mitm_core::credentials::LOCKED => {
                        let retry = AppCommand::ImportWorkspace { json, target, name, passphrase };
                        self.state.credentials.ask_unlock(Some(retry));
                    }
                    Err(e) => self.state.environments.import.error = Some(e),
                }
            }
            AppCommand::MoveToCredentials { env_id, var_idx, entry, field } => {
                let workspace_id = self.state.environments.active_workspace_id.clone();
                let environments = &mut self.state.environments;
                let Some(env) = environments.environments.iter_mut().find(|e| e.id == env_id) else { return };
                let Some(var) = env.variables.get_mut(var_idx) else { return };
                match crate::features::credentials::move_value(db_path, &workspace_id, &entry, &field, var.current_value().to_string()) {
                    Ok(link) => {
                        if var.variants.is_empty() {
                            var.value = link;
                        } else {
                            let idx = var.active_index.min(var.variants.len() - 1);
                            var.variants[idx].value = link.clone();
                            var.value = link;
                        }
                        environments.move_modal = None;
                        environments.mark_dirty(&env_id, ctx.input(|i| i.time));
                        environments.flush(db_path);
                        self.state.credentials.mark_stale();
                    }
                    Err(e) => {
                        if let Some(modal) = environments.move_modal.as_mut() {
                            modal.error = Some(e);
                        }
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

    fn finish_factory_reset(&mut self, ctx: &egui::Context, result: Result<Option<MitmCore>, String>) {
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

    fn save_rewrite_rules_now(&mut self) {
        if let Some(rules) = self.state.rewrite.take_save(0.0, true) {
            let _ = self.rt.block_on(self.core.save_rewrite_rules(rules));
        }
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
        self.state.credentials.flush(&self.core.db_path);
        self.save_rewrite_rules_now();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::widgets::template::set_active_env(self.state.environments.active_environment());
        self.state.credentials.sync(&self.core.db_path, &self.state.environments.active_workspace_id);
        let now = ui.input(|i| i.time);
        self.state.credentials.tick(ui.ctx(), &self.core.db_path, now);
        while let Ok(event) = self.core_events.try_recv() {
            self.apply_core_event(ui.ctx(), event);
        }
        let focused_tab = self.dock.find_active_focused().map(|(_, tab)| *tab);
        shortcuts::consume(ui.ctx(), focused_tab, &mut self.cmds);
        for (id, result) in self.repeater_runs.drain() {
            if is_locked_error(&result) {
                self.state.repeater.executing.remove(&id);
                self.state.credentials.ask_unlock(Some(AppCommand::RepeaterSend(id)));
                continue;
            }
            self.state.repeater.finish_run(&id, result, &self.core.db_path);
            self.state.environments.refresh(&self.core.db_path);
            self.state.credentials.mark_stale();
        }
        self.state.repeater.flush_if_idle(&self.core.db_path, now);
        for (id, result) in self.collection_runs.drain() {
            if is_locked_error(&result) {
                self.state.collections.finish_run(&id, result, &self.core.db_path);
                self.state.credentials.ask_unlock(Some(AppCommand::Collections(CollectionsCmd::Execute(id))));
                continue;
            }
            self.state.collections.finish_run(&id, result, &self.core.db_path);
            self.state.environments.refresh(&self.core.db_path);
            self.state.credentials.mark_stale();
        }
        self.state.collections.flush_if_idle(&self.core.db_path, now);
        self.state.environments.flush_if_idle(&self.core.db_path, now);
        self.state.credentials.flush_if_idle(&self.core.db_path, now);
        let save_errors = [
            self.state.repeater.take_save_errors(),
            self.state.collections.take_save_errors(),
            self.state.environments.take_save_errors(),
            self.state.credentials.take_save_errors(),
        ];
        for error in save_errors.into_iter().flatten() {
            top_bar::flash(ui, Err(error));
        }
        self.state.traffic.sync_header_search(&self.core.db_path);
        for (target, decoded) in self.qr_images.drain().flatten() {
            use crate::app::command::QrTarget;
            match (target, decoded) {
                (QrTarget::TotpTool, Ok(text)) => {
                    self.state.totp.input = text;
                    self.state.totp.qr_error = None;
                }
                (QrTarget::TotpTool, Err(e)) => self.state.totp.qr_error = Some(e),
                (QrTarget::CredentialField { id, key }, Ok(text)) => self.state.credentials.set_totp_secret(&id, &key, text, now),
                (QrTarget::CredentialField { .. }, Err(e)) => self.state.credentials.show_error(e),
            }
        }
        for result in self.credential_files.drain().flatten() {
            let import = &mut self.state.credentials.json_import;
            match result {
                Ok(json) => import.open_with(json),
                Err(e) => import.failed(e),
            }
        }
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
        for result in self.har_index.drain() {
            self.state.traffic.har_export.loaded(result);
        }
        for result in self.file_saves.drain().flatten() {
            top_bar::flash(ui, result);
        }
        let resets: Vec<_> = self.factory_resets.drain().collect();
        for result in resets {
            self.finish_factory_reset(ui.ctx(), result);
        }
        for (settings, result) in self.history_limits.drain() {
            match result {
                Ok(()) => {
                    self.state.traffic.set_history_limit(settings);
                    self.state.traffic.load_from_db(&self.core.db_path);
                }
                Err(e) => top_bar::flash(ui, Err(e)),
            }
        }
        {
            let cx = Cx { rt: &self.rt, core: &self.core, ctx: ui.ctx() };
            crate::features::webhooks::poll(&mut self.state.webhooks, &cx);
            crate::features::websockets::poll(&mut self.state.websockets, &cx);
            crate::features::settings::poll(self.state.settings(), &cx);
            crate::features::rewrite::poll(&mut self.state.rewrite, &cx, now);
            crate::features::intercept::poll(&mut self.state.intercept, &cx);
            crate::features::fuzzer::poll(&mut self.state.fuzzer, &cx);
        }
        crate::features::utilities::hashing::poll(&mut self.state.hashing);
        let palette = crate::ui::theme::Palette::of(ui.visuals());
        egui::Panel::top("top_panel")
            .frame(egui::Frame::new().fill(palette.base).inner_margin(egui::Margin::symmetric(8, 5)))
            .show(ui, |ui| {
                top_bar::show(ui, &mut self.state, &self.dock, &mut self.cmds);
            });
        egui::Panel::bottom("status_bar")
            .exact_size(22.0)
            .frame(egui::Frame::new().fill(palette.base).inner_margin(egui::Margin::symmetric(8, 0)))
            .show(ui, |ui| crate::ui::status_bar::show(ui, &self.state));

        if let Some((path, tab)) = dock::tab_bar_menus(ui, &self.dock) {
            dock::open_in_leaf(&mut self.dock, path, tab);
        }
        let mut viewer = TabViewer {
            state: &mut self.state,
            cmds: &mut self.cmds,
            db_path: &self.core.db_path,
            focus_search: self.focus_search.take(),
            focused: focused_tab,
            visuals: ui.visuals().clone(),
        };
        DockArea::new(&mut self.dock).style(crate::ui::theme::dock_style(ui.style())).show_inside(ui, &mut viewer);
        if let Some(text) = crate::widgets::template::take_decode_request() {
            self.cmds.push(AppCommand::DecodeInEncodingTool(text));
        }
        palette::show(ui.ctx(), &mut self.palette, &self.state, &mut self.cmds);
        crate::features::repeater::show_curl_import(ui.ctx(), &mut self.state.repeater.curl_import, &mut self.cmds);
        crate::features::inspector::show_window(ui.ctx(), &mut self.state.inspector, &self.state.traffic, &mut self.cmds);
        let now = ui.input(|i| i.time);
        environments::show_quick_vars(ui.ctx(), &mut self.state.environments, now);
        let workspace = self.state.environments.active_workspace().map(|w| (w.id.clone(), w.name.clone())).unwrap_or_default();
        environments::show_export(ui.ctx(), &mut self.state.environments.export, &mut self.cmds);
        crate::features::traffic::show_har_export(ui.ctx(), &mut self.state.traffic.har_export, &mut self.cmds);
        environments::show_import(
            ui.ctx(),
            &mut self.state.environments.import,
            (&workspace.0, &workspace.1),
            &self.core.db_path,
            &mut self.cmds,
        );
        crate::features::credentials::show_json_import(ui.ctx(), &mut self.state.credentials, &self.core.db_path, &mut self.cmds);
        // Last, so it sits above the dialog whose action asked for it.
        if let Some(retry) = crate::features::credentials::show_unlock(ui.ctx(), &mut self.state.credentials, &self.core.db_path) {
            self.cmds.push(retry);
        }

        crate::widgets::toast::show(ui.ctx());

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

fn write_har(db_path: &PathBuf, ids: &[i64], path: &std::path::Path) -> Result<usize, String> {
    let flows = mitm_core::db::get_history_by_ids(db_path, ids)?;
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &mitm_core::http::har::build_har(&flows)).map_err(|e| e.to_string())?;
    std::io::Write::flush(&mut writer).map_err(|e| e.to_string())?;
    Ok(flows.len())
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

fn is_locked_error<T>(result: &Result<T, String>) -> bool {
    matches!(result, Err(e) if e == mitm_core::credentials::LOCKED)
}
