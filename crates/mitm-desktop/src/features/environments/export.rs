use eframe::egui;
use mitm_core::credentials::Credential;
use mitm_core::workspace::file::{ExportOptions, AUTO_VARIANT};
use mitm_core::workspace::Environment;
use crate::app::command::{AppCommand, Commands};

/// Asks what goes into the file; only shown when the workspace has secrets, `(auto)` values or credentials.
#[derive(Default)]
pub struct ExportPrompt {
    open: bool,
    has_secrets: bool,
    has_auto: bool,
    has_credentials: bool,
    options: ExportOptions,
    include_credentials: bool,
    passphrase: String,
    repeat: String,
}

impl ExportPrompt {
    /// Opens the prompt if there's anything to ask, otherwise returns the options to export with.
    pub fn ask(&mut self, environments: &[Environment], credentials: &[Credential]) -> Option<ExportOptions> {
        let variables = || environments.iter().flat_map(|e| &e.variables);
        let has_secrets = variables().any(|v| v.r#type == "secret");
        let has_auto = variables().flat_map(|v| &v.variants).any(|v| v.name == AUTO_VARIANT && !v.value.is_empty())
            || credentials.iter().flat_map(|c| &c.fields).any(|f| f.extracted && !f.value.is_empty());
        let has_credentials = !credentials.is_empty();
        if !has_secrets && !has_auto && !has_credentials {
            return Some(ExportOptions::default());
        }
        *self = Self { open: true, has_secrets, has_auto, has_credentials, ..Self::default() };
        None
    }
}

pub fn show(ctx: &egui::Context, prompt: &mut ExportPrompt, cmds: &mut Commands) {
    if !prompt.open {
        return;
    }
    let mut close = false;
    let modal = egui::Modal::new(egui::Id::new("workspace_export")).show(ctx, |ui| {
        ui.set_width(480.0);
        ui.heading("Export workspace");
        ui.add_space(6.0);
        if prompt.has_secrets {
            ui.checkbox(&mut prompt.options.include_secrets, "Include secret values");
        }
        if prompt.has_auto {
            ui.checkbox(&mut prompt.options.include_auto, "Include (auto) values")
                .on_hover_text("Values saved by extract rules");
        }
        if prompt.has_credentials {
            ui.checkbox(&mut prompt.include_credentials, "Include credentials");
            if prompt.include_credentials {
                ui.add(egui::TextEdit::singleline(&mut prompt.passphrase).password(true).hint_text("Passphrase"));
                ui.add(egui::TextEdit::singleline(&mut prompt.repeat).password(true).hint_text("Repeat passphrase"));
            }
        }
        let passphrase_ok = !prompt.include_credentials || (!prompt.passphrase.is_empty() && prompt.passphrase == prompt.repeat);
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_enabled(passphrase_ok, crate::widgets::button::primary(ui, "Export…")).clicked() {
                cmds.push(AppCommand::ExportWorkspaceFile {
                    options: prompt.options,
                    passphrase: prompt.include_credentials.then(|| prompt.passphrase.clone()),
                });
                close = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });
    if close || modal.should_close() {
        *prompt = ExportPrompt::default();
    }
}
