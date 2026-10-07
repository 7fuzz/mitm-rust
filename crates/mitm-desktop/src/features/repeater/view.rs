use eframe::egui::{self, RichText};
use crate::command::{AppCommand, Commands};
use crate::features::request_editor::response_panel::{self, header_pairs, RequestCopy, ResponseData};
use crate::features::request_editor::run_list::{self, Run, RunAction};
use crate::features::request_editor::url_params::build_url;
use crate::widgets::split;
use super::{builder, sidebar, RepeaterState};

pub fn show(ui: &mut egui::Ui, state: &mut RepeaterState, cmds: &mut Commands) {
    let now = ui.input(|i| i.time);

    egui::Panel::left("repeater_sidebar")
        .resizable(true)
        .default_size(230.0)
        .size_range(150.0..=420.0)
        .show(ui, |ui| sidebar::show(ui, state, cmds));

    let Some(active_id) = state.active_id.clone() else {
        egui::CentralPanel::no_frame().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new("No repeater request selected").weak().italics());
                if ui.button("➕ Create New Request").clicked() {
                    cmds.push(AppCommand::RepeaterNew);
                }
            });
        });
        return;
    };

    if state.history_open {
        egui::Panel::right("repeater_history")
            .resizable(true)
            .default_size(288.0)
            .size_range(200.0..=520.0)
            .show(ui, |ui| history_drawer(ui, state, &active_id, now));
    }

    egui::CentralPanel::no_frame().show(ui, |ui| {
        let mut fraction = state.split;
        let [mut request_ui, mut response_ui] = split::regions(ui, "repeater_split", &mut fraction);
        state.split = fraction;
        builder::show(&mut request_ui, state, cmds, now);
        response(&mut response_ui, state, &active_id);
    });
}

fn response(ui: &mut egui::Ui, state: &mut RepeaterState, tab_id: &str) {
    let Some(tab) = state.tabs.iter().find(|t| t.id == tab_id) else { return };
    let request = RequestCopy {
        method: tab.method.clone(),
        url: build_url(&tab.url, &tab.params),
        headers: tab
            .headers
            .iter()
            .filter(|h| h.enabled && !h.key.trim().is_empty())
            .map(|h| (h.key.clone(), h.value.clone()))
            .collect(),
        body: tab.body_content.clone().unwrap_or_default(),
    };

    let shown = state.responses.get(tab_id);
    let exchange = shown
        .and_then(|s| state.history.get(tab_id)?.iter().find(|run| run.id == s.history_id))
        .map(|run| run.exchange());
    let data = shown.map(|s| ResponseData {
        cache_key: s.history_id as u64,
        status_code: s.status_code,
        status_text: &s.status_text,
        duration_ms: s.duration_ms,
        size: s.size,
        headers: header_pairs(&s.headers),
        body: &s.body,
    });
    let executing = state.executing.contains(tab_id);

    response_panel::show(ui, "repeater_response", &mut state.response_pane, data, Some(&request), exchange.as_ref(), |ui| {
        if executing {
            ui.spinner();
        }
    });
}

fn history_drawer(ui: &mut egui::Ui, state: &mut RepeaterState, tab_id: &str, now: f64) {
    let runs = state.history.get(tab_id).map(Vec::as_slice).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("🕘 Execution History ({})", runs.len())).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("✖").on_hover_text("Close").clicked() {
                state.history_open = false;
            }
        });
    });
    ui.separator();

    let selected = state.responses.get(tab_id).map(|s| s.history_id);
    match run_list::show(ui, "repeater_runs", runs, selected, true) {
        Some(RunAction::Select(run_id)) => state.show_run(tab_id, run_id),
        Some(RunAction::Load(run_id)) => state.load_run(tab_id, run_id, now),
        None => {}
    }
}
