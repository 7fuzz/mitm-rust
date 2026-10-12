use eframe::egui::{self, RichText};
use crate::ui::theme::{self, Tone};
use crate::app::command::Commands;
use crate::widgets::button::{icon_button, primary};
use crate::widgets::list_row::list_row;
use crate::widgets::pills::{badge, method_text, render_method_pill};
use crate::widgets::section::section;
use crate::widgets::toggle::labelled_toggle;
use crate::widgets::{source_scope, split};
use mitm_core::state::ListenerConfig;
use crate::features::request_editor::kv_table::{self, KvLabels};
use super::{Cmd, InterceptState};
use crate::ui::icons;

pub fn show(ui: &mut egui::Ui, state: &mut InterceptState, listeners: &[ListenerConfig], cmds: &mut Commands) {
    ui.horizontal(|ui| {
        if labelled_toggle(ui, &mut state.enabled, "Intercept").changed() {
            cmds.push(Cmd::Configure { enabled: state.enabled, mode: state.mode.clone() });
        }
        ui.add_space(12.0);

        ui.weak("Pause");
        let modes = [("request", "Requests"), ("response", "Responses"), ("both", "Requests and responses")];
        let mut mode = state.mode.clone();
        let selected = modes.iter().find(|(key, _)| *key == mode).map_or(mode.as_str(), |(_, label)| *label).to_string();
        egui::ComboBox::from_id_salt("intercept_mode_combo")
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for (key, label) in modes {
                    ui.selectable_value(&mut mode, key.to_string(), label);
                }
            });

        if mode != state.mode {
            state.mode = mode.clone();
            cmds.push(Cmd::Configure {
                enabled: state.enabled,
                mode,
            });
        }

        ui.add_space(12.0);
        if source_scope::chips(ui, &mut state.source_scope, listeners) {
            cmds.push(Cmd::SetScope(state.source_scope.clone()));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let rules = egui::Button::new(("Rules", RichText::new(state.rules.len().to_string()).monospace().weak()))
                .selected(state.prefs.rules_open);
            if ui.add(rules).on_hover_text("Show or hide the rules panel").clicked() {
                state.prefs.rules_open = !state.prefs.rules_open;
            }
            ui.toggle_value(&mut state.prefs.raise_window, icons::BELL).on_hover_text("Bring the window forward when a flow pauses");

            let count = state.pending.len();
            if count > 0 {
                ui.add_space(12.0);
                if ui.button(RichText::new("Drop all").color(theme::color(ui, Tone::Danger))).clicked() {
                    cmds.push(Cmd::DropAll);
                }
                if ui.button("Forward all").clicked() {
                    cmds.push(Cmd::ForwardAll);
                }
                badge(ui, &format!("{count} paused"), Tone::Warn);
            }
        });
    });
    ui.add_space(4.0);

    if !state.prefs.rules_open {
        paused_area(ui, state, cmds);
        return;
    }
    let mut work_fraction = state.prefs.work_fraction;
    let [mut work_ui, mut rules_ui] = split::side_by_side(ui, "intercept_rules_split", &mut work_fraction);
    state.prefs.work_fraction = work_fraction;
    paused_area(&mut work_ui, state, cmds);
    egui::ScrollArea::vertical().id_salt("intercept_rules").show(&mut rules_ui, |ui| {
        render_rules_manager(ui, state, cmds);
    });
}

fn paused_area(work_ui: &mut egui::Ui, state: &mut InterceptState, cmds: &mut Commands) {
    if state.pending.is_empty() {
        let hint = if state.enabled { "Nothing paused" } else { "Interception is off" };
        work_ui.weak(hint);
    } else {
        let mut queue_fraction = state.prefs.queue_fraction;
        let [mut queue_ui, mut editor_ui] = split::side_by_side(work_ui, "intercept_queue_split", &mut queue_fraction);
        state.prefs.queue_fraction = queue_fraction;
        queue(&mut queue_ui, state);
        egui::ScrollArea::vertical().id_salt("intercept_editor").show(&mut editor_ui, |ui| {
            render_paused_flows_editor(ui, state, cmds);
        });
    }
}

fn queue(ui: &mut egui::Ui, state: &mut InterceptState) {
    let mut selected = None;
    egui::ScrollArea::vertical().id_salt("intercept_queue").auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for flow in &state.pending {
            let is_active = state.active_id.as_deref() == Some(&flow.flow_id);
            let (host, path) = host_and_path(&flow.url);
            let row = list_row(ui, is_active, |ui| {
                ui.label(method_text(ui.visuals(), &flow.method));
                let tone = if flow.phase == "response" { Tone::Info } else { Tone::Warn };
                badge(ui, &flow.phase, tone);
                ui.add(egui::Label::new(RichText::new(host).monospace().weak()).truncate().selectable(false));
                ui.add(egui::Label::new(RichText::new(path).monospace()).truncate().selectable(false));
            });
            if row.clicked() {
                selected = Some(flow.flow_id.clone());
            }
        }
    });
    if let Some(id) = selected {
        state.select(&id);
    }
}

fn host_and_path(url: &str) -> (&str, &str) {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    }
}

fn render_paused_flows_editor(ui: &mut egui::Ui, state: &mut InterceptState, cmds: &mut Commands) {
    let active_id = match state.active_id.clone() {
        Some(id) => id,
        None => return,
    };

    let active_flow = match state.pending.iter().find(|p| p.flow_id == active_id) {
        Some(f) => f.clone(),
        None => return,
    };

    ui.horizontal(|ui| {
        if ui.add(primary(ui, "Forward")).on_hover_text("Forward with your edits").clicked() {
            cmds.push(Cmd::ForwardActive { modified: true });
        }
        if ui.button("Forward unmodified").clicked() {
            cmds.push(Cmd::ForwardActive { modified: false });
        }
        if ui.button(RichText::new("Drop").color(theme::color(ui, Tone::Danger))).clicked() {
            cmds.push(Cmd::DropActive);
        }
        ui.add_space(8.0);
        let tone = if active_flow.phase == "response" { Tone::Info } else { Tone::Warn };
        badge(ui, &active_flow.phase, tone);
    });
    ui.add_space(6.0);

    egui::Grid::new("intercept_editor_grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label(RichText::new("Method").weak());
            ui.horizontal(|ui| {
                render_method_pill(ui, &state.edit.method);
                egui::ComboBox::from_id_salt("edit_intercept_method_combo")
                    .selected_text(&state.edit.method)
                    .show_ui(ui, |ui| {
                        for m in &["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"] {
                            ui.selectable_value(&mut state.edit.method, m.to_string(), *m);
                        }
                    });
            });
            ui.end_row();

            ui.label(RichText::new("URL").weak());
            let mut url_layouter = crate::widgets::template::singleline_layouter(egui::TextStyle::Monospace, None);
            crate::widgets::template::show(ui, &mut state.edit.url, |text| {
                egui::TextEdit::singleline(text)
                    .font(egui::TextStyle::Monospace)
                    .layouter(&mut url_layouter)
                    .desired_width(f32::INFINITY)
            });
            ui.end_row();

            ui.label(RichText::new("Headers").weak());
            ui.vertical(|ui| {
                let labels = KvLabels { key: "Header Name", value: "Header Value" };
                kv_table::show(ui, "intercept_headers", &mut state.edit.headers, labels, &[]);
            });
            ui.end_row();

            ui.label(RichText::new("Body").weak());
            ui.add(
                egui::TextEdit::multiline(&mut state.edit.body)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(8)
                    .desired_width(f32::INFINITY),
            );
            ui.end_row();
        });
}

const PHASES: [(&str, &str); 3] = [("request", "Request"), ("response", "Response"), ("both", "Both")];
const FIELDS: [(&str, &str); 5] = [("url", "URL"), ("host", "Host"), ("path", "Path"), ("method", "Method"), ("header", "Header")];
const OPERATORS: [(&str, &str); 3] = [("contains", "contains"), ("equals", "equals"), ("regex", "regex")];

fn render_rules_manager(ui: &mut egui::Ui, state: &mut InterceptState, cmds: &mut Commands) {
    section(ui, "Rules");

    let rules = &mut state.rules;
    let mut modified = false;
    let mut delete = None;
    let mut swap = None;
    let count = rules.len();

    egui::Grid::new("rules_table_grid").striped(false).num_columns(7).spacing([8.0, 6.0]).show(ui, |ui| {
        for (index, rule) in rules.iter_mut().enumerate() {
            ui.push_id(&rule.id, |ui| {
                modified |= ui.checkbox(&mut rule.is_enabled, "").changed();
            });
            modified |= rule_cells(ui, &rule.id, &mut rule.target_phase, &mut rule.match_field, &mut rule.operator, &mut rule.match_value, &mut rule.action);
            ui.horizontal(|ui| {
                if ui.add_enabled(index > 0, egui::Button::new(icons::CARET_UP).small()).clicked() {
                    swap = Some((index, index - 1));
                }
                if ui.add_enabled(index + 1 < count, egui::Button::new(icons::CARET_DOWN).small()).clicked() {
                    swap = Some((index, index + 1));
                }
                if icon_button(ui, icons::TRASH, "Delete rule").clicked() {
                    delete = Some(index);
                }
            });
            ui.end_row();
        }

        ui.label("");
        let draft = &mut state.new_rule;
        let value_before = draft.value.clone();
        rule_cells(ui, "new_rule", &mut draft.phase, &mut draft.field, &mut draft.operator, &mut draft.value, &mut draft.action);
        ui.label("");
        ui.end_row();
        if value_before.is_empty() && !draft.value.trim().is_empty() {
            rules.push(mitm_core::db::InterceptRule {
                id: uuid::Uuid::new_v4().to_string(),
                is_enabled: true,
                target_phase: draft.phase.clone(),
                match_field: draft.field.clone(),
                operator: draft.operator.clone(),
                match_value: std::mem::take(&mut draft.value),
                order_index: 0,
                created_at_ms: chrono::Utc::now().timestamp_millis(),
                action: draft.action.clone(),
            });
            modified = true;
        }
    });

    if let Some((a, b)) = swap {
        rules.swap(a, b);
        modified = true;
    }
    if let Some(index) = delete {
        rules.remove(index);
        modified = true;
    }
    if modified {
        for (index, rule) in rules.iter_mut().enumerate() {
            rule.order_index = index as i32;
        }
        cmds.push(Cmd::SaveRules(rules.clone()));
    }
}

fn rule_cells(
    ui: &mut egui::Ui,
    id: &str,
    phase: &mut String,
    field: &mut String,
    operator: &mut String,
    value: &mut String,
    action: &mut String,
) -> bool {
    let mut changed = combo(ui, (id, "phase"), phase, &PHASES);
    changed |= combo(ui, (id, "field"), field, &FIELDS);
    changed |= combo(ui, (id, "operator"), operator, &OPERATORS);
    changed |= ui
        .add(egui::TextEdit::singleline(value).id_salt((id, "value")).hint_text("Match value").desired_width(180.0))
        .changed();
    changed |= action_chip(ui, action);
    changed
}

fn combo(ui: &mut egui::Ui, id: impl std::hash::Hash + std::fmt::Debug, value: &mut String, options: &[(&str, &str)]) -> bool {
    let before = value.clone();
    let selected = options.iter().find(|(key, _)| *key == value).map_or(value.as_str(), |(_, label)| *label).to_string();
    egui::ComboBox::from_id_salt(id).selected_text(selected).width(90.0).show_ui(ui, |ui| {
        for (key, label) in options {
            ui.selectable_value(value, key.to_string(), *label);
        }
    });
    *value != before
}

// Tauri's whitelist/blacklist: a matching rule either pauses the flow or lets it pass.
fn action_chip(ui: &mut egui::Ui, action: &mut String) -> bool {
    let pass = action == "pass";
    let (label, tone) = if pass { ("PASS", Tone::Success) } else { ("PAUSE", Tone::Warn) };
    let chip = egui::Button::new(RichText::new(label).color(theme::color(ui, tone)).strong().small())
        .fill(tone.tint(ui.visuals(), 36));
    let clicked = ui.add(chip).on_hover_text("Click to switch between pause and pass").clicked();
    if clicked {
        *action = if pass { "intercept" } else { "pass" }.to_string();
    }
    clicked
}
