use eframe::egui::{self, RichText};
use mitm_core::repeater::{HeaderItem, RepeaterTab};
use crate::command::{AppCommand, Commands};
use crate::features::request_editor::body::{content_type_for, convert, REPEATER_BODY_TYPES};
use crate::features::request_editor::kv_table::{self, standard_headers, KvLabels};
use crate::features::request_editor::steps::{self, StepOption};
use crate::features::request_editor::url_params::{build_url, new_id, parse_query};
use crate::features::request_editor::{body_editor, extract_rules};
use crate::format::pretty_json;
use crate::widgets::template;
use super::{EditorTab, RepeaterState};

pub const METHODS: [&str; 7] = ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"];

pub fn show(ui: &mut egui::Ui, state: &mut RepeaterState, cmds: &mut Commands, now: f64) {
    let Some(index) = state.active_id.as_ref().and_then(|id| state.tabs.iter().position(|t| &t.id == id)) else {
        return;
    };
    let step_options: Vec<StepOption> = state
        .tabs
        .iter()
        .filter(|t| t.id != state.tabs[index].id)
        .map(|t| StepOption { id: t.id.clone(), label: format!("{} {}", t.method, t.url) })
        .collect();
    let executing = state.executing.contains(&state.tabs[index].id);
    let tab = &mut state.tabs[index];
    let mut changed = false;
    let mut send = false;

    ui.horizontal(|ui| {
        ui.set_min_height(28.0);
        changed |= method_combo(ui, &mut tab.method);
        let width = ui.available_width() - 150.0;
        let mut layouter = template::layouter(egui::TextStyle::Monospace, None);
        let url = template::hover_tooltip(
            ui.add(
                egui::TextEdit::singleline(&mut tab.url)
                    .hint_text("https://api.example.com/v1/resource or {{BASE_URL}}/path")
                    .layouter(&mut layouter)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(width),
            ),
            &tab.url,
        );
        if url.changed() {
            tab.params = parse_query(&tab.url, &tab.params);
            changed = true;
        }
        let label = if executing { "Sending..." } else { "▶ Send" };
        let button = egui::Button::new(RichText::new(label).strong()).fill(ui.visuals().selection.bg_fill);
        if ui.add_enabled(!executing, button).on_hover_text("Ctrl+Enter").clicked() {
            send = true;
        }
        if ui.selectable_label(state.prefs.history_open, "🕘").on_hover_text("Execution history").clicked() {
            state.prefs.history_open = !state.prefs.history_open;
        }
    });
    let hotkey = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter));
    if hotkey && !executing && ui.ui_contains_pointer() {
        send = true;
    }
    ui.separator();

    ui.horizontal(|ui| {
        let params = tab.params.iter().filter(|p| p.enabled && !p.key.trim().is_empty()).count();
        let headers = tab.headers.iter().filter(|h| h.enabled && !h.key.trim().is_empty()).count();
        let tabs = [
            (EditorTab::Params, format!("Params ({})", params)),
            (EditorTab::Headers, format!("Headers ({})", headers)),
            (EditorTab::Body, "Body".to_string()),
            (EditorTab::Extract, format!("Auto-Extract ({})", tab.extract_rules.len())),
            (EditorTab::Steps, format!("Pre/Post ({})", tab.pre_requests.len() + tab.post_requests.len())),
        ];
        for (value, label) in tabs {
            let response = ui.selectable_value(&mut state.editor_tab, value, label);
            if value == EditorTab::Headers {
                response.on_hover_text("A disabled Content-Type is detected from the body");
            }
        }

        if state.editor_tab == EditorTab::Body {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if tab.body_type == "json" && ui.small_button("Format").on_hover_text("Pretty-print JSON").clicked() {
                    if let Some(pretty) = tab.body_content.as_deref().and_then(pretty_json) {
                        tab.body_content = Some(pretty);
                        changed = true;
                    }
                }
                changed |= body_type_combo(ui, tab);
            });
        }
    });
    ui.separator();

    match state.editor_tab {
        EditorTab::Body => {
            let content = tab.body_content.get_or_insert_with(String::new);
            changed |= body_editor::show(ui, "repeater_body", &tab.body_type, content);
        }
        other => {
            egui::ScrollArea::vertical().id_salt("repeater_editor").auto_shrink([false, false]).show(ui, |ui| match other {
                EditorTab::Params => {
                    let labels = KvLabels { key: "Parameter Key", value: "Parameter Value" };
                    if kv_table::show(ui, "repeater_params", &mut tab.params, labels, &[]) {
                        tab.url = build_url(&tab.url, &tab.params);
                        changed = true;
                    }
                }
                EditorTab::Headers => {
                    let labels = KvLabels { key: "Header Name", value: "Header Value" };
                    let suggestions = standard_headers(content_type_for(&tab.body_type).unwrap_or("application/json"));
                    changed |= kv_table::show(ui, "repeater_headers", &mut tab.headers, labels, &suggestions);
                }
                EditorTab::Extract => changed |= extract_rules::show(ui, "repeater_rules", &mut tab.extract_rules),
                EditorTab::Steps => {
                    changed |= steps::show(ui, &mut tab.pre_requests, &mut tab.post_requests, &step_options)
                }
                EditorTab::Body => {}
            });
        }
    }

    let id = tab.id.clone();
    if changed {
        state.mark_dirty(&id, now);
    }
    if send {
        cmds.push(AppCommand::RepeaterSend(id));
    }
}

pub fn method_combo(ui: &mut egui::Ui, method: &mut String) -> bool {
    let before = method.clone();
    egui::ComboBox::from_id_salt("method")
        .width(86.0)
        .selected_text(RichText::new(method.as_str()).strong())
        .show_ui(ui, |ui| {
            for m in METHODS {
                ui.selectable_value(method, m.to_string(), m);
            }
        });
    *method != before
}

fn body_type_combo(ui: &mut egui::Ui, tab: &mut RepeaterTab) -> bool {
    let current = REPEATER_BODY_TYPES
        .iter()
        .find(|(id, _)| *id == tab.body_type)
        .map_or("Raw", |(_, label)| *label);
    let mut picked = None;
    egui::ComboBox::from_id_salt("body_type").selected_text(current).show_ui(ui, |ui| {
        for (id, label) in REPEATER_BODY_TYPES {
            if ui.selectable_label(tab.body_type == id, label).clicked() && tab.body_type != id {
                picked = Some(id);
            }
        }
    });
    let Some(next) = picked else { return false };
    set_content_type_header(&mut tab.headers, content_type_for(next));
    let content = tab.body_content.take().unwrap_or_default();
    tab.body_content = Some(convert(&tab.body_type, next, &content));
    tab.body_type = next.to_string();
    true
}

fn set_content_type_header(headers: &mut Vec<HeaderItem>, content_type: Option<&str>) {
    let existing = headers.iter_mut().find(|h| h.key.trim().eq_ignore_ascii_case("content-type"));
    match (existing, content_type) {
        (Some(h), None) => h.enabled = false,
        (Some(h), Some(ct)) => {
            h.value = ct.to_string();
            h.enabled = true;
        }
        (None, Some(ct)) => headers.push(HeaderItem {
            id: new_id(),
            key: "Content-Type".to_string(),
            value: ct.to_string(),
            enabled: true,
        }),
        (None, None) => {}
    }
}
