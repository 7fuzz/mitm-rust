use eframe::egui::{self, RichText};
use crate::state::UiState;
use crate::views::pills::{format_bytes, format_duration, render_method_pill, render_status_pill};

pub fn render_inspector_view(ui: &mut egui::Ui, state: &mut UiState) {
    let flow = match state.selected_flow() {
        Some(f) => f.clone(),
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.heading("No Flow Selected");
                ui.label("Select a request from the traffic table to view its details.");
            });
            return;
        }
    };

    ui.horizontal(|ui| {
        render_method_pill(ui, &flow.method);
        render_status_pill(ui, flow.status_code);
        ui.label(RichText::new(&flow.url).strong());
    });

    ui.separator();

    ui.horizontal(|ui| {
        ui.label(RichText::new("Duration:").weak());
        ui.label(format_duration(flow.duration_ms));
        ui.separator();
        ui.label(RichText::new("Size:").weak());
        ui.label(format_bytes(flow.response_size));
        ui.separator();
        ui.label(RichText::new("Type:").weak());
        ui.label(&flow.content_type);
    });

    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("inspector_metadata_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                ui.strong("ID:");
                ui.label(flow.id.to_string());
                ui.end_row();

                ui.strong("Host:");
                ui.label(&flow.host);
                ui.end_row();

                ui.strong("Path:");
                ui.label(&flow.path);
                ui.end_row();

                ui.strong("Phase:");
                ui.label(&flow.phase);
                ui.end_row();

                if let Some(req_at) = &flow.request_at {
                    ui.strong("Request At:");
                    ui.label(req_at);
                    ui.end_row();
                }

                if let Some(res_at) = &flow.response_at {
                    ui.strong("Response At:");
                    ui.label(res_at);
                    ui.end_row();
                }

                ui.strong("Intercepted:");
                ui.label(if flow.is_intercepted { "Yes" } else { "No" });
                ui.end_row();

                ui.strong("Rewritten:");
                ui.label(if flow.is_rewritten { "Yes" } else { "No" });
                ui.end_row();

                ui.strong("Failed:");
                ui.label(if flow.is_failed { "Yes" } else { "No" });
                ui.end_row();
            });
    });
}
