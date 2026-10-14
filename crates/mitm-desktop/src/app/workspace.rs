//! Workspace-level commands: they span environments, collections and credentials, so the app runs them.
use eframe::egui;
use mitm_core::credentials::CredentialError;
use mitm_core::workspace::file::ExportOptions;
use crate::features::collections::CollectionsCmd;
use super::command::{AppCommand, QrTarget};
use super::cx::Cx;
use super::MitmApp;

impl MitmApp {
    pub(super) fn select_workspace(&mut self, ctx: &egui::Context, id: String) {
        let db = &self.core.db;
        let cx = Cx { rt: &self.rt, core: &self.core, ctx };
        cx.report(mitm_core::workspace::set_active_workspace_db(db, &id));
        self.state.environments.active_workspace_id = id.clone();
        self.state.environments.reload_environments(db);
        self.state.collections.apply(CollectionsCmd::SelectWorkspace(id), &cx);
    }

    pub(super) fn create_workspace(&mut self, ctx: &egui::Context, name: String) {
        let db = &self.core.db;
        let cx = Cx { rt: &self.rt, core: &self.core, ctx };
        if let Some(ws) = cx.report(mitm_core::workspace::create_workspace_db(db, name, None)) {
            self.state.environments.workspaces.push(ws.clone());
            self.run_command(ctx, AppCommand::SelectWorkspace(ws.id));
        }
    }

    pub(super) fn delete_workspace(&mut self, ctx: &egui::Context, id: String) {
        let db = &self.core.db;
        let cx = Cx { rt: &self.rt, core: &self.core, ctx };
        if cx.report(mitm_core::workspace::delete_workspace_db(db, &id)).is_none() {
            return;
        }
        self.state.environments.workspaces.retain(|w| w.id != id);
        if self.state.environments.active_workspace_id == id {
            if let Some(first) = self.state.environments.workspaces.first().cloned() {
                self.run_command(ctx, AppCommand::SelectWorkspace(first.id));
            } else if let Ok(ws) = mitm_core::workspace::create_workspace_db(db, "Default Workspace".to_string(), None) {
                self.state.environments.workspaces.push(ws.clone());
                self.run_command(ctx, AppCommand::SelectWorkspace(ws.id));
            }
        }
    }

    pub(super) fn pick_qr_image(&mut self, ctx: &egui::Context, target: QrTarget) {
        self.qr_images.spawn(&self.rt, ctx, async move {
            let path = crate::util::files::pick_open(&[("Image", &["png", "jpg", "jpeg", "gif", "webp"])]).await?;
            let decoded = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|b| crate::util::qr::decode_file(&b));
            Some((target, decoded))
        });
    }

    pub(super) fn pick_markdown_file(&mut self, ctx: &egui::Context) {
        self.markdown_files.spawn(&self.rt, ctx, async {
            let path = crate::util::files::pick_open(&[("Markdown", &["md", "markdown", "txt"])]).await?;
            let name = crate::util::files::file_name(&path);
            Some(std::fs::read_to_string(&path).map(|text| (name, text)).map_err(|e| e.to_string()))
        });
    }

    pub(super) fn pick_credentials_json(&mut self, ctx: &egui::Context) {
        self.credential_files.spawn(&self.rt, ctx, async {
            let path = crate::util::files::pick_open(&[("JSON", &["json"])]).await?;
            Some(std::fs::read_to_string(&path).map_err(|e| e.to_string()))
        });
    }

    pub(super) fn pick_import_file(&mut self, ctx: &egui::Context) {
        self.import_files.spawn(&self.rt, ctx, async {
            let path = crate::util::files::pick_open(&[("JSON", &["json"])]).await?;
            let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            Some(std::fs::read_to_string(&path).map(|json| (name, json)).map_err(|e| e.to_string()))
        });
    }

    pub(super) fn export_workspace(&mut self, ctx: &egui::Context) {
        let environments = &mut self.state.environments;
        if let Some(options) = environments.export.ask(&environments.environments, &self.state.credentials.linked) {
            self.run_command(ctx, AppCommand::ExportWorkspaceFile { options, passphrase: None });
        }
    }

    pub(super) fn export_workspace_file(&mut self, ctx: &egui::Context, options: ExportOptions, passphrase: Option<String>) {
        let db = &self.core.db;
        if passphrase.is_some() && mitm_core::credentials::is_locked(db) {
            self.state.credentials.ask_unlock(Some(AppCommand::ExportWorkspaceFile { options, passphrase }));
            return;
        }
        let Some(workspace) = self.state.environments.active_workspace().cloned() else { return };
        // Export reads the database, so edits still waiting for their debounced save go first.
        self.state.environments.flush(db);
        self.state.collections.flush(db);
        self.state.credentials.flush(db);
        let db = db.clone();
        self.file_saves.spawn(&self.rt, ctx, async move {
            let name = format!("{}.mitm.json", workspace.name);
            let path = crate::util::files::pick_save(&name, &[("Workspace file", &["json"])]).await?;
            let result = mitm_core::workspace::export::export_workspace_file_db(
                &db,
                &workspace.id,
                &path.to_string_lossy(),
                options,
                passphrase.as_deref(),
            );
            Some(result.map(|_| format!("Exported {}", crate::util::files::file_name(&path))))
        });
    }

    pub(super) fn import_workspace(&mut self, ctx: &egui::Context, json: String, target: Option<String>, name: Option<String>, passphrase: Option<String>) {
        let db = &self.core.db;
        let imported = mitm_core::workspace::import::import_workspace_json_db(
            db,
            &json,
            target.as_deref(),
            name.as_deref(),
            passphrase.as_deref(),
        );
        match imported {
            Ok(_) if target.is_some() => {
                self.state.environments.import = Default::default();
                self.state.environments.refresh(db);
                self.state.collections.reload_tree(db);
            }
            Ok(summary) => {
                self.state.environments.import = Default::default();
                self.state.environments.load(db);
                self.run_command(ctx, AppCommand::SelectWorkspace(summary.workspace_id));
            }
            Err(CredentialError::Locked) => {
                let retry = AppCommand::ImportWorkspace { json, target, name, passphrase };
                self.state.credentials.ask_unlock(Some(retry));
            }
            Err(e) => self.state.environments.import.error = Some(e.to_string()),
        }
    }

    pub(super) fn move_to_credentials(&mut self, ctx: &egui::Context, env_id: String, var_idx: usize, vault: String, entry: String, field: String) {
        let db = &self.core.db;
        let workspace_id = self.state.environments.active_workspace_id.clone();
        let environments = &mut self.state.environments;
        let Some(env) = environments.environments.iter_mut().find(|e| e.id == env_id) else { return };
        let Some(var) = env.variables.get_mut(var_idx) else { return };
        match crate::features::credentials::move_value(db, &workspace_id, &vault, &entry, &field, var.current_value().to_string()) {
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
                environments.flush(db);
                self.state.credentials.mark_stale();
            }
            Err(e) => {
                if let Some(modal) = environments.move_modal.as_mut() {
                    modal.error = Some(e);
                }
            }
        }
    }
}
