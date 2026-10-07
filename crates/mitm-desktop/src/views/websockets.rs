use std::path::PathBuf;
use eframe::egui::{self, Color32, CornerRadius, RichText};
use egui_extras::{Column, TableBuilder};
use crate::state::UiState;
use crate::views::pills::format_bytes;

pub fn render_websockets_view(ui: &mut egui::Ui, state: &mut UiState, db_path: &PathBuf) {
    ui.horizontal(|ui| {
        ui.heading("WebSocket Sessions");
        ui.separator();
        if ui.button("🔄 Refresh Sessions").clicked() {
            state.load_ws_connections(db_path);
        }
    });

    ui.separator();

    // Split: Sessions List on Left, Messages Timeline on Right
    ui.columns(2, |columns| {
        // Left Column: Sessions Table
        columns[0].vertical(|ui| {
            ui.heading(format!("Sessions ({})", state.ws_connections.len()));
            ui.add_space(4.0);

            if state.ws_connections.is_empty() {
                ui.label(RichText::new("No WebSocket connections captured.").weak());
                return;
            }

            let num_rows = state.ws_connections.len();
            let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
            let row_height = text_height + 6.0;

            let mut new_conn_selection: Option<i64> = None;

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(45.0))    // #
                .column(Column::remainder())    // URL
                .column(Column::exact(70.0))    // Status
                .column(Column::exact(50.0))    // Msgs
                .header(22.0, |mut header| {
                    header.col(|ui| { ui.strong("#"); });
                    header.col(|ui| { ui.strong("URL"); });
                    header.col(|ui| { ui.strong("Status"); });
                    header.col(|ui| { ui.strong("Msgs"); });
                })
                .body(|body| {
                    body.rows(row_height, num_rows, |mut row| {
                        let idx = row.index();
                        let conn = &state.ws_connections[idx];
                        let is_selected = state.selected_ws_connection_id == Some(conn.connection_id);
                        row.set_selected(is_selected);

                        row.col(|ui| {
                            if ui.selectable_label(is_selected, conn.connection_id.to_string()).clicked() {
                                new_conn_selection = Some(conn.connection_id);
                            }
                        });

                        row.col(|ui| {
                            ui.add(egui::Label::new(&conn.url).truncate());
                        });

                        row.col(|ui| {
                            render_ws_status_badge(ui, &conn.status);
                        });

                        row.col(|ui| {
                            ui.label(conn.message_count.to_string());
                        });

                        if row.response().clicked() {
                            new_conn_selection = Some(conn.connection_id);
                        }
                    });
                });

            if let Some(id) = new_conn_selection {
                state.select_ws_connection(id, db_path);
            }
        });

        // Right Column: Messages Timeline
        columns[1].vertical(|ui| {
            let conn_id = match state.selected_ws_connection_id {
                Some(id) => id,
                None => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.heading("No Session Selected");
                        ui.label("Select a WebSocket connection to view messages.");
                    });
                    return;
                }
            };

            ui.horizontal(|ui| {
                ui.heading(format!("Session #{} Messages ({})", conn_id, state.ws_messages.len()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔄 Refresh Messages").clicked() {
                        state.select_ws_connection(conn_id, db_path);
                    }
                });
            });

            ui.separator();

            // Direction Filter
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.selectable_value(&mut state.ws_filter_direction, "ALL".to_string(), "All");
                ui.selectable_value(&mut state.ws_filter_direction, "IN".to_string(), "Incoming (Server → Client)");
                ui.selectable_value(&mut state.ws_filter_direction, "OUT".to_string(), "Outgoing (Client → Server)");
            });

            ui.separator();

            // Messages table
            let filtered_messages: Vec<&mitm_core::state::WebSocketMessage> = state
                .ws_messages
                .iter()
                .filter(|m| {
                    match state.ws_filter_direction.as_str() {
                        "IN" => m.direction.eq_ignore_ascii_case("server") || m.direction.eq_ignore_ascii_case("incoming"),
                        "OUT" => m.direction.eq_ignore_ascii_case("client") || m.direction.eq_ignore_ascii_case("outgoing"),
                        _ => true,
                    }
                })
                .collect();

            if filtered_messages.is_empty() {
                ui.label(RichText::new("No messages in this session.").weak());
                return;
            }

            let num_msgs = filtered_messages.len();
            let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
            let row_height = text_height + 6.0;

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(70.0))    // Dir
                .column(Column::exact(55.0))    // Size
                .column(Column::remainder())    // Payload Preview
                .header(22.0, |mut header| {
                    header.col(|ui| { ui.strong("Dir"); });
                    header.col(|ui| { ui.strong("Size"); });
                    header.col(|ui| { ui.strong("Payload"); });
                })
                .body(|body| {
                    body.rows(row_height, num_msgs, |mut row| {
                        let idx = row.index();
                        let msg = filtered_messages[idx];

                        row.col(|ui| {
                            render_ws_direction_badge(ui, &msg.direction);
                        });

                        row.col(|ui| {
                            ui.label(format_bytes(msg.length as u64));
                        });

                        row.col(|ui| {
                            let preview = if msg.payload.len() > 80 {
                                format!("{}...", &msg.payload[..77])
                            } else {
                                msg.payload.clone()
                            };
                            ui.add(egui::Label::new(RichText::new(preview).monospace()).truncate());
                        });
                    });
                });
        });
    });
}

fn render_ws_status_badge(ui: &mut egui::Ui, status: &str) {
    let (bg_color, text_color) = if status.eq_ignore_ascii_case("connected") || status.eq_ignore_ascii_case("open") {
        (Color32::from_rgb(20, 83, 45), Color32::from_rgb(187, 247, 208))
    } else {
        (Color32::from_rgb(55, 65, 81), Color32::from_rgb(209, 213, 219))
    };

    let text = RichText::new(format!(" {} ", status.to_uppercase()))
        .color(text_color)
        .size(10.0)
        .strong();

    egui::Frame::NONE
        .fill(bg_color)
        .corner_radius(CornerRadius::same(3))
        .show(ui, |ui| {
            ui.label(text);
        });
}

fn render_ws_direction_badge(ui: &mut egui::Ui, direction: &str) {
    let is_incoming = direction.eq_ignore_ascii_case("server") || direction.eq_ignore_ascii_case("incoming");
    let (bg_color, text_color, label) = if is_incoming {
        (Color32::from_rgb(20, 83, 45), Color32::from_rgb(187, 247, 208), "▼ SVR")
    } else {
        (Color32::from_rgb(30, 64, 175), Color32::from_rgb(191, 219, 254), "▲ CLI")
    };

    let text = RichText::new(format!(" {} ", label))
        .color(text_color)
        .size(10.0)
        .strong();

    egui::Frame::NONE
        .fill(bg_color)
        .corner_radius(CornerRadius::same(3))
        .show(ui, |ui| {
            ui.label(text);
        });
}
