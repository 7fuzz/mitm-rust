use eframe::egui;
use crate::command::{AppCommand, Commands};

/// Asks whether secret values go into the file; only shown when the workspace has secrets.
#[derive(Default)]
pub struct ExportPrompt {
    open: bool,
    include_secrets: bool,
}

impl ExportPrompt {
    pub fn open(&mut self) {
        *self = Self { open: true, include_secrets: false };
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
        ui.checkbox(&mut prompt.include_secrets, "Include secret values");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Export…").clicked() {
                cmds.push(AppCommand::ExportWorkspaceFile { include_secrets: prompt.include_secrets });
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
