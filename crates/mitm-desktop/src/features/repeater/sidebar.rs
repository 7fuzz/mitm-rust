use eframe::egui::{self, RichText};
use mitm_core::repeater::RepeaterTab;
use crate::command::{AppCommand, Commands};
use crate::widgets::pills::{render_method_pill, render_status_pill, AMBER, ROSE};
use super::RepeaterState;

pub fn show(ui: &mut egui::Ui, state: &mut RepeaterState, cmds: &mut Commands) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Requests ({})", state.tabs.len())).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("➕ New").on_hover_text("Create a new request").clicked() {
                cmds.push(AppCommand::RepeaterNew);
            }
        });
    });
    ui.horizontal(|ui| {
        let width = ui.available_width() - if state.search.is_empty() { 0.0 } else { 24.0 };
        ui.add(egui::TextEdit::singleline(&mut state.search).hint_text("🔍 Filter requests...").desired_width(width));
        if !state.search.is_empty() && ui.small_button("✖").clicked() {
            state.search.clear();
        }
    });
    ui.separator();

    let term = state.search.trim().to_lowercase();
    let visible: Vec<&RepeaterTab> = state
        .tabs
        .iter()
        .filter(|t| term.is_empty() || format!("{} {}", t.method, t.url).to_lowercase().contains(&term))
        .collect();

    if visible.is_empty() {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            let text = if term.is_empty() { "No requests created" } else { "No matching requests" };
            ui.label(RichText::new(text).weak().italics());
        });
        return;
    }

    egui::ScrollArea::vertical().id_salt("repeater_sidebar_list").auto_shrink([false, false]).show(ui, |ui| {
        for tab in visible {
            let active = state.active_id.as_deref() == Some(tab.id.as_str());
            if let Some(cmd) = card(ui, tab, active) {
                cmds.push(cmd);
            }
        }
    });
}

fn card(ui: &mut egui::Ui, tab: &RepeaterTab, active: bool) -> Option<AppCommand> {
    let (host, path) = split_url(&tab.url);
    let mut cmd = None;

    let mut frame = egui::Frame::NONE.inner_margin(egui::Margin::symmetric(8, 6));
    if active {
        frame = frame
            .fill(ui.visuals().selection.bg_fill.gamma_multiply(0.3))
            .stroke(egui::Stroke::new(1.0, ui.visuals().selection.bg_fill));
    }
    let response = ui
        .push_id(&tab.id, |ui| {
            frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    render_method_pill(ui, &tab.method);
                    let width = ui.available_width() - 24.0;
                    ui.add_sized(
                        [width, 18.0],
                        egui::Label::new(RichText::new(host).strong().monospace()).truncate().selectable(false),
                    );
                    if ui.small_button(RichText::new("✖").color(ROSE)).on_hover_text("Delete request").clicked() {
                        cmd = Some(AppCommand::RepeaterDelete(tab.id.clone()));
                    }
                });
                ui.add(egui::Label::new(RichText::new(path).weak().monospace().size(11.0)).truncate().selectable(false));
                ui.horizontal(|ui| {
                    let hits = tab.execution_count;
                    let noun = if hits == 1 { "hit" } else { "hits" };
                    ui.label(RichText::new("⚡").color(AMBER).size(10.0));
                    ui.label(RichText::new(format!("{} {}", hits, noun)).weak().size(10.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match tab.last_status_code {
                        Some(code) => {
                            if let Some(ms) = tab.last_duration_ms {
                                ui.label(RichText::new(format!("{}ms", ms)).weak().size(10.0));
                            }
                            render_status_pill(ui, code);
                        }
                        None => {
                            ui.label(RichText::new("Unsent").weak().italics().size(10.0));
                        }
                    });
                });
            })
        })
        .inner
        .response
        .interact(egui::Sense::click());

    if cmd.is_none() && response.clicked() {
        cmd = Some(AppCommand::RepeaterSelect(tab.id.clone()));
    }
    ui.separator();
    cmd
}

fn split_url(url: &str) -> (String, String) {
    let url = url.trim();
    if url.is_empty() {
        return ("https://localhost".to_string(), "/".to_string());
    }
    let (scheme, rest) = url.split_once("://").unwrap_or(("https", url));
    let (host, path) = rest.find(['/', '?']).map_or((rest, "/"), |i| (&rest[..i], &rest[i..]));
    (format!("{}://{}", scheme, host), path.to_string())
}
