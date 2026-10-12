use std::collections::{BTreeMap, HashMap};
use eframe::egui::{self, RichText};
use crate::app::command::{AppCommand, Commands};
use crate::ui::theme::{self, Tone};
use crate::widgets::tri_state::{tri_state_button, TriState};
use super::HarExport;
use crate::ui::icons;

pub fn show(ctx: &egui::Context, export: &mut HarExport, cmds: &mut Commands) {
    if !export.open {
        return;
    }
    let mut close = false;
    let modal = egui::Modal::new(egui::Id::new("har_export")).show(ctx, |ui| {
        ui.set_width(560.0);
        ui.heading("Export HAR");
        ui.add_space(6.0);
        if let Some(error) = &export.error {
            ui.label(RichText::new(error).color(theme::color(ui, Tone::Danger)));
        } else if export.rows.is_none() {
            ui.spinner();
        } else {
            filters(ui, export);
        }

        ui.add_space(6.0);
        ui.separator();
        let ids = export.selected_ids();
        ui.horizontal(|ui| {
            let total = export.rows.as_ref().map_or(0, Vec::len);
            ui.label(RichText::new(ids.len().to_string()).strong().monospace());
            ui.label(RichText::new(format!("of {} requests", total)).weak().monospace());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    close = true;
                }
                if ui.add_enabled(!ids.is_empty(), egui::Button::new("Export…")).clicked() {
                    cmds.push(AppCommand::ExportHar(ids));
                    close = true;
                }
            });
        });
    });
    if close || modal.should_close() {
        export.open = false;
        export.rows = None;
    }
}

fn filters(ui: &mut egui::Ui, export: &mut HarExport) {
    group_label(ui, "HOSTS");
    hosts(ui, export);

    ui.add_space(6.0);
    let facets = &export.facets;
    let filters = &mut export.filters;
    chip_row(ui, "METHOD", &facets.methods, &mut filters.methods);
    chip_row(ui, "RESPONSE", &facets.responses, &mut filters.responses);
    if facets.sources.len() > 1 {
        chip_row(ui, "SOURCE", &facets.sources, &mut filters.sources);
    }
    ui.horizontal(|ui| {
        group_label(ui, "DUPLICATES");
        ui.selectable_value(&mut filters.unique_only, false, "All");
        ui.selectable_value(&mut filters.unique_only, true, "Unique only")
            .on_hover_text("One request per method, host and path, the newest");
    });
}

fn hosts(ui: &mut egui::Ui, export: &mut HarExport) {
    let mut bulk = None;
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut export.host_search).hint_text(format!("{} Search hosts...", icons::MAGNIFYING_GLASS)).desired_width(240.0));
        if ui.small_button("Include all").clicked() {
            bulk = Some(TriState::Include);
        }
        if ui.small_button("Exclude all").clicked() {
            bulk = Some(TriState::Exclude);
        }
        if ui.small_button("Reset").clicked() {
            bulk = Some(TriState::Neutral);
        }
    });
    if let Some(state) = bulk {
        export.set_matching_hosts(state);
    }

    let search = export.host_search.trim().to_lowercase();
    let matching: Vec<(&String, &usize)> =
        export.facets.hosts.iter().filter(|(host, _)| host.to_lowercase().contains(&search)).collect();
    let row_height = ui.spacing().interact_size.y;
    egui::ScrollArea::vertical()
        .max_height(240.0)
        .auto_shrink([false, true])
        .show_rows(ui, row_height, matching.len(), |ui, range| {
            for (host, count) in &matching[range] {
                ui.horizontal(|ui| {
                    tri_state_button(ui, host, export.filters.hosts.entry((*host).clone()).or_default());
                    ui.label(RichText::new(count.to_string()).weak().monospace());
                });
            }
        });
}

fn chip_row(ui: &mut egui::Ui, label: &str, values: &BTreeMap<String, usize>, filters: &mut HashMap<String, TriState>) {
    ui.horizontal_wrapped(|ui| {
        group_label(ui, label);
        for value in values.keys() {
            tri_state_button(ui, value, filters.entry(value.clone()).or_default());
        }
    });
}

fn group_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).weak().small().strong());
}
