pub mod command;
pub mod cx;
pub mod jobs;
pub mod prefs;
pub mod state;
mod poll;
mod startup;
mod workspace;

use std::sync::Arc;
use crossbeam_channel::Receiver;
use eframe::egui;
use egui_dock::{DockArea, DockState};
use mitm_core::events::CoreEvent;
use mitm_core::MitmCore;
use tokio::runtime::Runtime;
use command::{AppCommand, Commands};
use startup::{forward_core_events, startup_state};
use cx::Cx;
use crate::ui::dock::{self, Tab, TabViewer};
use mitm_core::collections::execute::ExecutionResult;
use mitm_core::http::RunError;
use mitm_core::repeater::RepeaterExecutionResult;
use crate::features::collections::CollectionsCmd;
use crate::features::request_editor::captured::CapturedRequest;
use crate::features::environments::{self, QuickVarScope};
use jobs::Jobs;
use state::AppState;
use crate::ui::palette::{self, PaletteState};
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
    repeater_runs: Jobs<(String, Result<RepeaterExecutionResult, RunError>)>,
    collection_runs: Jobs<(String, Result<ExecutionResult, RunError>)>,
    import_files: Jobs<Option<Result<(String, String), String>>>,
    credential_files: Jobs<Option<Result<String, String>>>,
    qr_images: Jobs<Option<(crate::app::command::QrTarget, Result<String, String>)>>,
    file_saves: Jobs<Option<Result<String, String>>>,
    factory_resets: Jobs<Result<Option<MitmCore>, String>>,
}

impl MitmApp {
    pub fn new(ctx: &egui::Context, storage: Option<&dyn eframe::Storage>, rt: Arc<Runtime>, core: Arc<MitmCore>) -> Self {
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
            factory_resets: Jobs::default(),
        }
    }

    fn apply_core_event(&mut self, ctx: &egui::Context, event: CoreEvent) {
        let db = &self.core.db;
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
                traffic.refresh_detail_if_selected(id, db);
            }
            CoreEvent::TrafficUpdated { id, status, size, duration_ms } => {
                traffic.update_response(id, status, size, duration_ms);
                traffic.refresh_detail_if_selected(id, db);
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
        let db = &self.core.db;
        // These reload environments from the DB, and a later flush of a stale copy would undo the switch.
        if cmd.touches_environments() {
            self.state.environments.flush(db);
            self.state.credentials.flush(db);
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
            AppCommand::DetachInspector(detach) => dock::detach_inspector(&mut self.dock, detach),
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

            AppCommand::Traffic(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::traffic::handle(&mut self.state.traffic, &mut self.state.inspector, &cx, cmd);
            }
            AppCommand::SaveFlowToCollection { flow_id, target } => {
                if let Some(flow) = self.state.traffic.flow(flow_id) {
                    let detail = mitm_core::db::get_history_detail(db, flow_id).ok().flatten();
                    let captured = CapturedRequest::from_flow(flow, detail.as_ref());
                    let name = format!("{} {}", flow.method, if flow.path.is_empty() { &flow.url } else { &flow.path });
                    let saved = self.state.collections.save_captured(db, target, name, captured);
                    crate::widgets::toast::push(ctx, saved.map(|_| "Saved to collection".to_string()));
                }
            }
            AppCommand::SaveBody(body) => {
                self.file_saves.spawn(&self.rt, ctx, async move {
                    let path = crate::util::files::pick_save(&body.file_name, &[]).await?;
                    let result = std::fs::write(&path, &body.bytes).map_err(|e| e.to_string());
                    Some(result.map(|_| format!("Saved {}", crate::util::files::file_name(&path))))
                });
            }
            AppCommand::AddInterceptRuleForHost(host) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::intercept::handle(&mut self.state.intercept, &cx, crate::features::intercept::Cmd::AddRuleForHost(host));
                dock::open_tab(&mut self.dock, Tab::Intercept);
            }
            AppCommand::SendFlowToRepeater(id) => {
                if let Some(flow) = self.state.traffic.flow(id).cloned() {
                    let detail = mitm_core::db::get_history_detail(db, id).ok().flatten();
                    self.state.repeater.create_from_flow(&flow, detail.as_ref(), db);
                    dock::open_tab(&mut self.dock, Tab::Repeater);
                }
            }
            AppCommand::SendFlowToFuzzer(id) => {
                if let Some(flow) = self.state.traffic.flow(id).cloned() {
                    let detail = mitm_core::db::get_history_detail(db, id).ok().flatten();
                    let captured = CapturedRequest::from_flow(&flow, detail.as_ref());
                    self.state.fuzzer.load_capture(captured);
                    dock::open_tab(&mut self.dock, Tab::Fuzzer);
                }
            }

            AppCommand::OpenCurlImport => self.state.repeater.curl_import.open(),
            AppCommand::RepeaterImportCurl(curl) => {
                self.state.repeater.create_from_curl(curl, db);
                dock::open_tab(&mut self.dock, Tab::Repeater);
            }
            AppCommand::Repeater(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::repeater::handle(&mut self.state.repeater, &cx, cmd);
            }
            AppCommand::Credentials(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::credentials::handle(&mut self.state.credentials, &cx, cmd);
            }
            AppCommand::RepeaterSend(id) => {
                self.state.credentials.touch(ctx.input(|i| i.time));
                let repeater = &mut self.state.repeater;
                repeater.flush(db);
                if let Some(tab) = repeater.tabs.iter().find(|t| t.id == id).cloned() {
                    repeater.executing.insert(id);
                    let db = db.clone();
                    self.repeater_runs.spawn(&self.rt, ctx, async move {
                        let result = mitm_core::repeater::execute::execute_repeater_tab(&db, &tab).await;
                        (tab.id, result)
                    });
                }
            }
            AppCommand::Fuzzer(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::fuzzer::handle(&mut self.state.fuzzer, &cx, cmd);
            }
            AppCommand::SendFuzzResultToRepeater { run_id, idx } => {
                if let Some(result) = self.state.fuzzer.result(run_id, idx, db) {
                    self.state.repeater.create_from_fuzz_result(&result, db);
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
                    rewrite.show_log_entry_for(&flow, db);
                    dock::open_tab(&mut self.dock, Tab::Rewrite);
                }
            }

            AppCommand::Collections(CollectionsCmd::Execute(id)) => {
                self.state.credentials.touch(ctx.input(|i| i.time));
                if self.state.collections.start_run(&id, db) {
                    let db = db.clone();
                    self.collection_runs.spawn(&self.rt, ctx, async move {
                        let result = mitm_core::collections::execute::execute_collection_request_db(&db, &id).await;
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
            AppCommand::SelectWorkspace(id) => self.select_workspace(ctx, id),
            AppCommand::Environments(cmd) => {
                let cx = Cx { rt: &self.rt, core: &self.core, ctx };
                crate::features::environments::handle(&mut self.state.environments, &cx, cmd);
            }
            AppCommand::CreateWorkspace(name) => self.create_workspace(ctx, name),
            AppCommand::DeleteWorkspace(id) => self.delete_workspace(ctx, id),
            AppCommand::OpenImport => self.state.environments.import.open_with(None, String::new()),
            AppCommand::PickQrImage(target) => self.pick_qr_image(ctx, target),
            AppCommand::PickCredentialsJson => self.pick_credentials_json(ctx),
            AppCommand::PickImportFile => self.pick_import_file(ctx),
            AppCommand::ExportWorkspace => self.export_workspace(ctx),
            AppCommand::ExportWorkspaceFile { options, passphrase } => self.export_workspace_file(ctx, options, passphrase),
            AppCommand::ImportWorkspace { json, target, name, passphrase } => self.import_workspace(ctx, json, target, name, passphrase),
            AppCommand::MoveToCredentials { env_id, var_idx, entry, field } => self.move_to_credentials(ctx, env_id, var_idx, entry, field),
        }
    }

    fn save_rewrite_rules_now(&mut self) {
        if let Some(rules) = self.state.rewrite.take_save(0.0, true) {
            // The window is closing, so there's nowhere left to show a failure.
            let _ = self.rt.block_on(self.core.save_rewrite_rules(rules));
        }
    }

}

impl eframe::App for MitmApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        prefs::persist(storage, &self.dock, &self.state);
    }

    fn on_exit(&mut self) {
        self.state.repeater.flush(&self.core.db);
        self.state.collections.flush(&self.core.db);
        self.state.environments.flush(&self.core.db);
        self.state.credentials.flush(&self.core.db);
        self.save_rewrite_rules_now();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::widgets::template::set_active_env(self.state.environments.active_environment());
        self.state.credentials.sync(&self.core.db, &self.state.environments.active_workspace_id);
        let now = ui.input(|i| i.time);
        self.state.credentials.tick(ui.ctx(), &self.core.db, now);
        while let Ok(event) = self.core_events.try_recv() {
            self.apply_core_event(ui.ctx(), event);
        }
        let focused_tab = self.dock.find_active_focused().map(|(_, tab)| *tab);
        shortcuts::consume(ui.ctx(), focused_tab, &mut self.cmds);
        self.poll_jobs(ui, now);
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

        self.state.inspector.prefs.popped_out = self.dock.find_tab(&Tab::Inspector).is_some();
        if let Some((path, tab)) = dock::tab_bar_menus(ui, &self.dock) {
            dock::open_in_leaf(&mut self.dock, path, tab);
        }
        let mut viewer = TabViewer {
            state: &mut self.state,
            cmds: &mut self.cmds,
            db: &self.core.db,
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
        let now = ui.input(|i| i.time);
        environments::show_quick_vars(ui.ctx(), &mut self.state.environments, now);
        let workspace = self.state.environments.active_workspace().map(|w| (w.id.clone(), w.name.clone())).unwrap_or_default();
        environments::show_export(ui.ctx(), &mut self.state.environments.export, &mut self.cmds);
        crate::features::traffic::show_har_export(ui.ctx(), &mut self.state.traffic.har_export, &mut self.cmds);
        environments::show_import(
            ui.ctx(),
            &mut self.state.environments.import,
            (&workspace.0, &workspace.1),
            &self.core.db,
            &mut self.cmds,
        );
        crate::features::credentials::show_json_import(ui.ctx(), &mut self.state.credentials, &self.core.db, &mut self.cmds);
        // Last, so it sits above the dialog whose action asked for it.
        if let Some(retry) = crate::features::credentials::show_unlock(ui.ctx(), &mut self.state.credentials, &self.core.db) {
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




