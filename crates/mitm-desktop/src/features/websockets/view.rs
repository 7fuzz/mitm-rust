use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::state::WebSocketConn;
use crate::command::{AppCommand, Commands};
use crate::format::format_bytes;
use crate::theme::{self, Tone};
use crate::widgets::pills::badge;
use crate::widgets::split;
use super::WebSocketsState;

pub fn show(ui: &mut egui::Ui, state: &mut WebSocketsState, cmds: &mut Commands) {
    super::connect::show(ui.ctx(), &mut state.connect, cmds);
    let [mut left, mut right] = split::side_by_side(ui, "ws_list_split", &mut state.prefs.list_split);
    sessions_list(&mut left, state, cmds);
    frames_and_detail(&mut right, state, cmds);
}

fn sessions_list(ui: &mut egui::Ui, state: &mut WebSocketsState, cmds: &mut Commands) {
    ui.horizontal(|ui| {
        ui.heading(format!("Sessions ({})", state.connections.len()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("➕").on_hover_text("Connect a WebSocket").clicked() {
                state.connect.open();
            }
            let mitm = ui
                .toggle_value(&mut state.mitm_enabled, "MITM")
                .on_hover_text("Capture proxied WebSocket frames (new connections)");
            if mitm.changed() {
                cmds.push(AppCommand::SetWsMitm(state.mitm_enabled));
            }
        });
    });

    if state.connections.is_empty() {
        ui.add_space(4.0);
        ui.weak("No WebSocket connections captured");
        return;
    }

    let sources = super::sources(&state.connections);
    if sources.len() > 1 {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut state.source_filter, "ALL".to_string(), "All sources");
            for source in &sources {
                ui.selectable_value(&mut state.source_filter, source.clone(), source);
            }
        });
    }
    ui.add_space(4.0);

    let rows: Vec<usize> = state
        .connections
        .iter()
        .enumerate()
        .filter(|(_, c)| state.source_filter == "ALL" || super::source_label(c) == state.source_filter)
        .map(|(i, _)| i)
        .collect();

    let row_height = egui::TextStyle::Body.resolve(ui.style()).size + 6.0;
    let mut new_selection = None;
    let mut action = None;

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .sense(egui::Sense::click())
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(45.0))
        .column(Column::remainder())
        .column(Column::exact(70.0))
        .column(Column::exact(50.0))
        .header(22.0, |mut header| {
            header.col(|ui| { ui.strong("#"); });
            header.col(|ui| { ui.strong("URL"); });
            header.col(|ui| { ui.strong("Status"); });
            header.col(|ui| { ui.strong("Msgs"); });
        })
        .body(|body| {
            body.rows(row_height, rows.len(), |mut row| {
                let conn = &state.connections[rows[row.index()]];
                let is_selected = state.selected_id == Some(conn.connection_id);
                row.set_selected(is_selected);
                row.col(|ui| { ui.label(conn.connection_id.to_string()); });
                row.col(|ui| { ui.add(egui::Label::new(&conn.url).truncate()); });
                row.col(|ui| { status_badge(ui, &conn.status); });
                row.col(|ui| { ui.label(conn.message_count.to_string()); });
                let response = row.response();
                if response.clicked() {
                    new_selection = Some(conn.connection_id);
                }
                response.context_menu(|ui| session_menu(ui, conn, &mut action));
            });
        });

    if let Some(id) = new_selection {
        cmds.push(AppCommand::SelectWsConnection(id));
    }
    if let Some(action) = action {
        cmds.push(action);
    }
}

fn session_menu(ui: &mut egui::Ui, conn: &WebSocketConn, action: &mut Option<AppCommand>) {
    let id = conn.connection_id;
    let open = is_open(&conn.status);
    if ui.button("📋 Copy URL").clicked() {
        ui.ctx().copy_text(conn.url.clone());
        ui.close();
    }
    if conn.is_client_session && open && ui.button("⏹ Disconnect").clicked() {
        *action = Some(AppCommand::DisconnectWs(id));
        ui.close();
    }
    ui.separator();
    if ui.button("Clear frames").clicked() {
        *action = Some(AppCommand::ClearWsSession(id));
        ui.close();
    }
    let delete = ui
        .add_enabled(!open, egui::Button::new(RichText::new("🗑 Delete").color(theme::color(ui, Tone::Danger))))
        .on_disabled_hover_text("Session is still open");
    if delete.clicked() {
        *action = Some(AppCommand::DeleteWsSession(id));
        ui.close();
    }
}

fn frames_and_detail(ui: &mut egui::Ui, state: &mut WebSocketsState, cmds: &mut Commands) {
    let Some(selected) = state.selected_id else {
        ui.weak("Select a session to view its frames");
        return;
    };
    let live = state.connections.iter().find(|c| c.connection_id == selected && is_open(&c.status)).cloned();

    let Some(conn) = live else {
        frames_with_detail(ui, state, cmds);
        return;
    };
    let [mut top, mut bottom] = split::stacked(ui, "ws_composer_split", &mut state.prefs.composer_split);
    frames_with_detail(&mut top, state, cmds);
    super::composer::show(&mut bottom, &mut state.composer, &mut state.prefs.templates, &conn, cmds);
}

fn frames_with_detail(ui: &mut egui::Ui, state: &mut WebSocketsState, cmds: &mut Commands) {
    let [mut top, mut bottom] = split::stacked(ui, "ws_detail_split", &mut state.prefs.detail_split);
    frames_table(&mut top, state);
    frame_detail(&mut bottom, state, cmds);
}

fn frames_table(ui: &mut egui::Ui, state: &mut WebSocketsState) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.direction_filter, "ALL".to_string(), "All");
        ui.selectable_value(&mut state.direction_filter, "IN".to_string(), "▼ Server");
        ui.selectable_value(&mut state.direction_filter, "OUT".to_string(), "▲ Client");
        ui.separator();
        ui.add(egui::TextEdit::singleline(&mut state.frame_search).hint_text("Search frames").desired_width(140.0));
        if !state.frame_search.is_empty() && ui.small_button("✕").on_hover_text("Clear search").clicked() {
            state.frame_search.clear();
        }
    });
    ui.add_space(2.0);

    let query = state.frame_search.to_lowercase();
    let rows: Vec<usize> = state
        .messages
        .iter()
        .enumerate()
        .filter(|(_, m)| direction_matches(&state.direction_filter, &m.direction))
        .filter(|(_, m)| query.is_empty() || m.payload.to_lowercase().contains(&query))
        .map(|(i, _)| i)
        .collect();

    if rows.is_empty() {
        ui.weak("No frames in this session");
        return;
    }

    let row_height = egui::TextStyle::Body.resolve(ui.style()).size + 6.0;
    let mut new_selection = None;

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .sense(egui::Sense::click())
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(60.0))
        .column(Column::exact(55.0))
        .column(Column::exact(55.0))
        .column(Column::exact(75.0))
        .column(Column::remainder())
        .header(22.0, |mut header| {
            header.col(|ui| { ui.strong("Dir"); });
            header.col(|ui| { ui.strong("Type"); });
            header.col(|ui| { ui.strong("Size"); });
            header.col(|ui| { ui.strong("Time"); });
            header.col(|ui| { ui.strong("Payload"); });
        })
        .body(|body| {
            body.rows(row_height, rows.len(), |mut row| {
                let msg = &state.messages[rows[row.index()]];
                let is_selected = state.selected_msg == Some(msg.id);
                row.set_selected(is_selected);
                row.col(|ui| { direction_badge(ui, &msg.direction); });
                row.col(|ui| { ui.label(msg.msg_type.to_uppercase()); });
                row.col(|ui| { ui.label(format_bytes(msg.length as u64)); });
                row.col(|ui| { ui.label(clock(msg.timestamp)); });
                row.col(|ui| {
                    ui.add(egui::Label::new(RichText::new(preview(&msg.payload)).monospace()).truncate());
                });
                if row.response().clicked() {
                    new_selection = Some(msg.id);
                }
            });
        });

    if let Some(id) = new_selection {
        state.selected_msg = Some(id);
    }
}

fn frame_detail(ui: &mut egui::Ui, state: &mut WebSocketsState, cmds: &mut Commands) {
    let Some(id) = state.selected_msg else {
        ui.weak("Select a frame");
        return;
    };
    let Some(msg) = state.messages.iter().find(|m| m.id == id) else {
        ui.weak("Select a frame");
        return;
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("#{}", msg.id)).strong());
        direction_badge(ui, &msg.direction);
        ui.weak(clock(msg.timestamp));
        ui.weak(format_bytes(msg.length as u64));
        ui.weak(msg.msg_type.to_uppercase());
        if msg.is_injected {
            badge(ui, "INJECTED", Tone::Warn);
        }
    });
    ui.separator();

    let data = crate::widgets::message_pane::PaneData {
        cache_key: msg.id as u64,
        title: "",
        headers: &[],
        body: &msg.payload,
        content_type: content_type(&msg.msg_type),
        params: None,
        cookies: None,
        form_parser: None,
    };
    if let Some(save) = crate::widgets::message_pane::show(ui, "ws_frame_pane", &mut state.detail_pane, &data) {
        cmds.push(AppCommand::SaveBody(save));
    }
}

fn content_type(msg_type: &str) -> &'static str {
    match msg_type {
        "json" => "application/json",
        "binary" => "application/octet-stream",
        _ => "text/plain",
    }
}

fn direction_matches(filter: &str, direction: &str) -> bool {
    match filter {
        "IN" => is_incoming(direction),
        "OUT" => !is_incoming(direction),
        _ => true,
    }
}

fn is_incoming(direction: &str) -> bool {
    direction.eq_ignore_ascii_case("server")
        || direction.eq_ignore_ascii_case("incoming")
        || direction.eq_ignore_ascii_case("to_client")
}

fn preview(payload: &str) -> String {
    let one_line = payload.replace(['\n', '\r'], " ");
    if one_line.chars().count() > 80 {
        let head: String = one_line.chars().take(77).collect();
        format!("{head}…")
    } else {
        one_line
    }
}

fn clock(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|at| at.with_timezone(&chrono::Local).format("%H:%M:%S").to_string())
        .unwrap_or_default()
}

fn is_open(status: &str) -> bool {
    status.eq_ignore_ascii_case("connected") || status.eq_ignore_ascii_case("open")
}

fn status_badge(ui: &mut egui::Ui, status: &str) {
    badge(ui, &status.to_uppercase(), if is_open(status) { Tone::Success } else { Tone::Neutral });
}

fn direction_badge(ui: &mut egui::Ui, direction: &str) {
    if is_incoming(direction) {
        badge(ui, "▼ SVR", Tone::Success);
    } else {
        badge(ui, "▲ CLI", Tone::Info);
    }
}
