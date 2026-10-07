use eframe::egui::{self, RichText};
use crate::command::{AppCommand, Commands};
use crate::features::request_editor::response_panel::{self, header_pairs, ResponseData};
use crate::features::request_editor::run_list::{self, local_time, Run, RunAction};
use crate::widgets::pills::{method_text, AMBER, ROSE};
use crate::widgets::split;
use super::{editor, extract_modal, tree, CollectionsCmd, CollectionsState};

pub fn show(ui: &mut egui::Ui, state: &mut CollectionsState, cmds: &mut Commands) {
    let now = ui.input(|i| i.time);

    egui::Panel::left("collections_sidebar")
        .resizable(true)
        .default_size(240.0)
        .size_range(160.0..=480.0)
        .show(ui, |ui| tree::show(ui, state, cmds));

    egui::CentralPanel::no_frame().show(ui, |ui| {
        tab_strip(ui, state, cmds);
        ui.separator();

        let Some(active_id) = state.active_id.clone() else {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new("No collection request tab open").weak().italics());
            });
            return;
        };

        if state.prefs.history_open {
            egui::Panel::right("collections_history")
                .resizable(true)
                .default_size(288.0)
                .size_range(200.0..=520.0)
                .show(ui, |ui| history_drawer(ui, state, &active_id, cmds));
        }

        egui::CentralPanel::no_frame().show(ui, |ui| {
            let [mut request_ui, mut response_ui] = split::side_by_side(ui, "collections_split", &mut state.prefs.split);
            editor::show(&mut request_ui, state, cmds, now);
            response(&mut response_ui, state, &active_id);
        });
    });

    extract_modal::show(ui.ctx(), state, cmds);
}

fn tab_strip(ui: &mut egui::Ui, state: &CollectionsState, cmds: &mut Commands) {
    egui::ScrollArea::horizontal()
        .id_salt("collections_tabs")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if state.open.is_empty() {
                    ui.weak("Open a request from the tree");
                }
                for request in &state.open {
                    let active = state.active_id.as_deref() == Some(request.id.as_str());
                    let style = ui.style().clone();
                    let mut text = egui::text::LayoutJob::default();
                    method_text(&request.method).append_to(&mut text, &style, egui::FontSelection::Default, egui::Align::Center);
                    RichText::new(format!(" {}", request.name)).append_to(&mut text, &style, egui::FontSelection::Default, egui::Align::Center);

                    let response = ui.add(egui::Button::selectable(active, text));
                    if response.clicked() {
                        cmds.push(AppCommand::Collections(CollectionsCmd::Activate(request.id.clone())));
                    }
                    let close = ui.small_button("✖").on_hover_text("Close tab").clicked();
                    if close || response.middle_clicked() {
                        cmds.push(AppCommand::Collections(CollectionsCmd::Close(request.id.clone())));
                    }
                    ui.add_space(6.0);
                }
            });
        });
}

fn response(ui: &mut egui::Ui, state: &mut CollectionsState, request_id: &str) {
    let runs = state.runs.get(request_id).map(Vec::as_slice).unwrap_or_default();
    let latest = state.latest.get(request_id);

    if state.executing.contains(request_id) {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.spinner();
            ui.label(RichText::new("Executing request...").weak());
        });
        return;
    }

    let run_index = state
        .selected_run
        .get(request_id)
        .and_then(|id| runs.iter().position(|r| r.id == *id))
        .unwrap_or(0);
    let run = runs.get(run_index);

    let data = match (run, latest) {
        (Some(run), _) => Some(ResponseData {
            cache_key: run.id as u64,
            status_code: run.status_code,
            status_text: latest.filter(|l| l.history_id == run.id).map_or("", |l| l.status_text.as_str()),
            duration_ms: run.duration_ms,
            size: run.response_body().len() as u64,
            headers: header_pairs(&run.response_headers),
            body: run.response_body(),
        }),
        (None, Some(latest)) => Some(ResponseData {
            cache_key: latest.history_id as u64,
            status_code: latest.status_code,
            status_text: &latest.status_text,
            duration_ms: latest.duration_ms,
            size: latest.response_size,
            headers: header_pairs(&latest.response_headers),
            body: &latest.response_body,
        }),
        (None, None) => None,
    };
    let exchange = run.map(Run::exchange);
    let older_run = run.filter(|_| run_index > 0).map(|r| (runs.len() - run_index, local_time(r.executed_at_ms)));
    let mut show_latest = false;

    response_panel::show(ui, "collections_response", &mut state.response_pane, data, None, exchange.as_ref(), |ui| {
        if let Some((number, time)) = &older_run {
            if ui.small_button("Latest").on_hover_text("Show the latest run").clicked() {
                show_latest = true;
            }
            ui.label(RichText::new(format!("🕘 Run {} · {}", number, time)).color(AMBER).size(11.0));
        }
    });
    if show_latest {
        state.selected_run.remove(request_id);
    }
}

fn history_drawer(ui: &mut egui::Ui, state: &mut CollectionsState, request_id: &str, cmds: &mut Commands) {
    let runs = state.runs.get(request_id).map(Vec::as_slice).unwrap_or_default();
    let confirm_id = ui.id().with(("clear_runs", request_id));
    let mut confirming: bool = ui.data_mut(|d| d.get_temp(confirm_id)).unwrap_or(false);

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("🕘 Run History ({})", runs.len())).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("✖").on_hover_text("Close run history").clicked() {
                state.prefs.history_open = false;
            }
            if !runs.is_empty() {
                let label = if confirming { RichText::new("🗑 Clear all?").color(ROSE) } else { RichText::new("🗑") };
                let button = ui.small_button(label).on_hover_text("Delete all recorded runs of this request");
                if button.clicked() {
                    if confirming {
                        cmds.push(AppCommand::Collections(CollectionsCmd::ClearRuns(request_id.to_string())));
                    }
                    confirming = !confirming;
                } else if confirming && !button.hovered() {
                    confirming = false;
                }
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(confirm_id, confirming));
    ui.separator();

    let selected = state.selected_run.get(request_id).copied().or_else(|| runs.first().map(|r| r.id));
    if let Some(RunAction::Select(run_id)) = run_list::show(ui, "collections_runs", runs, selected, false) {
        let newest = runs.first().is_some_and(|r| r.id == run_id);
        if newest {
            state.selected_run.remove(request_id);
        } else {
            state.selected_run.insert(request_id.to_string(), run_id);
        }
    }
}
