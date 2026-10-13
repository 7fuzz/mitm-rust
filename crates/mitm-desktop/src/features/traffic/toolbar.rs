use eframe::egui::containers::menu::{MenuButton, MenuConfig};
use eframe::egui::{self, PopupCloseBehavior, RichText};
use mitm_core::proxy::listener::ListenerConfig;
use crate::app::command::Commands;
use crate::widgets::confirm::confirm_button;
use crate::widgets::tri_state::tri_state_button;
use super::{Cmd, TrafficState, FLAGS, METHODS, STATUS_GROUPS};
use crate::ui::icons;
use crate::ui::theme::Palette;
use crate::util::format::format_count;
use crate::widgets::button::icon_button;
use crate::widgets::search::search_field_sized;

const LIMIT_PRESETS: [u32; 7] = [100, 250, 500, 1000, 2500, 5000, 10000];

pub fn show(
    ui: &mut egui::Ui,
    state: &mut TrafficState,
    listeners: &[ListenerConfig],
    visible_count: usize,
    cmds: &mut Commands,
) {
    ui.horizontal(|ui| {
        search_field_sized(ui, &mut state.prefs.search_query, "Search host, path, status…", 300.0);
        header_search_toggle(ui, &mut state.prefs.search_headers);
        ui.add_space(8.0);
        filters_toggle(ui, state);
        ui.add_space(8.0);
        request_count(ui, state, visible_count);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let confirm_id = ui.id().with("clear_traffic");
            if confirm_button(ui, confirm_id, RichText::new(icons::TRASH), "Clear all?", "Clear all traffic logs") {
                cmds.push(Cmd::Clear);
            }
            if icon_button(ui, icons::EXPORT, "Export as HAR").clicked() {
                cmds.push(Cmd::OpenHarExport);
            }
            ui.add_space(8.0);
            limit_menu(ui, state, cmds);
        });
    });

    if state.prefs.filters_open {
        filter_row(ui, state, listeners);
    }
}

fn header_search_toggle(ui: &mut egui::Ui, on: &mut bool) {
    let visuals = ui.visuals();
    let color = if *on { visuals.selection.stroke.color } else { visuals.weak_text_color() };
    let button = egui::Button::new(RichText::new("H").family(egui::FontFamily::Monospace).color(color)).frame_when_inactive(false);
    if ui.add(button).on_hover_text("Also search request and response headers").clicked() {
        *on = !*on;
    }
}

fn filters_toggle(ui: &mut egui::Ui, state: &mut TrafficState) {
    let active = state.active_filter_count();
    let arrow = if state.prefs.filters_open { icons::CARET_UP } else { icons::CARET_DOWN };
    let caret = RichText::new(arrow).weak();
    let button = match active {
        0 => egui::Button::new(("Filters", caret)),
        n => egui::Button::new(("Filters", count_pill(ui, n), caret)),
    };
    if ui.add(button.frame_when_inactive(state.prefs.filters_open)).clicked() {
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
    ui.label(RichText::new(format_count(visible_count)).monospace());
    ui.label(RichText::new(noun).weak());
}

fn limit_menu(ui: &mut egui::Ui, state: &mut TrafficState, cmds: &mut Commands) {
    let current = &state.history_limit;
    let limit = if current.limiter_enabled { format!("Limit {}", format_count(current.max_rows as usize)) } else { "No limit".to_string() };
    let title = egui::Button::new((RichText::new(limit).weak(), RichText::new(icons::CARET_DOWN).weak())).frame_when_inactive(false);

    // Plain menus close on any click inside, which would shut this one when toggling the checkbox.
    let config = MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside);
    MenuButton::from_button(title).config(config).ui(ui, |ui| {
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
        let mut cap_bodies = draft.max_body_mb > 0;
        if ui.checkbox(&mut cap_bodies, "Skip large bodies").on_hover_text("Bodies over the size are stored as a short note; new traffic only").changed() {
            draft.max_body_mb = if cap_bodies { 10 } else { 0 };
        }
        if cap_bodies {
            ui.add(egui::DragValue::new(&mut draft.max_body_mb).range(1..=1024).prefix("Over ").suffix(" MB"));
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Apply & Delete Old").clicked() {
                cmds.push(Cmd::SetHistoryLimit(draft.clone()));
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
    let palette = Palette::of(ui.visuals());
    egui::Frame::new().fill(palette.raised).corner_radius(4).inner_margin(egui::Margin::symmetric(8, 4)).show(ui, |ui| ui.horizontal_wrapped(|ui| {
        group_label(ui, "METHOD");
        for m in METHODS {
            tri_state_button(ui, m, state.prefs.method_filters.entry(m.to_string()).or_default());
        }

        ui.add_space(12.0);
        group_label(ui, "STATUS");
        for st in STATUS_GROUPS {
            tri_state_button(ui, st, state.prefs.status_filters.entry(st.to_string()).or_default());
        }

        ui.add_space(12.0);
        group_label(ui, "FLAGS");
        for (key, label) in FLAGS {
            tri_state_button(ui, label, state.prefs.flag_filters.entry(key.to_string()).or_default());
        }

        if listeners.len() > 1 {
            ui.add_space(12.0);
            group_label(ui, "SOURCE");
            source_combo(ui, &mut state.prefs.source_filter, listeners);
        }

        if state.active_filter_count() > 0 && ui.link("Reset filters").clicked() {
            state.reset_filters();
        }
    }));
}

/// The active filter count, as a small accent pill inside the Filters button.
fn count_pill(ui: &egui::Ui, n: usize) -> RichText {
    let palette = Palette::of(ui.visuals());
    RichText::new(format!(" {n} ")).small().color(palette.accent_text).background_color(palette.accent)
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
