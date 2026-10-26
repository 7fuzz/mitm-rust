use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::proxy::ContainerRow;
use crate::app::command::Commands;
use crate::ui::icons;
use crate::ui::theme::Tone;
use crate::util::format::format_count;
use crate::widgets::confirm::confirm_button;
use crate::widgets::pills::badge;
use super::{Cmd, ContainersState};

pub fn show(ui: &mut egui::Ui, state: &mut ContainersState, cmds: &mut Commands) {
    state.visible = true;
    ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
    toolbar(ui, state, cmds);
    ui.add_space(4.0);
    if state.rows.is_empty() {
        ui.weak("No containers yet");
        return;
    }
    table(ui, &state.rows, cmds);
}

fn toolbar(ui: &mut egui::Ui, state: &ContainersState, cmds: &mut Commands) {
    let unregistered = state.unregistered().count();
    ui.horizontal(|ui| {
        ui.label(format!("{} registered", state.rows.len() - unregistered));
        if unregistered > 0 {
            ui.label(RichText::new(format!("{unregistered} unregistered")).weak());
            let delete = confirm_button(ui, ui.id().with("ct_clear_unregistered"), RichText::new(icons::TRASH), "Delete their flows?", "Delete the flows of every unregistered container");
            if delete {
                cmds.push(Cmd::DeleteUnregisteredFlows);
            }
        }
    });
}

fn table(ui: &mut egui::Ui, rows: &[ContainerRow], cmds: &mut Commands) {
    let row_height = egui::TextStyle::Body.resolve(ui.style()).size + 8.0;
    let now = now_unix();
    TableBuilder::new(ui)
        .striped(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(110.0))
        .column(Column::remainder().at_least(160.0))
        .column(Column::exact(70.0))
        .column(Column::exact(50.0))
        .column(Column::exact(80.0))
        .column(Column::exact(90.0))
        .column(Column::exact(64.0))
        .header(22.0, |mut header| {
            for title in ["Status", "Name", "Id", "Run", "Flows", "Last seen", ""] {
                header.col(|ui| {
                    ui.label(RichText::new(title).small().weak());
                });
            }
        })
        .body(|body| {
            body.rows(row_height, rows.len(), |mut row| {
                let container = &rows[row.index()];
                row.col(|ui| status(ui, container));
                row.col(|ui| {
                    ui.add(egui::Label::new(&container.name).truncate());
                });
                row.col(|ui| {
                    let id = if container.registered { container.id.as_str() } else { "" };
                    ui.add(egui::Label::new(RichText::new(id).monospace().weak()).truncate());
                });
                row.col(|ui| {
                    ui.label(RichText::new(if container.registered { container.run_id.to_string() } else { String::new() }).monospace().weak());
                });
                row.col(|ui| {
                    ui.label(RichText::new(format_count(container.flows as usize)).monospace());
                });
                row.col(|ui| {
                    ui.label(RichText::new(last_seen(container.last_seen, now)).monospace().weak());
                });
                row.col(|ui| actions(ui, container, cmds));
            });
        });
}

fn status(ui: &mut egui::Ui, container: &ContainerRow) {
    let (text, tone) = if container.registered { ("Registered", Tone::Success) } else { ("Unregistered", Tone::Warn) };
    badge(ui, text, tone);
}

fn actions(ui: &mut egui::Ui, container: &ContainerRow, cmds: &mut Commands) {
    if container.flows > 0 {
        let delete = confirm_button(ui, ui.id().with(("ct_flows", &container.id)), RichText::new(icons::TRASH), "Delete?", "Delete this container's flows");
        if delete {
            cmds.push(Cmd::DeleteFlows(container.id.clone()));
        }
    }
    if container.registered {
        let remove = confirm_button(ui, ui.id().with(("ct_unregister", &container.id)), RichText::new(icons::X), "Remove?", "Unregister; stored flows stay");
        if remove {
            cmds.push(Cmd::Unregister(container.id.clone()));
        }
    }
}

fn last_seen(unix: Option<i64>, now: i64) -> String {
    let Some(unix) = unix else { return "never".to_string() };
    match (now - unix).max(0) {
        s if s < 5 => "now".to_string(),
        s if s < 60 => format!("{s}s ago"),
        s if s < 3600 => format!("{}m ago", s / 60),
        s if s < 86400 => format!("{}h ago", s / 3600),
        s => format!("{}d ago", s / 86400),
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}
