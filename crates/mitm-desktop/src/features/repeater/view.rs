use eframe::egui::{self, Color32, RichText};
use mitm_core::repeater::RepeaterTab;
use crate::command::{AppCommand, Commands};
use crate::format::{format_bytes, format_duration, parse_header_items, pretty_json};
use crate::widgets::pills::{render_method_pill, render_status_pill};
use super::{RepeaterState, RepeaterTabState};

pub fn show(ui: &mut egui::Ui, state: &mut RepeaterState, cmds: &mut Commands) {
    if state.tabs.is_empty() {
        state.tabs.push(RepeaterTabState::default());
        state.active = 0;
    }

    let mut close_tab_idx: Option<usize> = None;
    let mut add_new_tab = false;

    ui.horizontal_wrapped(|ui| {
        for (idx, tab) in state.tabs.iter().enumerate() {
            let is_active = idx == state.active;
            let title = if tab.name.is_empty() { "Request".to_string() } else { tab.name.clone() };

            ui.horizontal(|ui| {
                if ui.selectable_label(is_active, title).clicked() {
                    state.active = idx;
                }
                if state.tabs.len() > 1 && ui.small_button("✖").clicked() {
                    close_tab_idx = Some(idx);
                }
            });
        }

        if ui.button("➕ New Tab").clicked() {
            add_new_tab = true;
        }
    });

    if let Some(idx) = close_tab_idx {
        state.tabs.remove(idx);
        if state.active >= state.tabs.len() {
            state.active = state.tabs.len().saturating_sub(1);
        }
    }

    if add_new_tab {
        state.tabs.push(RepeaterTabState::default());
        state.active = state.tabs.len() - 1;
    }

    ui.separator();

    let tab_idx = state.active;
    let tab = &mut state.tabs[tab_idx];

    let mut should_send = false;

    ui.horizontal(|ui| {
        render_method_pill(ui, &tab.method);

        egui::ComboBox::from_id_salt("repeater_method_combo")
            .selected_text(&tab.method)
            .show_ui(ui, |ui| {
                for m in &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
                    ui.selectable_value(&mut tab.method, m.to_string(), *m);
                }
            });

        ui.add(
            egui::TextEdit::singleline(&mut tab.url)
                .hint_text("Enter target URL (e.g. https://api.example.com/v1/users)")
                .desired_width(ui.available_width() - 140.0),
        );

        if tab.is_loading {
            ui.spinner();
            ui.label("Sending...");
        } else {
            let send_btn = RichText::new("▶ Send")
                .color(Color32::from_rgb(34, 197, 94))
                .strong();
            if ui.button(send_btn).clicked() {
                should_send = true;
            }
        }
    });

    ui.separator();

    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            ui.heading("Request");
            ui.add_space(4.0);

            ui.strong("Headers:");
            ui.add(
                egui::TextEdit::multiline(&mut tab.headers)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("Key: Value (one per line)")
                    .desired_rows(8)
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(6.0);
            ui.strong("Body:");
            ui.add(
                egui::TextEdit::multiline(&mut tab.body)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("Payload body...")
                    .desired_rows(12)
                    .desired_width(f32::INFINITY),
            );
        });

        columns[1].vertical(|ui| {
            ui.heading("Response");
            ui.add_space(4.0);

            if let Some(res) = &tab.last_result {
                ui.horizontal(|ui| {
                    render_status_pill(ui, res.status_code);
                    ui.label(RichText::new(&res.status_text).strong());
                    ui.separator();
                    ui.label(RichText::new("Time:").weak());
                    ui.label(format_duration(Some(res.duration_ms)));
                    ui.separator();
                    ui.label(RichText::new("Size:").weak());
                    ui.label(format_bytes(res.response_size));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("📋 Copy Body").clicked() {
                            ui.ctx().copy_text(res.response_body.clone());
                        }
                    });
                });

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.collapsing(format!("Headers ({})", res.response_headers.len()), |ui| {
                        egui::Grid::new("repeater_res_headers_grid")
                            .striped(true)
                            .num_columns(2)
                            .spacing([12.0, 4.0])
                            .show(ui, |ui| {
                                for h in &res.response_headers {
                                    ui.label(RichText::new(&h.key).color(Color32::from_rgb(147, 197, 253)).strong());
                                    ui.label(&h.value);
                                    ui.end_row();
                                }
                            });
                    });

                    ui.separator();
                    ui.strong("Response Body:");

                    if let Some(json) = pretty_json(&res.response_body) {
                        ui.add(
                            egui::TextEdit::multiline(&mut json.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY),
                        );
                    } else {
                        ui.add(
                            egui::TextEdit::multiline(&mut res.response_body.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY),
                        );
                    }
                });
            } else if let Some(err) = &tab.error {
                ui.colored_label(Color32::RED, format!("Request Error: {}", err));
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(RichText::new("No response yet. Configure your request and click Send.").weak());
                });
            }
        });
    });

    if should_send {
        tab.is_loading = true;
        tab.error = None;

        let req_tab = RepeaterTab {
            id: tab.id.clone(),
            method: tab.method.clone(),
            url: tab.url.clone(),
            headers: parse_header_items(&tab.headers),
            params: Vec::new(),
            body_type: "raw".to_string(),
            body_content: if tab.body.is_empty() { None } else { Some(tab.body.clone()) },
            extract_rules: Vec::new(),
            pre_requests: Vec::new(),
            post_requests: Vec::new(),
            order_index: tab_idx as i32,
            created_at_ms: chrono::Utc::now().timestamp_millis(),
            updated_at_ms: chrono::Utc::now().timestamp_millis(),
            execution_count: 0,
            last_status_code: None,
            last_duration_ms: None,
        };

        cmds.push(AppCommand::SendRepeaterRequest {
            tab_id: tab.id.clone(),
            request: req_tab,
        });
    }
}
