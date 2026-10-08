use eframe::egui::{self, RichText};
use crate::theme::{self, Tone};
use crate::command::{AppCommand, Commands};
use crate::widgets::pills::render_method_pill;
use super::InterceptState;

pub fn show(ui: &mut egui::Ui, state: &mut InterceptState, cmds: &mut Commands) {
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

        ui.separator();

        let count = state.pending.len();
        if count > 0 {
            let badge = RichText::new(format!("⚠️ {} Paused Flow(s)", count))
                .color(theme::color(ui, Tone::Warn))
                .strong();
            ui.label(badge);
        } else {
            ui.label(RichText::new("No paused flows").weak());
        }
    });

    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        if !state.pending.is_empty() {
            render_paused_flows_editor(ui, state, cmds);
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
        }

        render_rules_manager(ui, state, cmds);
    });
}

fn render_paused_flows_editor(ui: &mut egui::Ui, state: &mut InterceptState, cmds: &mut Commands) {
    ui.heading("Paused Flows");

    let mut selected_id: Option<String> = None;
    ui.horizontal_wrapped(|ui| {
        let active_id = state.active_id.clone();
        for p in &state.pending {
            let is_sel = active_id.as_deref() == Some(&p.flow_id);
            let label = format!("{} [{}] {}", p.method, p.phase, p.url);
            let short_label = if label.len() > 40 { format!("{}...", &label[..37]) } else { label };

            if ui.selectable_label(is_sel, short_label).clicked() {
                selected_id = Some(p.flow_id.clone());
            }
        }
    });

    if let Some(id) = selected_id {
        state.select(&id);
    }

    ui.separator();

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
            let url_resp = ui.add(
                egui::TextEdit::singleline(&mut state.edit.url)
                    .font(egui::TextStyle::Monospace)
                    .layouter(&mut url_layouter)
                    .desired_width(f32::INFINITY),
            );
            crate::widgets::template::hover_tooltip(url_resp, &state.edit.url);
            ui.end_row();

            ui.strong("Headers:");
            let mut headers_layouter = crate::widgets::template::layouter(egui::TextStyle::Monospace, None);
            let headers_resp = ui.add(
                egui::TextEdit::multiline(&mut state.edit.headers)
                    .font(egui::TextStyle::Monospace)
                    .layouter(&mut headers_layouter)
                    .desired_rows(6)
                    .desired_width(f32::INFINITY),
            );
            crate::widgets::template::hover_tooltip(headers_resp, &state.edit.headers);
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

fn render_rules_manager(ui: &mut egui::Ui, state: &mut InterceptState, cmds: &mut Commands) {
    ui.heading("Interception Rules");
    ui.label(RichText::new("Traffic matching enabled rules will pause at breakpoint.").weak());
    ui.add_space(4.0);

    let mut rules = state.rules.clone();
    let mut modified = false;

    egui::Grid::new("rules_table_grid")
        .striped(true)
        .num_columns(7)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.strong("Enabled");
            ui.strong("Phase");
            ui.strong("Field");
            ui.strong("Operator");
            ui.strong("Match Value");
            ui.strong("Action");
            ui.strong("");
            ui.end_row();

            let mut delete_index: Option<usize> = None;

            for (idx, rule) in rules.iter_mut().enumerate() {
                if ui.checkbox(&mut rule.is_enabled, "").changed() {
                    modified = true;
                }
                ui.label(&rule.target_phase);
                ui.label(&rule.match_field);
                ui.label(&rule.operator);
                ui.label(&rule.match_value);
                ui.label(&rule.action);

                if ui.small_button("🗑").clicked() {
                    delete_index = Some(idx);
                }
                ui.end_row();
            }

            if let Some(idx) = delete_index {
                rules.remove(idx);
                modified = true;
            }
        });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);

    ui.strong("Add Intercept Rule");
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("new_rule_phase")
            .selected_text(&state.new_rule.phase)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.new_rule.phase, "request".to_string(), "Request");
                ui.selectable_value(&mut state.new_rule.phase, "response".to_string(), "Response");
                ui.selectable_value(&mut state.new_rule.phase, "both".to_string(), "Both");
            });

        egui::ComboBox::from_id_salt("new_rule_field")
            .selected_text(&state.new_rule.field)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.new_rule.field, "url".to_string(), "URL");
                ui.selectable_value(&mut state.new_rule.field, "host".to_string(), "Host");
                ui.selectable_value(&mut state.new_rule.field, "path".to_string(), "Path");
                ui.selectable_value(&mut state.new_rule.field, "method".to_string(), "Method");
                ui.selectable_value(&mut state.new_rule.field, "header".to_string(), "Header");
            });

        egui::ComboBox::from_id_salt("new_rule_operator")
            .selected_text(&state.new_rule.operator)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.new_rule.operator, "contains".to_string(), "contains");
                ui.selectable_value(&mut state.new_rule.operator, "equals".to_string(), "equals");
                ui.selectable_value(&mut state.new_rule.operator, "regex".to_string(), "regex");
            });

        ui.add(
            egui::TextEdit::singleline(&mut state.new_rule.value)
                .hint_text("Match pattern...")
                .desired_width(160.0),
        );

        egui::ComboBox::from_id_salt("new_rule_action")
            .selected_text(&state.new_rule.action)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.new_rule.action, "intercept".to_string(), "Intercept");
                ui.selectable_value(&mut state.new_rule.action, "pass".to_string(), "Pass through");
            });

        let can_add = !state.new_rule.value.trim().is_empty();
        if ui.add_enabled(can_add, egui::Button::new("➕ Add Rule")).clicked() {
            let new_rule = mitm_core::db::InterceptRule {
                id: uuid::Uuid::new_v4().to_string(),
                is_enabled: true,
                target_phase: state.new_rule.phase.clone(),
                match_field: state.new_rule.field.clone(),
                operator: state.new_rule.operator.clone(),
                match_value: state.new_rule.value.trim().to_string(),
                order_index: rules.len() as i32,
                created_at_ms: chrono::Utc::now().timestamp_millis(),
                action: state.new_rule.action.clone(),
            };
            rules.push(new_rule);
            state.new_rule.value.clear();
            modified = true;
        }
    });

    if modified {
        state.rules = rules.clone();
        cmds.push(AppCommand::SaveInterceptRules(rules));
    }
}
