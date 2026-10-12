use std::path::PathBuf;
use eframe::egui::{self, RichText};
use mitm_core::db::{recovery, Db, InitError};
use tokio::runtime::Runtime;
use crate::util::files;
use crate::app::jobs::Jobs;
use crate::ui::theme::{self, Tone};
use crate::widgets::confirm::confirm_button;

pub struct Recovery {
    error: InitError,
    db: Db,
    status: Option<(String, Tone)>,
    exports: Jobs<Option<Result<PathBuf, String>>>,
}

impl Recovery {
    pub fn new(error: InitError) -> Self {
        Self {
            error,
            db: Db::new(mitm_core::db::get_default_db_path()),
            status: None,
            exports: Jobs::default(),
        }
    }

    /// Returns true when the core should be started again.
    pub fn show(&mut self, ui: &mut egui::Ui, rt: &Runtime) -> bool {
        for result in self.exports.drain().flatten() {
            self.status = Some(match result {
                Ok(path) => (format!("Exported to {}", path.display()), Tone::Success),
                Err(e) => (e, Tone::Danger),
            });
        }

        let mut restart = false;
        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height() * 0.3).max(24.0));
                ui.set_max_width(560.0);
                ui.label(RichText::new(heading(&self.error)).heading().color(theme::color(ui, Tone::Danger)));
                ui.add_space(8.0);
                ui.label(RichText::new(self.error.message()).monospace().weak());
                ui.label(RichText::new(self.db.display().to_string()).monospace().weak().small());
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    restart |= ui.add(crate::widgets::button::primary(ui, "Retry migrations")).clicked();
                    let id = ui.id().with("backup_reset");
                    if confirm_button(ui, id, RichText::new("Back up & reset"), "Confirm reset", "Moves the database aside and starts empty") {
                        restart |= self.backup_and_reset();
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
        restart
    }

    fn backup_and_reset(&mut self) -> bool {
        match recovery::backup_and_reset(&self.db, None) {
            Ok(_) => true,
            Err(error) => {
                self.status = Some((error.to_string(), Tone::Danger));
                false
            }
        }
    }

    fn export(&self, rt: &Runtime, ctx: &egui::Context) {
        let db = self.db.clone();
        self.exports.spawn(rt, ctx, async move {
            let destination = files::pick_save("mitm.db", &[("SQLite database", &["db"])]).await?;
            let copy = tokio::task::spawn_blocking(move || {
                recovery::export_database(&db, &destination).map(|_| destination)
            });
            Some(copy.await.unwrap_or_else(|e| Err(e.to_string())))
        });
    }
}

fn heading(error: &InitError) -> &'static str {
    match error {
        InitError::Locked(_) => "Database is locked",
        InitError::Migration(_) => "Database migration failed",
        InitError::Io(_) => "Database could not be opened",
    }
}
