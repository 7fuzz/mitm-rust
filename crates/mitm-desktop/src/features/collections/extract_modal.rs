use eframe::egui::{self, RichText};
use crate::app::command::{AppCommand, Commands};
use crate::features::request_editor::extract_rules;
use super::{CollectionsCmd, CollectionsState};

pub fn show(ctx: &egui::Context, state: &mut CollectionsState, cmds: &mut Commands) {
    let Some(modal) = state.extract_modal.as_mut() else { return };
    let mut close = false;
    let mut switch_to = None;

    let response = egui::Modal::new(egui::Id::new("collections_extract_modal")).show(ctx, |ui| {
        ui.set_width(640.0);
        ui.heading("Auto-extract rules");
        ui.add_space(8.0);

        if modal.request_ids.len() > 1 {
            let name_of = |id: &str| {
                super::all_requests(&state.trees)
                    .into_iter()
                    .find(|r| r.id == id)
                    .map_or(id.to_string(), |r| format!("{} ({})", r.name, r.method))
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new("Request").weak());
                egui::ComboBox::from_id_salt("extract_modal_request")
                    .selected_text(name_of(&modal.selected))
                    .width(360.0)
                    .show_ui(ui, |ui| {
                        for id in &modal.request_ids {
                            if ui.selectable_label(*id == modal.selected, name_of(id)).clicked() && *id != modal.selected {
                                switch_to = Some(id.clone());
                            }
                        }
                    });
            });
            ui.add_space(4.0);
        }

        crate::widgets::modal_scroll::vertical(420.0).show(ui, |ui| {
            extract_rules::show(ui, "extract_modal_rules", &mut modal.rules);
        });

        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(crate::widgets::button::primary(ui, "Save")).clicked() {
                cmds.push(AppCommand::Collections(CollectionsCmd::SaveExtractRules {
                    request_id: modal.selected.clone(),
                    rules: modal.rules.clone(),
                }));
                close = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });

    if let Some(id) = switch_to {
        let rules = super::all_requests(&state.trees)
            .into_iter()
            .find(|r| r.id == id)
            .map(|r| r.extract_rules.clone())
            .unwrap_or_default();
        if let Some(modal) = state.extract_modal.as_mut() {
            modal.selected = id;
            modal.rules = rules;
        }
    }
    if close || response.should_close() {
        state.extract_modal = None;
    }
}
