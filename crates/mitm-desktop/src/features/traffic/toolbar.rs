use eframe::egui::containers::menu::{MenuButton, MenuConfig};
use eframe::egui::{self, PopupCloseBehavior, RichText};
use mitm_core::state::ListenerConfig;
use crate::app::command::{AppCommand, Commands};
use crate::widgets::confirm::confirm_button;
use crate::widgets::tri_state::tri_state_button;
use super::{TrafficState, FLAGS, METHODS, STATUS_GROUPS};

const LIMIT_PRESETS: [u32; 7] = [100, 250, 500, 1000, 2500, 5000, 10000];

pub fn show(
    ui: &mut egui::Ui,
    state: &mut TrafficState,
    listeners: &[ListenerConfig],
    visible_count: usize,
    cmds: &mut Commands,
) {
    ui.horizontal(|ui| {
        search_box(ui, &mut state.prefs.search_query);
        ui.toggle_value(&mut state.prefs.search_headers, RichText::new("H").monospace().strong())
            .on_hover_text("Also search request and response headers");
        filters_toggle(ui, state);
        request_count(ui, state, visible_count);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let confirm_id = ui.id().with("clear_traffic");
            if confirm_button(ui, confirm_id, RichText::new("🗑"), "🗑 Clear all?", "Clear all traffic logs") {
                cmds.push(AppCommand::ClearTraffic);
            }
            if ui.button("📤").on_hover_text("Export as HAR").clicked() {
                cmds.push(AppCommand::OpenHarExport);
            }
            limit_menu(ui, state, cmds);
        });
    });

    if state.prefs.filters_open {
        filter_row(ui, state, listeners);
    }
}

fn search_box(ui: &mut egui::Ui, query: &mut String) {
    let edit = ui.add(
        egui::TextEdit::singleline(query)
            .hint_text("🔍 Search host, path, status...")
            .desired_width(280.0),
    );
    crate::widgets::search::claim_focus(&edit);
    if !query.is_empty() && ui.small_button("✖").clicked() {
        query.clear();
    }
}

fn filters_toggle(ui: &mut egui::Ui, state: &mut TrafficState) {
    let active = state.active_filter_count();
    let arrow = if state.prefs.filters_open { "⏶" } else { "⏷" };
    let label = match active {
        0 => format!("Filters {}", arrow),
        n => format!("Filters ({}) {}", n, arrow),
    };
    if ui.add(egui::Button::new(label).selected(state.prefs.filters_open || active > 0)).clicked() {
        state.prefs.filters_open = !state.prefs.filters_open;
    }
}

fn request_count(ui: &mut egui::Ui, state: &TrafficState, visible_count: usize) {
    let filtering = state.active_filter_count() > 0 || !state.prefs.search_query.trim().is_empty();
    let noun = match (filtering, visible_count) {
        (true, _) => "matching",
        (false, 1) => "request",
        (false, _) => "requests",
    };
    ui.label(RichText::new(visible_count.to_string()).strong().monospace());
    ui.label(RichText::new(noun).weak().monospace());
}

fn limit_menu(ui: &mut egui::Ui, state: &mut TrafficState, cmds: &mut Commands) {
    let current = &state.history_limit;
    let title = if current.limiter_enabled {
        format!("Limit: {} logs ⏷", current.max_rows)
    } else {
        "Limit: Off ⏷".to_string()
    };

    // Plain menus close on any click inside, which would shut this one when toggling the checkbox.
    let config = MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside);
    MenuButton::new(title).config(config).ui(ui, |ui| {
        ui.set_min_width(240.0);
        let draft = &mut state.limit_draft;

        ui.checkbox(&mut draft.limiter_enabled, "Limit max capacity")
            .on_hover_text("Delete oldest logs when full");

        if draft.limiter_enabled {
            ui.add(egui::DragValue::new(&mut draft.max_rows).range(10..=50_000).suffix(" rows"));
            ui.horizontal_wrapped(|ui| {
                for preset in LIMIT_PRESETS {
                    if ui.selectable_label(draft.max_rows == preset, preset.to_string()).clicked() {
                        draft.max_rows = preset;
                    }
                }
            });
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Apply & Delete Old").clicked() {
                cmds.push(AppCommand::SetHistoryLimit(draft.clone()));
                ui.close();
            }
            if ui.button("Cancel").clicked() {
                *draft = state.history_limit.clone();
                ui.close();
            }
        });
    });
}

fn filter_row(ui: &mut egui::Ui, state: &mut TrafficState, listeners: &[ListenerConfig]) {
    ui.horizontal_wrapped(|ui| {
        group_label(ui, "METHOD");
        for m in METHODS {
            tri_state_button(ui, m, state.prefs.method_filters.entry(m.to_string()).or_default());
        }

        ui.separator();
        group_label(ui, "STATUS");
        for st in STATUS_GROUPS {
            tri_state_button(ui, st, state.prefs.status_filters.entry(st.to_string()).or_default());
        }

        ui.separator();
        group_label(ui, "FLAGS");
        for (key, label) in FLAGS {
            tri_state_button(ui, label, state.prefs.flag_filters.entry(key.to_string()).or_default());
        }

        if listeners.len() > 1 {
            ui.separator();
            group_label(ui, "SOURCE");
            source_combo(ui, &mut state.prefs.source_filter, listeners);
        }

        if state.active_filter_count() > 0 && ui.link("Reset filters").clicked() {
            state.reset_filters();
        }
    });
}

fn group_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).weak().small().strong());
}

fn source_combo(ui: &mut egui::Ui, filter: &mut Option<String>, listeners: &[ListenerConfig]) {
    let selected = filter.clone().unwrap_or_else(|| "All Sources".to_string());
    egui::ComboBox::from_id_salt("traffic_source_filter")
        .selected_text(selected)
        .show_ui(ui, |ui| {
            ui.selectable_value(filter, None, "All Sources");
            for l in listeners {
                ui.selectable_value(filter, Some(l.label.clone()), format!("{} ({})", l.label, l.address));
            }
        });
}
