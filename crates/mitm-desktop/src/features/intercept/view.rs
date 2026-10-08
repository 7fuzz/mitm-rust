use eframe::egui::{self, RichText};
use crate::theme::{self, Tone};
use crate::command::{AppCommand, Commands};
use crate::widgets::pills::{badge, render_method_pill};
use crate::widgets::{source_scope, split};
use mitm_core::state::ListenerConfig;
use crate::features::request_editor::kv_table::{self, KvLabels};
use super::InterceptState;

pub fn show(ui: &mut egui::Ui, state: &mut InterceptState, listeners: &[ListenerConfig], cmds: &mut Commands) {
    ui.horizontal(|ui| {
        ui.heading("Interception & Breakpoints");
        ui.separator();

        let mut enabled = state.enabled;
        let toggle_label = if enabled {
            RichText::new("● Interceptor ON").color(theme::color(ui, Tone::Danger)).strong()
        } else {
            RichText::new("○ Interceptor OFF").weak()
        };

        if ui.selectable_label(enabled, toggle_label).clicked() {
            enabled = !enabled;
            state.enabled = enabled;
            cmds.push(AppCommand::ConfigureInterceptor {
                enabled,
                mode: state.mode.clone(),
            });
        }

        ui.separator();

        ui.label("Mode:");
        let mut mode = state.mode.clone();
        egui::ComboBox::from_id_salt("intercept_mode_combo")
            .selected_text(&mode)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut mode, "request".to_string(), "Requests only");
                ui.selectable_value(&mut mode, "response".to_string(), "Responses only");
                ui.selectable_value(&mut mode, "both".to_string(), "Both Requests & Responses");
            });

        if mode != state.mode {
            state.mode = mode.clone();
            cmds.push(AppCommand::ConfigureInterceptor {
                enabled: state.enabled,
                mode,
            });
        }

        ui.toggle_value(&mut state.prefs.raise_window, "🔔").on_hover_text("Bring the window forward when a flow pauses");
        ui.separator();
        if source_scope::chips(ui, &mut state.source_scope, listeners) {
            cmds.push(AppCommand::SetInterceptScope(state.source_scope.clone()));
        }
        ui.separator();

        let count = state.pending.len();
        if count > 0 {
            let badge = RichText::new(format!("⚠️ {} Paused Flow(s)", count))
                .color(theme::color(ui, Tone::Warn))
                .strong();
            ui.label(badge);
            if ui.button("▶ Forward all").clicked() {
                cmds.push(AppCommand::ForwardAllIntercepts);
            }
            if ui.button(RichText::new("⏹ Drop all").color(theme::color(ui, Tone::Danger))).clicked() {
                cmds.push(AppCommand::DropAllIntercepts);
            }
        }
    });

    ui.separator();

    let mut work_fraction = state.prefs.work_fraction;
    let [mut work_ui, mut rules_ui] = split::side_by_side(ui, "intercept_rules_split", &mut work_fraction);
    state.prefs.work_fraction = work_fraction;
    if !state.pending.is_empty() {
        let mut queue_fraction = state.prefs.queue_fraction;
        let [mut queue_ui, mut editor_ui] = split::side_by_side(&mut work_ui, "intercept_queue_split", &mut queue_fraction);
        state.prefs.queue_fraction = queue_fraction;
        queue(&mut queue_ui, state);
        egui::ScrollArea::vertical().id_salt("intercept_editor").show(&mut editor_ui, |ui| {
            render_paused_flows_editor(ui, state, cmds);
        });
    }
    egui::ScrollArea::vertical().id_salt("intercept_rules").show(&mut rules_ui, |ui| {
        render_rules_manager(ui, state, cmds);
    });
}

fn queue(ui: &mut egui::Ui, state: &mut InterceptState) {
    let mut selected = None;
    egui::ScrollArea::vertical().id_salt("intercept_queue").auto_shrink([false, false]).show(ui, |ui| {
        for flow in &state.pending {
            let is_active = state.active_id.as_deref() == Some(&flow.flow_id);
            let (host, path) = host_and_path(&flow.url);
            let highlight = ui.painter().add(egui::Shape::Noop);
            let response = ui
                .horizontal(|ui| {
                    render_method_pill(ui, &flow.method);
                    let tone = if flow.phase == "response" { Tone::Info } else { Tone::Warn };
                    badge(ui, &flow.phase.to_uppercase(), tone);
                    ui.add(egui::Label::new(RichText::new(host).strong()).truncate());
                    ui.add(egui::Label::new(RichText::new(path).weak()).truncate());
                })
                .response;
            let row = ui.interact(response.rect, ui.id().with(&flow.flow_id), egui::Sense::click());
            if is_active {
                let fill = ui.visuals().selection.bg_fill.gamma_multiply(0.35);
                ui.painter().set(highlight, egui::epaint::RectShape::filled(response.rect, 2.0, fill));
            }
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
        if ui.button(RichText::new("▶ Forward (with changes)").color(theme::color(ui, Tone::Success)).strong()).clicked() {
            let headers = state.edited_headers();
            let body_bytes = state.edit.body.as_bytes().to_vec();
            cmds.push(AppCommand::ForwardIntercept {
                flow_id: active_id.clone(),
                url: Some(state.edit.url.clone()),
                method: Some(state.edit.method.clone()),
                headers: Some(headers),
                body: Some(body_bytes),
            });
        }

        if ui.button("▶ Forward Unmodified").clicked() {
            cmds.push(AppCommand::ForwardIntercept {
                flow_id: active_id.clone(),
                url: None,
                method: None,
                headers: None,
                body: None,
            });
        }

        if ui.button(RichText::new("⏹ Drop Flow").color(theme::color(ui, Tone::Danger)).strong()).clicked() {
            cmds.push(AppCommand::DropIntercept {
                flow_id: active_id.clone(),
            });
        }

        ui.separator();
        ui.label(RichText::new(format!("Phase: {}", active_flow.phase)).weak());
    });

    ui.separator();

    egui::Grid::new("intercept_editor_grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.strong("Method:");
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

            ui.strong("URL:");
            let mut url_layouter = crate::widgets::template::layouter(egui::TextStyle::Monospace, None);
            crate::widgets::template::show(ui, &mut state.edit.url, |text| {
                egui::TextEdit::singleline(text)
                    .font(egui::TextStyle::Monospace)
                    .layouter(&mut url_layouter)
                    .desired_width(f32::INFINITY)
            });
            ui.end_row();

            ui.strong("Headers:");
            ui.vertical(|ui| {
                let labels = KvLabels { key: "Header Name", value: "Header Value" };
                kv_table::show(ui, "intercept_headers", &mut state.edit.headers, labels, &[]);
            });
            ui.end_row();

            ui.strong("Body:");
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
    ui.strong("Rules");
    ui.add_space(4.0);

    let rules = &mut state.rules;
    let mut modified = false;
    let mut delete = None;
    let mut swap = None;
    let count = rules.len();

    egui::Grid::new("rules_table_grid").striped(true).num_columns(7).spacing([8.0, 6.0]).show(ui, |ui| {
        for (index, rule) in rules.iter_mut().enumerate() {
            ui.push_id(&rule.id, |ui| {
                modified |= ui.checkbox(&mut rule.is_enabled, "").changed();
            });
            modified |= rule_cells(ui, &rule.id, &mut rule.target_phase, &mut rule.match_field, &mut rule.operator, &mut rule.match_value, &mut rule.action);
            ui.horizontal(|ui| {
                if ui.add_enabled(index > 0, egui::Button::new("⏶").small()).clicked() {
                    swap = Some((index, index - 1));
                }
                if ui.add_enabled(index + 1 < count, egui::Button::new("⏷").small()).clicked() {
                    swap = Some((index, index + 1));
                }
                if ui.small_button("🗑").clicked() {
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
        cmds.push(AppCommand::SaveInterceptRules(rules.clone()));
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
    let chip = egui::Button::new(RichText::new(label).color(theme::color(ui, tone)).strong().size(11.0))
        .fill(tone.tint(ui.visuals(), 36));
    let clicked = ui.add(chip).on_hover_text("Click to switch between pause and pass").clicked();
    if clicked {
        *action = if pass { "intercept" } else { "pass" }.to_string();
    }
    clicked
}
