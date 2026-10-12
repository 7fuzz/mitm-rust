use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::state::WebSocketConn;
use crate::app::command::{AppCommand, Commands};
use crate::util::format::format_bytes;
use crate::ui::theme::{self, Tone};
use crate::widgets::pills::badge;
use crate::widgets::split;
use super::{Cmd, WebSocketsState};
use crate::ui::icons;
use crate::widgets::button::icon_button;
use crate::widgets::search::search_field_sized;
use crate::widgets::section::section_with;

pub fn show(ui: &mut egui::Ui, state: &mut WebSocketsState, cmds: &mut Commands) {
    super::connect::show(ui.ctx(), &mut state.connect, cmds);
    let [mut left, mut right] = split::side_by_side(ui, "ws_list_split", &mut state.prefs.list_split);
    sessions_list(&mut left, state, cmds);
    frames_and_detail(&mut right, state, cmds);
}

fn sessions_list(ui: &mut egui::Ui, state: &mut WebSocketsState, cmds: &mut Commands) {
    section_with(ui, "Sessions", |ui| {
        if icon_button(ui, icons::PLUS, "Connect a WebSocket").clicked() {
            state.connect.open();
        }
        let mitm = ui
            .toggle_value(&mut state.mitm_enabled, "MITM")
            .on_hover_text("Capture proxied WebSocket frames (new connections)");
        if mitm.changed() {
            cmds.push(Cmd::SetMitm(state.mitm_enabled));
        }
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
        .striped(false)
        .resizable(true)
        .sense(egui::Sense::click())
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(45.0))
        .column(Column::remainder())
        .column(Column::exact(70.0))
        .column(Column::exact(50.0))
        .header(22.0, |mut header| {
            for title in ["#", "URL", "Status", "Frames"] {
                header.col(|ui| header_label(ui, title));
            }
        })
        .body(|body| {
            body.rows(row_height, rows.len(), |mut row| {
                let conn = &state.connections[rows[row.index()]];
                let is_selected = state.selected_id == Some(conn.connection_id);
                row.set_selected(is_selected);
                row.col(|ui| { ui.label(RichText::new(conn.connection_id.to_string()).monospace().weak()); });
                row.col(|ui| { ui.add(egui::Label::new(RichText::new(&conn.url).monospace()).truncate()); });
                row.col(|ui| { status_badge(ui, &conn.status); });
                row.col(|ui| { ui.label(RichText::new(conn.message_count.to_string()).monospace().weak()); });
                let response = row.response();
                if response.clicked() {
                    new_selection = Some(conn.connection_id);
                }
                response.context_menu(|ui| session_menu(ui, conn, &mut action));
            });
        });

    if let Some(id) = new_selection {
        cmds.push(Cmd::Select(id));
    }
    if let Some(action) = action {
        cmds.push(action);
    }
}

fn session_menu(ui: &mut egui::Ui, conn: &WebSocketConn, action: &mut Option<Cmd>) {
    let id = conn.connection_id;
    let open = is_open(&conn.status);
    if ui.button("Copy URL").clicked() {
        ui.ctx().copy_text(conn.url.clone());
        ui.close();
    }
    if conn.is_client_session && open && ui.button("Disconnect").clicked() {
        *action = Some(Cmd::Disconnect(id));
        ui.close();
    }
    ui.separator();
    if ui.button("Clear frames").clicked() {
        *action = Some(Cmd::ClearSession(id));
        ui.close();
    }
    let delete = ui
        .add_enabled(!open, egui::Button::new(RichText::new("Delete").color(theme::color(ui, Tone::Danger))))
        .on_disabled_hover_text("Session is still open");
    if delete.clicked() {
        *action = Some(Cmd::DeleteSession(id));
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
        ui.selectable_value(&mut state.direction_filter, "IN".to_string(), "From server");
        ui.selectable_value(&mut state.direction_filter, "OUT".to_string(), "From client");
        ui.add_space(8.0);
        search_field_sized(ui, &mut state.frame_search, "Search frames", 200.0);
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
        .striped(false)
        .resizable(true)
        .sense(egui::Sense::click())
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(28.0))
        .column(Column::exact(55.0))
        .column(Column::exact(55.0))
        .column(Column::exact(75.0))
        .column(Column::remainder())
        .header(22.0, |mut header| {
            for title in ["", "Type", "Size", "Time", "Payload"] {
                header.col(|ui| header_label(ui, title));
            }
        })
        .body(|body| {
            body.rows(row_height, rows.len(), |mut row| {
                let msg = &state.messages[rows[row.index()]];
                let is_selected = state.selected_msg == Some(msg.id);
                row.set_selected(is_selected);
                row.col(|ui| { direction_badge(ui, &msg.direction); });
                row.col(|ui| { ui.label(RichText::new(msg.msg_type.to_uppercase()).small().weak()); });
                row.col(|ui| { ui.label(RichText::new(format_bytes(msg.length as u64)).monospace().weak()); });
                row.col(|ui| { ui.label(RichText::new(clock(msg.timestamp)).monospace().weak()); });
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
        direction_badge(ui, &msg.direction);
        ui.label(RichText::new(format!("#{}", msg.id)).monospace().strong());
        let stats = format!("{} · {} · {}", clock(msg.timestamp), format_bytes(msg.length as u64), msg.msg_type.to_uppercase());
        ui.label(RichText::new(stats).monospace().weak());
        if msg.is_injected {
            badge(ui, "INJECTED", Tone::Warn);
        }
    });
    ui.add_space(4.0);

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
    let (icon, tone, hover) = if is_incoming(direction) {
        (icons::ARROW_DOWN_LEFT, Tone::Success, "From server")
    } else {
        (icons::ARROW_UP_RIGHT, Tone::Info, "From client")
    };
    ui.label(RichText::new(icon).color(theme::color(ui, tone))).on_hover_text(hover);
}

fn header_label(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).small().weak());
}
