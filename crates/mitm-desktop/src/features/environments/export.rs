use eframe::egui;
use mitm_core::workspace::file::{ExportOptions, AUTO_VARIANT};
use mitm_core::workspace::Environment;
use crate::app::command::{AppCommand, Commands};

/// Asks what goes into the file; only shown when the workspace has secrets or `(auto)` values.
#[derive(Default)]
pub struct ExportPrompt {
    open: bool,
    has_secrets: bool,
    has_auto: bool,
    options: ExportOptions,
}

impl ExportPrompt {
    /// Opens the prompt if there's anything to ask, otherwise returns the options to export with.
    pub fn ask(&mut self, environments: &[Environment]) -> Option<ExportOptions> {
        let variables = || environments.iter().flat_map(|e| &e.variables);
        let has_secrets = variables().any(|v| v.r#type == "secret");
        let has_auto = variables().flat_map(|v| &v.variants).any(|v| v.name == AUTO_VARIANT && !v.value.is_empty());
        if !has_secrets && !has_auto {
            return Some(ExportOptions::default());
        }
        *self = Self { open: true, has_secrets, has_auto, options: ExportOptions::default() };
        None
    }
}

pub fn show(ctx: &egui::Context, prompt: &mut ExportPrompt, cmds: &mut Commands) {
    if !prompt.open {
        return;
    }
    let mut close = false;
    let modal = egui::Modal::new(egui::Id::new("workspace_export")).show(ctx, |ui| {
        ui.heading("Export workspace");
        ui.add_space(6.0);
        if prompt.has_secrets {
            ui.checkbox(&mut prompt.options.include_secrets, "Include secret values");
        }
        if prompt.has_auto {
            ui.checkbox(&mut prompt.options.include_auto, "Include (auto) values")
                .on_hover_text("Values saved by extract rules");
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Export…").clicked() {
                cmds.push(AppCommand::ExportWorkspaceFile(prompt.options));
                close = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });
    if close || modal.should_close() {
        prompt.open = false;
    }
}
