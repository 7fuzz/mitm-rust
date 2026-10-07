use eframe::egui::{self, RichText};
use crate::state::UiState;

pub fn render_settings_view(ui: &mut egui::Ui, state: &mut UiState) {
    ui.heading("Proxy Settings & System Info");
    ui.separator();

    egui::Grid::new("settings_grid")
        .num_columns(2)
        .spacing([12.0, 10.0])
        .show(ui, |ui| {
            ui.strong("Proxy Port:");
            ui.horizontal(|ui| {
                if state.proxy_running {
                    ui.label(state.proxy_port.to_string());
                    ui.label(RichText::new("(running, stop proxy to change)").weak());
                } else {
                    ui.add(egui::DragValue::new(&mut state.proxy_port).range(1024..=65535));
                }
            });
            ui.end_row();

            ui.strong("Listener Label:");
            ui.label(&state.proxy_label);
            ui.end_row();

            ui.strong("Status:");
            if state.proxy_running {
                ui.label(RichText::new("● Active").color(egui::Color32::GREEN));
            } else {
                ui.label(RichText::new("○ Stopped").color(egui::Color32::GRAY));
            }
            ui.end_row();

            if let Some(err) = &state.proxy_error {
                ui.strong("Last Error:");
                ui.colored_label(egui::Color32::RED, err);
                ui.end_row();
            }

            ui.strong("Memory Flows:");
            ui.label(format!("{}/{}", state.flows.len(), state.max_flows));
            ui.end_row();
        });
}
