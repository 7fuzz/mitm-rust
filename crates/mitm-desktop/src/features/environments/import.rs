use std::path::Path;
use eframe::egui::{self, RichText};
use mitm_core::workspace::import::{preview_workspace_import_db, ImportPreview};
use crate::app::command::{AppCommand, Commands};
use crate::ui::theme::{self, Tone};
use crate::ui::icons;

#[derive(Default)]
pub struct ImportState {
    pub open: bool,
    json: String,
    file_label: Option<String>,
    preview: Option<Result<ImportPreview, String>>,
    stale: bool,
    into_current: bool,
    name: String,
    passphrase: String,
    pub error: Option<String>,
}

impl ImportState {
    pub fn open_with(&mut self, file_label: Option<String>, json: String) {
        *self = Self { open: true, json, file_label, stale: true, ..Self::default() };
    }
}

pub fn show(ctx: &egui::Context, state: &mut ImportState, current_workspace: (&str, &str), db_path: &Path, cmds: &mut Commands) {
    if !state.open {
        return;
    }
    if state.stale {
        state.stale = false;
        state.error = None;
        state.preview = (!state.json.trim().is_empty()).then(|| preview_workspace_import_db(&db_path.to_path_buf(), &state.json));
    }

    let (current_id, current_name) = current_workspace;
    let mut close = false;
    let modal = egui::Modal::new(egui::Id::new("workspace_import")).show(ctx, |ui| {
        ui.set_width(480.0);
        ui.heading("Import workspace");
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            if ui.button("Choose file…").clicked() {
                cmds.push(AppCommand::PickImportFile);
            }
            if let Some(label) = &state.file_label {
                ui.label(RichText::new(label).monospace());
                if crate::widgets::button::icon_button(ui, icons::X, "Clear").clicked() {
                    state.open_with(None, String::new());
                }
            }
        });
        if state.file_label.is_none() {
            let paste = ui.add(
                egui::TextEdit::multiline(&mut state.json)
                    .hint_text("…or paste JSON")
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(6)
                    .desired_width(f32::INFINITY),
            );
            state.stale |= paste.changed();
        }

        ui.add_space(6.0);
        let ready = match &state.preview {
            Some(Ok(preview)) => {
                let s = &preview.summary;
                let mut line = format!(
                    "{} · {} collections · {} requests · {} environments",
                    preview.format.label(),
                    s.collections_imported,
                    s.requests_imported,
                    s.environments_imported
                );
                if s.credentials_imported > 0 {
                    line.push_str(&format!(" · {} credentials", s.credentials_imported));
                }
                ui.label(line);
                if preview.credentials_sealed {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.passphrase)
                            .password(true)
                            .hint_text("Credentials passphrase (empty: names only)")
                            .desired_width(280.0),
                    );
                }
                true
            }
            Some(Err(e)) => {
                ui.label(RichText::new(e).color(theme::color(ui, Tone::Danger)));
                false
            }
            None => false,
        };
        if let Some(error) = &state.error {
            ui.label(RichText::new(error).color(theme::color(ui, Tone::Danger)));
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.radio_value(&mut state.into_current, false, "New workspace");
            let detected = match &state.preview {
                Some(Ok(preview)) => preview.summary.workspace_name.as_str(),
                _ => "Name",
            };
            ui.add_enabled(!state.into_current, egui::TextEdit::singleline(&mut state.name).hint_text(detected).desired_width(180.0));
        });
        ui.radio_value(&mut state.into_current, true, format!("Into {current_name}"));

        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_enabled(ready, crate::widgets::button::primary(ui, "Import")).clicked() {
                let name = state.name.trim();
                cmds.push(AppCommand::ImportWorkspace {
                    json: state.json.clone(),
                    target: state.into_current.then(|| current_id.to_string()),
                    name: (!state.into_current && !name.is_empty()).then(|| name.to_string()),
                    passphrase: (!state.passphrase.is_empty()).then(|| state.passphrase.clone()),
                });
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });
    if close || modal.should_close() {
        *state = ImportState::default();
    }
}

pub fn is_json_file(name: &str) -> bool {
    Path::new(name).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
}

pub fn file_label(name: &str, bytes: usize) -> String {
    format!("{name} · {:.1} KB", bytes as f64 / 1024.0)
}
