use eframe::egui::{self, RichText};
use mitm_core::state::RewriteRule;
use super::ACTIONS;

const MATCH_FIELDS: [(&str, &str); 6] =
    [("all", "All traffic"), ("url", "URL"), ("host", "Host"), ("path", "Path"), ("method", "Method"), ("header", "Header")];
const OPERATORS: [(&str, &str); 5] =
    [("contains", "contains"), ("equals", "equals"), ("starts_with", "starts with"), ("ends_with", "ends with"), ("regex", "regex")];

/// Edits one rule in place; the fields shown follow the action type. Returns whether anything changed.
pub fn show(ui: &mut egui::Ui, rule: &mut RewriteRule) -> bool {
    let mut changed = false;
    egui::Grid::new(("rewrite_editor", &rule.id)).num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
        ui.label("Name");
        changed |= ui.add(egui::TextEdit::singleline(&mut rule.name).desired_width(f32::INFINITY)).changed();
        ui.end_row();

        ui.label("Action");
        let before = rule.action_type.clone();
        changed |= combo(ui, "action", &mut rule.action_type, &ACTIONS);
        if rule.action_type != before {
            rule.target_part = targets(&rule.action_type).first().map_or(String::new(), |(key, _)| key.to_string());
        }
        ui.end_row();

        ui.label("When");
        ui.horizontal(|ui| {
            changed |= combo(ui, "match_field", &mut rule.match_field, &MATCH_FIELDS);
            if rule.match_field != "all" {
                changed |= combo(ui, "operator", &mut rule.match_operator, &OPERATORS);
                changed |= ui.add(egui::TextEdit::singleline(&mut rule.match_value).hint_text("value").desired_width(200.0)).changed();
            }
        });
        ui.end_row();

        let target_options = targets(&rule.action_type);
        if target_options.len() > 1 {
            ui.label("Target");
            changed |= combo(ui, "target", &mut rule.target_part, target_options);
            ui.end_row();
        }

        let needs_header = rule.match_field == "header" || rule.target_part == "header";
        if needs_header && rule.action_type != "full_response" {
            ui.label("Header name");
            let name = rule.target_header.get_or_insert_with(String::new);
            changed |= ui.add(egui::TextEdit::singleline(name).desired_width(200.0)).changed();
            ui.end_row();
        }

        changed |= action_fields(ui, rule);
    });
    changed
}

fn targets(action: &str) -> &'static [(&'static str, &'static str)] {
    match action {
        "partial_request" => &[("url", "URL"), ("query", "Query param"), ("header", "Header"), ("body", "Body")],
        "partial_response" => &[("body", "Body"), ("header", "Header")],
        "full_request" => &[("url", "URL"), ("body", "Body"), ("method", "Method")],
        _ => &[],
    }
}

fn action_fields(ui: &mut egui::Ui, rule: &mut RewriteRule) -> bool {
    let mut changed = false;
    match (rule.action_type.as_str(), rule.target_part.as_str()) {
        ("partial_request" | "partial_response", "url" | "body") | ("redirect", _) => {
            ui.label("Find");
            ui.horizontal(|ui| {
                changed |= ui.add(egui::TextEdit::singleline(&mut rule.match_pattern).font(egui::TextStyle::Monospace).desired_width(220.0)).changed();
                changed |= ui.checkbox(&mut rule.is_regex, "regex").changed();
            });
            ui.end_row();
            let label = if rule.action_type == "redirect" { "Redirect to" } else { "Replace with" };
            changed |= text_row(ui, label, &mut rule.replacement_value);
            if rule.action_type == "redirect" {
                ui.label("Status");
                let mut status = rule.mock_status_code.unwrap_or(307);
                if ui.add(egui::DragValue::new(&mut status).range(300..=399)).changed() {
                    rule.mock_status_code = Some(status);
                    changed = true;
                }
                ui.end_row();
            }
        }
        (_, "header") => {
            ui.label("Value");
            changed |= ui.add(egui::TextEdit::singleline(&mut rule.replacement_value).hint_text("empty removes the header").desired_width(f32::INFINITY)).changed();
            ui.end_row();
        }
        ("partial_request", "query") => {
            changed |= text_row(ui, "Param", &mut rule.match_pattern);
            changed |= text_row(ui, "Value", &mut rule.replacement_value);
        }
        ("full_request", _) => changed |= text_row(ui, "New value", &mut rule.replacement_value),
        _ => {}
    }
    changed
}

fn text_row(ui: &mut egui::Ui, label: &str, value: &mut String) -> bool {
    ui.label(label);
    let changed = ui.add(egui::TextEdit::singleline(value).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY)).changed();
    ui.end_row();
    changed
}

fn combo(ui: &mut egui::Ui, id: &str, value: &mut String, options: &[(&str, &str)]) -> bool {
    let before = value.clone();
    let selected = options.iter().find(|(key, _)| *key == value).map_or(value.as_str(), |(_, label)| *label).to_string();
    egui::ComboBox::from_id_salt(("rewrite", id)).selected_text(RichText::new(selected)).show_ui(ui, |ui| {
        for (key, label) in options {
            ui.selectable_value(value, key.to_string(), *label);
        }
    });
    *value != before
}
