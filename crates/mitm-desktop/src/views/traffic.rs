use std::sync::Arc;
use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::MitmCore;
use mitm_core::state::HistoryEntry;
use crate::state::{TriState, UiState};
use crate::views::pills::{format_bytes, format_duration, render_method_pill, render_source_pill, render_status_pill, render_tri_state_button};

pub fn render_traffic_view(
    ui: &mut egui::Ui,
    state: &mut UiState,
    core: &Arc<MitmCore>,
) {
    egui::ScrollArea::horizontal()
        .id_salt("traffic_toolbar_scroll")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text("Filter URL, host, path...")
                        .desired_width(160.0),
                );

                if !state.search_query.is_empty() && ui.small_button("✖").clicked() {
                    state.search_query.clear();
                }

                ui.separator();

                let sources = state.available_source_labels();
                if !sources.is_empty() {
                    egui::ComboBox::from_id_salt("traffic_source_filter_combo")
                        .selected_text(format!("Src: {}", state.source_filter))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.source_filter, "ALL".to_string(), "All Sources");
                            for src in &sources {
                                ui.selectable_value(&mut state.source_filter, src.clone(), src);
                            }
                        });
                    ui.separator();
                }

                ui.label(RichText::new("Method:").weak().size(11.0));
                for m in &["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"] {
                    let s = state.method_filters.entry(m.to_string()).or_default();
                    render_tri_state_button(ui, m, s);
                }

                ui.separator();

                ui.label(RichText::new("Status:").weak().size(11.0));
                for st in &["2xx", "3xx", "4xx", "5xx", "ERR"] {
                    let s = state.status_filters.entry(st.to_string()).or_default();
                    render_tri_state_button(ui, st, s);
                }

                let has_active_filters = state.source_filter != "ALL"
                    || state.method_filters.values().any(|&v| v != TriState::Neutral)
                    || state.status_filters.values().any(|&v| v != TriState::Neutral);

                if has_active_filters && ui.small_button("↺ Reset").on_hover_text("Reset all filters").clicked() {
                    state.reset_filters();
                }

                ui.separator();

                ui.checkbox(&mut state.auto_scroll, "Auto-scroll");

                ui.separator();

                if ui.button(format!("🗑 Clear ({})", state.flows.len())).clicked() {
                    let _ = core.clear_all_traffic();
                    state.clear_flows();
                }
            });
        });

    ui.separator();

    let filtered_indices = state.filtered_flow_indices();
    let num_rows = filtered_indices.len();

    let egui_ctx = ui.ctx().clone();
    let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
    let row_height = text_height + 8.0;

    let mut new_selection: Option<i64> = None;
    let mut should_clear = false;
    let mut send_to_repeater: Option<HistoryEntry> = None;
    let mut send_to_fuzzer: Option<HistoryEntry> = None;

    let available_w = ui.available_width();
    let path_col_width = (available_w - 560.0).max(200.0);

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
                    header.col(|ui| { ui.strong("#"); });
                    header.col(|ui| { ui.strong("Source"); });
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
                        let flow_url = flow.url.clone();
                        let flow_clone = flow.clone();
                        let is_selected = state.selected_flow_id == Some(flow_id);

                        row.set_selected(is_selected);

                        row.col(|ui| {
                            ui.label(flow_id.to_string());
                        });

                        row.col(|ui| {
                            render_source_pill(ui, &flow.listener_label);
                        });

                        row.col(|ui| {
                            render_method_pill(ui, &flow.method);
                        });

                        row.col(|ui| {
                            render_status_pill(ui, flow.status_code);
                        });

                        row.col(|ui| {
                            ui.add(egui::Label::new(&flow.host).truncate());
                        });

                        row.col(|ui| {
                            let mut path_text = RichText::new(&flow.path);
                            if flow.is_failed {
                                path_text = path_text.color(Color32::from_rgb(248, 113, 113));
                            }
                            ui.add(egui::Label::new(path_text).truncate());
                        });

                        row.col(|ui| {
                            let short_type = flow.content_type.split(';').next().unwrap_or("-");
                            ui.add(egui::Label::new(short_type).truncate());
                        });

                        row.col(|ui| {
                            ui.label(format_bytes(flow.response_size));
                        });

                        row.col(|ui| {
                            ui.label(format_duration(flow.duration_ms));
                        });

                        let response = row.response().interact(egui::Sense::click());
                        if response.clicked() {
                            new_selection = Some(flow_id);
                        }
                        if response.double_clicked() {
                            egui_ctx.copy_text(flow_url.clone());
                        }

                        response.context_menu(|ui| {
                            if ui.button("📋 Copy URL").clicked() {
                                ui.ctx().copy_text(flow_url.clone());
                                ui.close();
                            }
                            if ui.button("📋 Copy Host").clicked() {
                                ui.ctx().copy_text(flow_clone.host.clone());
                                ui.close();
                            }
                            if ui.button("📋 Copy Path").clicked() {
                                ui.ctx().copy_text(flow_clone.path.clone());
                                ui.close();
                            }
                            if ui.button("📋 Copy as cURL").clicked() {
                                let curl_cmd = crate::views::inspector::build_curl_command(&flow_clone, state.selected_flow_detail.as_ref());
                                ui.ctx().copy_text(curl_cmd);
                                ui.close();
                            }
                            if ui.button("📋 Copy Entire Row").clicked() {
                                let row_summary = format!(
                                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                                    flow_clone.id,
                                    flow_clone.listener_label,
                                    flow_clone.method,
                                    flow_clone.status_code,
                                    flow_clone.host,
                                    flow_clone.path,
                                    format_bytes(flow_clone.response_size),
                                    format_duration(flow_clone.duration_ms)
                                );
                                ui.ctx().copy_text(row_summary);
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("🔁 Send to Repeater").clicked() {
                                send_to_repeater = Some(flow_clone.clone());
                                ui.close();
                            }
                            if ui.button("⚡ Send to Fuzzer").clicked() {
                                send_to_fuzzer = Some(flow_clone.clone());
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("🗑 Clear Flows").clicked() {
                                should_clear = true;
                                ui.close();
                            }
                        });
                    });
                });
        });

    if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::C)) {
        if let Some(selected) = state.selected_flow() {
            ui.ctx().copy_text(selected.url.clone());
        }
    }

    if let Some(id) = new_selection {
        state.select_flow(id, &core.db_path);
    }
    let detail = state.selected_flow_detail.clone();
    if let Some(f) = send_to_repeater {
        state.send_flow_to_repeater(&f, detail.as_ref());
    }
    if let Some(f) = send_to_fuzzer {
        state.send_flow_to_fuzzer(&f, detail.as_ref());
    }
    if should_clear {
        let _ = core.clear_all_traffic();
        state.clear_flows();
    }
}
