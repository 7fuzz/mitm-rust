use eframe::egui::{self, Color32, RichText};
use crate::state::{InterceptActionCmd, UiState};
use crate::views::pills::render_method_pill;

pub fn render_intercept_view(ui: &mut egui::Ui, state: &mut UiState) {
    // 1. Master Intercept Controls
    ui.horizontal(|ui| {
        ui.heading("Interception & Breakpoints");
        ui.separator();

        let mut enabled = state.intercept_enabled;
        let toggle_label = if enabled {
            RichText::new("● Interceptor ON").color(Color32::from_rgb(239, 68, 68)).strong()
        } else {
            RichText::new("○ Interceptor OFF").color(Color32::GRAY)
        };

        if ui.selectable_label(enabled, toggle_label).clicked() {
            enabled = !enabled;
            state.intercept_enabled = enabled;
            state.intercept_action_queue.push(InterceptActionCmd::ToggleMaster {
                enabled,
                mode: state.intercept_mode.clone(),
            });
        }

        ui.separator();

        ui.label("Mode:");
        let mut mode = state.intercept_mode.clone();
        egui::ComboBox::from_id_salt("intercept_mode_combo")
            .selected_text(&mode)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut mode, "request".to_string(), "Requests only");
                ui.selectable_value(&mut mode, "response".to_string(), "Responses only");
                ui.selectable_value(&mut mode, "both".to_string(), "Both Requests & Responses");
            });

        if mode != state.intercept_mode {
            state.intercept_mode = mode.clone();
            state.intercept_action_queue.push(InterceptActionCmd::ToggleMaster {
                enabled: state.intercept_enabled,
                mode,
            });
        }

        ui.separator();

        let count = state.pending_intercepts.len();
        if count > 0 {
            let badge = RichText::new(format!("⚠️ {} Paused Flow(s)", count))
                .color(Color32::from_rgb(245, 158, 11))
                .strong();
            ui.label(badge);
        } else {
            ui.label(RichText::new("No paused flows").weak());
        }
    });

    ui.separator();

    // 2. Main content: Pending Breakpoints vs Rules Manager
    egui::ScrollArea::vertical().show(ui, |ui| {
        if !state.pending_intercepts.is_empty() {
            render_paused_flows_editor(ui, state);
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
        }

        render_rules_manager(ui, state);
    });
}

fn render_paused_flows_editor(ui: &mut egui::Ui, state: &mut UiState) {
    ui.heading("Paused Flows");

    // Flow selector tabs/list
    let mut selected_id: Option<String> = None;
    ui.horizontal_wrapped(|ui| {
        let active_id = state.active_intercept_id.clone();
        for p in &state.pending_intercepts {
            let is_sel = active_id.as_deref() == Some(&p.flow_id);
            let label = format!("{} [{}] {}", p.method, p.phase, p.url);
            let short_label = if label.len() > 40 { format!("{}...", &label[..37]) } else { label };

            if ui.selectable_label(is_sel, short_label).clicked() {
                selected_id = Some(p.flow_id.clone());
            }
        }
    });

    if let Some(id) = selected_id {
        state.select_active_intercept(&id);
    }

    ui.separator();

    let active_id = match state.active_intercept_id.clone() {
        Some(id) => id,
        None => return,
    };

    let active_flow = match state.pending_intercepts.iter().find(|p| p.flow_id == active_id) {
        Some(f) => f.clone(),
        None => return,
    };

    // Breakpoint action buttons
    ui.horizontal(|ui| {
        if ui.button(RichText::new("▶ Forward (with changes)").color(Color32::from_rgb(34, 197, 94)).strong()).clicked() {
            let headers = state.parse_edited_headers();
            let body_bytes = state.edit_intercept_body.as_bytes().to_vec();
            state.intercept_action_queue.push(InterceptActionCmd::Forward {
                flow_id: active_id.clone(),
                url: Some(state.edit_intercept_url.clone()),
                method: Some(state.edit_intercept_method.clone()),
                headers: Some(headers),
                body: Some(body_bytes),
            });
        }

        if ui.button("▶ Forward Unmodified").clicked() {
            state.intercept_action_queue.push(InterceptActionCmd::Forward {
                flow_id: active_id.clone(),
                url: None,
                method: None,
                headers: None,
                body: None,
            });
        }

        if ui.button(RichText::new("⏹ Drop Flow").color(Color32::from_rgb(239, 68, 68)).strong()).clicked() {
            state.intercept_action_queue.push(InterceptActionCmd::Drop {
                flow_id: active_id.clone(),
            });
        }

        ui.separator();
        ui.label(RichText::new(format!("Phase: {}", active_flow.phase)).weak());
    });

    ui.separator();

    // Editor fields
    egui::Grid::new("intercept_editor_grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.strong("Method:");
            ui.horizontal(|ui| {
                render_method_pill(ui, &state.edit_intercept_method);
                egui::ComboBox::from_id_salt("edit_intercept_method_combo")
                    .selected_text(&state.edit_intercept_method)
                    .show_ui(ui, |ui| {
                        for m in &["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"] {
                            ui.selectable_value(&mut state.edit_intercept_method, m.to_string(), *m);
                        }
                    });
            });
            ui.end_row();

            ui.strong("URL:");
            ui.add(
                egui::TextEdit::singleline(&mut state.edit_intercept_url)
                    .desired_width(f32::INFINITY),
            );
            ui.end_row();

            ui.strong("Headers:");
            ui.add(
                egui::TextEdit::multiline(&mut state.edit_intercept_headers)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(6)
                    .desired_width(f32::INFINITY),
            );
            ui.end_row();

            ui.strong("Body:");
            ui.add(
                egui::TextEdit::multiline(&mut state.edit_intercept_body)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(8)
                    .desired_width(f32::INFINITY),
            );
            ui.end_row();
        });
}

fn render_rules_manager(ui: &mut egui::Ui, state: &mut UiState) {
    ui.heading("Interception Rules");
    ui.label(RichText::new("Traffic matching enabled rules will pause at breakpoint.").weak());
    ui.add_space(4.0);

    let mut rules = state.intercept_rules.clone();
    let mut modified = false;

    // Rules Table
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

    // Add Rule Form
    ui.strong("Add Intercept Rule");
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("new_rule_phase")
            .selected_text(&state.new_rule_phase)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.new_rule_phase, "request".to_string(), "Request");
                ui.selectable_value(&mut state.new_rule_phase, "response".to_string(), "Response");
                ui.selectable_value(&mut state.new_rule_phase, "both".to_string(), "Both");
            });

        egui::ComboBox::from_id_salt("new_rule_field")
            .selected_text(&state.new_rule_field)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.new_rule_field, "url".to_string(), "URL");
                ui.selectable_value(&mut state.new_rule_field, "host".to_string(), "Host");
                ui.selectable_value(&mut state.new_rule_field, "path".to_string(), "Path");
                ui.selectable_value(&mut state.new_rule_field, "method".to_string(), "Method");
                ui.selectable_value(&mut state.new_rule_field, "header".to_string(), "Header");
            });

        egui::ComboBox::from_id_salt("new_rule_operator")
            .selected_text(&state.new_rule_operator)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.new_rule_operator, "contains".to_string(), "contains");
                ui.selectable_value(&mut state.new_rule_operator, "equals".to_string(), "equals");
                ui.selectable_value(&mut state.new_rule_operator, "regex".to_string(), "regex");
            });

        ui.add(
            egui::TextEdit::singleline(&mut state.new_rule_value)
                .hint_text("Match pattern...")
                .desired_width(160.0),
        );

        egui::ComboBox::from_id_salt("new_rule_action")
            .selected_text(&state.new_rule_action)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.new_rule_action, "intercept".to_string(), "Intercept");
                ui.selectable_value(&mut state.new_rule_action, "pass".to_string(), "Pass through");
            });

        let can_add = !state.new_rule_value.trim().is_empty();
        if ui.add_enabled(can_add, egui::Button::new("➕ Add Rule")).clicked() {
            let new_rule = mitm_core::db::InterceptRule {
                id: uuid::Uuid::new_v4().to_string(),
                is_enabled: true,
                target_phase: state.new_rule_phase.clone(),
                match_field: state.new_rule_field.clone(),
                operator: state.new_rule_operator.clone(),
                match_value: state.new_rule_value.trim().to_string(),
                order_index: rules.len() as i32,
                created_at_ms: chrono::Utc::now().timestamp_millis(),
                action: state.new_rule_action.clone(),
            };
            rules.push(new_rule);
            state.new_rule_value.clear();
            modified = true;
        }
    });

    if modified {
        state.intercept_rules = rules.clone();
        state.intercept_action_queue.push(InterceptActionCmd::SaveRules { rules });
    }
}
