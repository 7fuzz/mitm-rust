use eframe::egui::{self, RichText};
use mitm_core::repeater::RepeaterTab;
use crate::command::{AppCommand, Commands};
use crate::theme::{self, Tone};
use crate::widgets::pills::{method_text, status_text};
use crate::widgets::search::search_field;
use super::RepeaterState;

pub fn show(ui: &mut egui::Ui, state: &mut RepeaterState, cmds: &mut Commands) {
    egui::Sides::new().show(
        ui,
        |ui| {
            ui.strong("Requests");
            ui.weak(state.tabs.len().to_string());
        },
        |ui| {
            if ui.button("➕").on_hover_text("New request").clicked() {
                cmds.push(AppCommand::RepeaterNew);
            }
        },
    );
    search_field(ui, &mut state.search, "Filter");
    ui.separator();

    let term = state.search.trim().to_lowercase();
    let visible: Vec<&RepeaterTab> = state
        .tabs
        .iter()
        .filter(|t| term.is_empty() || format!("{} {}", t.method, t.url).to_lowercase().contains(&term))
        .collect();

    if visible.is_empty() {
        ui.weak(if term.is_empty() { "No requests yet" } else { "No matches" });
        return;
    }

    egui::ScrollArea::vertical().id_salt("repeater_sidebar_list").auto_shrink([false, false]).show(ui, |ui| {
        for tab in visible {
            let active = state.active_id.as_deref() == Some(tab.id.as_str());
            if let Some(cmd) = row(ui, tab, active) {
                cmds.push(cmd);
            }
        }
    });
}

fn row(ui: &mut egui::Ui, tab: &RepeaterTab, active: bool) -> Option<AppCommand> {
    let mut left = egui::text::LayoutJob::default();
    method_text(ui.visuals(), &tab.method).append_to(&mut left, ui.style(), egui::FontSelection::Default, egui::Align::Center);
    RichText::new(format!("  {}", url_path(&tab.url)))
        .monospace()
        .append_to(&mut left, ui.style(), egui::FontSelection::Default, egui::Align::Center);

    let right = match tab.last_status_code {
        Some(code) => status_text(ui.visuals(), code),
        None => RichText::new("·").weak(),
    };

    let response = ui
        .add(
            egui::Button::selectable(active, left)
                .right_text(right)
                .truncate()
                .min_size(egui::vec2(ui.available_width(), 24.0)),
        )
        .on_hover_ui(|ui| {
            ui.monospace(&tab.url);
            match (tab.last_status_code, tab.last_duration_ms) {
                (Some(code), Some(ms)) => ui.weak(format!("Last: {} in {}ms · sent {}×", code, ms, tab.execution_count)),
                _ => ui.weak("Not sent yet"),
            };
        });

    let mut cmd = None;
    response.context_menu(|ui| {
        if ui.button(RichText::new("Delete").color(theme::color(ui, Tone::Danger))).clicked() {
            cmd = Some(AppCommand::RepeaterDelete(tab.id.clone()));
            ui.close();
        }
    });
    if response.clicked() {
        cmd = Some(AppCommand::RepeaterSelect(tab.id.clone()));
    }
    cmd
}

fn url_path(url: &str) -> &str {
    let rest = url.trim().split_once("://").map_or(url.trim(), |(_, rest)| rest);
    rest.find(['/', '?']).map_or(rest, |i| &rest[i..])
}
