use std::path::PathBuf;
use std::sync::Arc;
use eframe::egui::{self, RichText};
use mitm_core::db::{recovery, InitError};
use mitm_core::MitmCore;
use tokio::runtime::Runtime;
use crate::files;
use crate::jobs::Jobs;
use crate::theme::{self, Tone};
use crate::widgets::confirm::confirm_button;

pub struct Recovery {
    error: InitError,
    db_path: PathBuf,
    status: Option<(String, Tone)>,
    exports: Jobs<Option<Result<PathBuf, String>>>,
}

impl Recovery {
    pub fn new(error: InitError) -> Self {
        Self {
            error,
            db_path: mitm_core::db::get_default_db_path(),
            status: None,
            exports: Jobs::default(),
        }
    }

    /// Returns the core once a retry or reset gets the database working.
    pub fn show(&mut self, ui: &mut egui::Ui, rt: &Runtime) -> Option<Arc<MitmCore>> {
        for result in self.exports.drain().flatten() {
            self.status = Some(match result {
                Ok(path) => (format!("Exported to {}", path.display()), Tone::Success),
                Err(e) => (e, Tone::Danger),
            });
        }

        let mut started = None;
        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height() * 0.3).max(24.0));
                ui.set_max_width(560.0);
                ui.label(RichText::new(heading(&self.error)).heading().color(theme::color(ui, Tone::Danger)));
                ui.add_space(8.0);
                ui.label(RichText::new(self.error.message()).monospace().weak());
                ui.label(RichText::new(self.db_path.display().to_string()).monospace().weak().small());
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui.button("Retry migrations").clicked() {
                        started = self.retry(rt);
                    }
                    let id = ui.id().with("backup_reset");
                    if confirm_button(ui, id, RichText::new("Back up & reset"), "Confirm reset", "Moves the database aside and starts empty") {
                        started = self.backup_and_reset(rt);
                    }
                    if ui.button("Export DB…").clicked() {
                        self.export(rt, ui.ctx());
                    }
                    if ui.button("Quit").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                if let Some((message, tone)) = &self.status {
                    ui.add_space(8.0);
                    ui.label(RichText::new(message).color(theme::color(ui, *tone)));
                }
            });
        });
        started
    }

    fn retry(&mut self, rt: &Runtime) -> Option<Arc<MitmCore>> {
        match start_core(rt) {
            Ok(core) => Some(core),
            Err(error) => {
                self.error = error;
                self.status = Some(("Still failing".to_string(), Tone::Danger));
                None
            }
        }
    }

    fn backup_and_reset(&mut self, rt: &Runtime) -> Option<Arc<MitmCore>> {
        if let Err(error) = recovery::backup_and_reset(&self.db_path, None) {
            self.status = Some((error.to_string(), Tone::Danger));
            return None;
        }
        self.retry(rt)
    }

    fn export(&self, rt: &Runtime, ctx: &egui::Context) {
        let db_path = self.db_path.clone();
        self.exports.spawn(rt, ctx, async move {
            let destination = files::pick_save("mitm.db", &[("SQLite database", &["db"])]).await?;
            let copy = tokio::task::spawn_blocking(move || {
                recovery::export_database(&db_path, &destination).map(|_| destination)
            });
            Some(copy.await.unwrap_or_else(|e| Err(e.to_string())))
        });
    }
}

pub fn start_core(rt: &Runtime) -> Result<Arc<MitmCore>, InitError> {
    let _guard = rt.enter();
    MitmCore::init().map(Arc::new)
}

fn heading(error: &InitError) -> &'static str {
    match error {
        InitError::Locked(_) => "Database is locked",
        InitError::Migration(_) => "Database migration failed",
        InitError::Io(_) => "Database could not be opened",
    }
}
