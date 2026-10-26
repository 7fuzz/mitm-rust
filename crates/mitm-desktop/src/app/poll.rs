use eframe::egui;
use mitm_core::http::RunError;
use crate::features::collections::CollectionsCmd;
use crate::features::environments;
use crate::ui::top_bar;
use super::command::{AppCommand, QrTarget};
use super::cx::Cx;
use super::MitmApp;

impl MitmApp {
    /// Takes the results of background jobs and idle saves, once per frame.
    pub(super) fn poll_jobs(&mut self, ui: &egui::Ui, now: f64) {
        for (id, result) in self.repeater_runs.drain() {
            if matches!(result, Err(RunError::Locked)) {
                self.state.repeater.executing.remove(&id);
                self.state.credentials.ask_unlock(Some(AppCommand::RepeaterSend(id)));
                continue;
            }
            self.state.repeater.finish_run(&id, result.map_err(|e| e.to_string()), &self.core.db);
            self.state.environments.refresh(&self.core.db);
            self.state.credentials.mark_stale();
        }
        self.state.repeater.flush_if_idle(&self.core.db, now);
        for (id, result) in self.collection_runs.drain() {
            if matches!(result, Err(RunError::Locked)) {
                self.state.collections.finish_run(&id, result.map_err(|e| e.to_string()), &self.core.db);
                self.state.credentials.ask_unlock(Some(AppCommand::Collections(CollectionsCmd::Execute(id))));
                continue;
            }
            self.state.collections.finish_run(&id, result.map_err(|e| e.to_string()), &self.core.db);
            self.state.environments.refresh(&self.core.db);
            self.state.credentials.mark_stale();
        }
        self.state.collections.flush_if_idle(&self.core.db, now);
        self.state.environments.flush_if_idle(&self.core.db, now);
        self.state.credentials.flush_if_idle(&self.core.db, now);
        let save_errors = [
            self.state.repeater.take_save_errors(),
            self.state.collections.take_save_errors(),
            self.state.environments.take_save_errors(),
            self.state.credentials.take_save_errors(),
        ];
        for error in save_errors.into_iter().flatten() {
            top_bar::flash(ui, Err(error));
        }
        self.state.traffic.sync_header_search(&self.core.db, ui.ctx(), now);
        for (target, decoded) in self.qr_images.drain().flatten() {
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
        for result in self.markdown_files.drain().flatten() {
            match result {
                Ok((name, text)) => self.state.markdown.loaded(name, text),
                Err(e) => top_bar::flash(ui, Err(e)),
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
        for result in self.file_saves.drain().flatten() {
            top_bar::flash(ui, result);
        }
        let resets: Vec<_> = self.factory_resets.drain().collect();
        for result in resets {
            self.finish_factory_reset(ui.ctx(), result);
        }
        {
            let cx = Cx { rt: &self.rt, core: &self.core, ctx: ui.ctx() };
            crate::features::traffic::poll(&mut self.state.traffic, &cx);
            crate::features::webhooks::poll(&mut self.state.webhooks, &cx);
            crate::features::containers::poll(&mut self.state.containers, &cx, now);
            crate::features::websockets::poll(&mut self.state.websockets, &cx);
            crate::features::settings::poll(self.state.settings(), &cx);
            crate::features::rewrite::poll(&mut self.state.rewrite, &cx, now);
            crate::features::intercept::poll(&mut self.state.intercept, &cx);
            crate::features::fuzzer::poll(&mut self.state.fuzzer, &cx);
        }
        crate::features::utilities::hashing::poll(&mut self.state.hashing);
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
