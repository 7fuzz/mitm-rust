use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column, TableBuilder};
use crate::state::UiState;
use crate::views::pills::{format_bytes, format_duration, render_method_pill, render_status_pill};

pub fn render_traffic_view(ui: &mut egui::Ui, state: &mut UiState) {
    // 1. Toolbar on top
    ui.horizontal(|ui| {
        // Search bar
        ui.label("🔍");
        ui.add(
            egui::TextEdit::singleline(&mut state.search_query)
                .hint_text("Filter by URL, host, path...")
                .desired_width(220.0),
        );

        if !state.search_query.is_empty() && ui.small_button("✖").clicked() {
            state.search_query.clear();
        }

        ui.separator();

        // Method selector combo
        egui::ComboBox::from_id_salt("method_filter_combo")
            .selected_text(format!("Method: {}", state.method_filter))
            .show_ui(ui, |ui| {
                for method in &["ALL", "GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"] {
                    ui.selectable_value(&mut state.method_filter, method.to_string(), *method);
                }
            });

        // Status code filter
        egui::ComboBox::from_id_salt("status_filter_combo")
            .selected_text(format!("Status: {}", state.status_filter))
            .show_ui(ui, |ui| {
                for status in &["ALL", "2xx", "3xx", "4xx", "5xx", "ERR"] {
                    ui.selectable_value(&mut state.status_filter, status.to_string(), *status);
                }
            });

        ui.separator();

        // Auto scroll checkbox
        ui.checkbox(&mut state.auto_scroll, "Auto-scroll");

        ui.separator();

        // Clear button
        if ui.button(format!("🗑 Clear ({})", state.flows.len())).clicked() {
            state.clear_flows();
        }
    });

    ui.separator();

    // 2. Virtualized Traffic Table
    let filtered_indices = state.filtered_flow_indices();
    let num_rows = filtered_indices.len();

    let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
    let row_height = text_height + 8.0;

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(50.0))    // ID
        .column(Column::exact(65.0))    // Method
        .column(Column::exact(55.0))    // Status
        .column(Column::initial(150.0)) // Host
        .column(Column::remainder())    // Path
        .column(Column::initial(90.0))  // Type
        .column(Column::exact(65.0))    // Size
        .column(Column::exact(60.0))    // Time
        .header(24.0, |mut header| {
            header.col(|ui| { ui.strong("#"); });
            header.col(|ui| { ui.strong("Method"); });
            header.col(|ui| { ui.strong("Status"); });
            header.col(|ui| { ui.strong("Host"); });
            header.col(|ui| { ui.strong("Path"); });
            header.col(|ui| { ui.strong("Type"); });
            header.col(|ui| { ui.strong("Size"); });
            header.col(|ui| { ui.strong("Time"); });
        })
        .body(|body| {
            body.rows(row_height, num_rows, |mut row| {
                let row_idx = row.index();
                let flow_idx = filtered_indices[row_idx];
                let flow = &state.flows[flow_idx];
                let flow_id = flow.id;
                let is_selected = state.selected_flow_id == Some(flow_id);

                row.set_selected(is_selected);

                // ID
                row.col(|ui| {
                    if ui.selectable_label(is_selected, flow_id.to_string()).clicked() {
                        state.selected_flow_id = Some(flow_id);
                    }
                });

                // Method
                row.col(|ui| {
                    render_method_pill(ui, &flow.method);
                });

                // Status
                row.col(|ui| {
                    render_status_pill(ui, flow.status_code);
                });

                // Host
                row.col(|ui| {
                    ui.add(egui::Label::new(&flow.host).truncate());
                });

                // Path
                row.col(|ui| {
                    let mut path_text = RichText::new(&flow.path);
                    if flow.is_failed {
                        path_text = path_text.color(Color32::from_rgb(248, 113, 113));
                    }
                    ui.add(egui::Label::new(path_text).truncate());
                });

                // Content Type
                row.col(|ui| {
                    let short_type = flow.content_type.split(';').next().unwrap_or("-");
                    ui.add(egui::Label::new(short_type).truncate());
                });

                // Size
                row.col(|ui| {
                    ui.label(format_bytes(flow.response_size));
                });

                // Time / Duration
                row.col(|ui| {
                    ui.label(format_duration(flow.duration_ms));
                });

                if row.response().clicked() {
                    state.selected_flow_id = Some(flow_id);
                }
            });
        });
}
