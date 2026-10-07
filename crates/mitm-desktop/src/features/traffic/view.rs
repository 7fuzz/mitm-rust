use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column, TableBuilder, TableRow};
use mitm_core::state::{HistoryEntry, ListenerConfig};
use crate::command::{AppCommand, Commands};
use crate::dock::Tab;
use crate::format::{format_bytes, format_duration};
use crate::widgets::pills::{render_method_pill, render_source_pill, render_status_pill};
use crate::widgets::tri_state::tri_state_button;
use super::{TrafficState, METHODS, STATUS_GROUPS};

pub fn show(ui: &mut egui::Ui, state: &mut TrafficState, listeners: &[ListenerConfig], cmds: &mut Commands) {
    toolbar(ui, state, listeners, cmds);
    filter_row(ui, state);
    ui.separator();
    table(ui, state, cmds);

    if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::C)) {
        if let Some(selected) = state.selected_flow() {
            ui.ctx().copy_text(selected.url.clone());
        }
    }
}

fn toolbar(ui: &mut egui::Ui, state: &mut TrafficState, listeners: &[ListenerConfig], cmds: &mut Commands) {
    ui.horizontal(|ui| {
        ui.label("🔍");
        ui.add(
            egui::TextEdit::singleline(&mut state.search_query)
                .hint_text("Filter URL, host, path...")
                .desired_width(170.0),
        );

        if !state.search_query.is_empty() && ui.small_button("✖").clicked() {
            state.search_query.clear();
        }

        let sources = state.source_labels(listeners);
        if !sources.is_empty() {
            ui.separator();
            egui::ComboBox::from_id_salt("traffic_source_filter_combo")
                .selected_text(format!("Src: {}", state.source_filter))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.source_filter, "ALL".to_string(), "All Sources");
                    for src in &sources {
                        ui.selectable_value(&mut state.source_filter, src.clone(), src);
                    }
                });
        }

        ui.separator();
        ui.checkbox(&mut state.auto_scroll, "Auto-scroll");

        ui.separator();
        if ui.button(format!("🗑 Clear ({})", state.flows.len())).clicked() {
            cmds.push(AppCommand::ClearTraffic);
        }
    });
}

fn filter_row(ui: &mut egui::Ui, state: &mut TrafficState) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Method:").weak().size(11.0));
        for m in METHODS {
            tri_state_button(ui, m, state.method_filters.entry(m.to_string()).or_default());
        }

        ui.separator();

        ui.label(RichText::new("Status:").weak().size(11.0));
        for st in STATUS_GROUPS {
            tri_state_button(ui, st, state.status_filters.entry(st.to_string()).or_default());
        }

        if state.has_active_filters() {
            ui.separator();
            if ui.small_button("↺ Reset").on_hover_text("Reset all filters").clicked() {
                state.reset_filters();
            }
        }
    });
}

fn table(ui: &mut egui::Ui, state: &mut TrafficState, cmds: &mut Commands) {
    let filtered = state.filtered_indices();
    let row_height = egui::TextStyle::Body.resolve(ui.style()).size + 8.0;
    let path_col_width = (ui.available_width() - 560.0).max(200.0);

    egui::ScrollArea::horizontal()
        .id_salt("traffic_table_hscroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut builder = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(45.0))
                .column(Column::initial(70.0).at_least(50.0))
                .column(Column::exact(60.0))
                .column(Column::exact(50.0))
                .column(Column::initial(150.0).at_least(90.0))
                .column(Column::initial(path_col_width).at_least(180.0))
                .column(Column::initial(80.0).at_least(60.0))
                .column(Column::exact(60.0))
                .column(Column::exact(55.0));

            if state.auto_scroll {
                builder = builder.scroll_to_row(0, Some(egui::Align::TOP));
            }

            builder
                .header(24.0, |mut header| {
                    for title in ["#", "Source", "Method", "Status", "Host", "Path", "Type", "Size", "Time"] {
                        header.col(|ui| {
                            ui.strong(title);
                        });
                    }
                })
                .body(|body| {
                    body.rows(row_height, filtered.len(), |mut row| {
                        let flow = &state.flows[filtered[row.index()]];
                        row.set_selected(state.selected_id == Some(flow.id));
                        flow_row(&mut row, flow, cmds);
                    });
                });
        });
}

fn flow_row(row: &mut TableRow, flow: &HistoryEntry, cmds: &mut Commands) {
    let mut text_cells: Vec<egui::Response> = Vec::new();

    row.col(|ui| {
        text_cells.push(ui.label(flow.id.to_string()));
    });
    row.col(|ui| render_source_pill(ui, &flow.listener_label));
    row.col(|ui| render_method_pill(ui, &flow.method));
    row.col(|ui| render_status_pill(ui, flow.status_code));
    row.col(|ui| {
        text_cells.push(ui.add(egui::Label::new(&flow.host).truncate()));
    });
    row.col(|ui| {
        let mut path_text = RichText::new(&flow.path);
        if flow.is_failed {
            path_text = path_text.color(Color32::from_rgb(248, 113, 113));
        }
        text_cells.push(ui.add(egui::Label::new(path_text).truncate()));
    });
    row.col(|ui| {
        let short_type = flow.content_type.split(';').next().unwrap_or("-");
        text_cells.push(ui.add(egui::Label::new(short_type).truncate()));
    });
    row.col(|ui| {
        text_cells.push(ui.label(format_bytes(flow.response_size)));
    });
    row.col(|ui| {
        text_cells.push(ui.label(format_duration(flow.duration_ms)));
    });

    // Selectable labels swallow clicks; a drag-to-select never registers as a click, so merging keeps text selection usable.
    let response = text_cells
        .into_iter()
        .fold(row.response().interact(egui::Sense::click()), |acc, r| acc | r);

    if response.clicked() || response.double_clicked() {
        cmds.push(AppCommand::SelectFlow(flow.id));
        cmds.push(AppCommand::OpenTab(Tab::Inspector));
    }
    if response.double_clicked() {
        response.ctx.copy_text(flow.url.clone());
    }

    response.context_menu(|ui| flow_context_menu(ui, flow, cmds));
}

fn flow_context_menu(ui: &mut egui::Ui, flow: &HistoryEntry, cmds: &mut Commands) {
    let copies = [
        ("📋 Copy URL", flow.url.clone()),
        ("📋 Copy Host", flow.host.clone()),
        ("📋 Copy Path", flow.path.clone()),
    ];
    for (label, text) in copies {
        if ui.button(label).clicked() {
            ui.ctx().copy_text(text);
            ui.close();
        }
    }
    if ui.button("📋 Copy as cURL").clicked() {
        cmds.push(AppCommand::CopyFlowAsCurl(flow.id));
        ui.close();
    }
    if ui.button("📋 Copy Entire Row").clicked() {
        ui.ctx().copy_text(row_summary(flow));
        ui.close();
    }
    ui.separator();
    if ui.button("🔁 Send to Repeater").clicked() {
        cmds.push(AppCommand::SendFlowToRepeater(flow.id));
        ui.close();
    }
    if ui.button("⚡ Send to Fuzzer").clicked() {
        cmds.push(AppCommand::SendFlowToFuzzer(flow.id));
        ui.close();
    }
    ui.separator();
    if ui.button("🗑 Clear Flows").clicked() {
        cmds.push(AppCommand::ClearTraffic);
        ui.close();
    }
}

fn row_summary(flow: &HistoryEntry) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        flow.id,
        flow.listener_label,
        flow.method,
        flow.status_code,
        flow.host,
        flow.path,
        format_bytes(flow.response_size),
        format_duration(flow.duration_ms)
    )
}
