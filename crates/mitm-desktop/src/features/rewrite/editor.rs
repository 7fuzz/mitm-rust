use eframe::egui::{self, RichText};
use mitm_core::state::RewriteRule;
use mitm_core::repeater::HeaderItem;
use crate::features::request_editor::body_editor::code_editor;
use crate::features::request_editor::captured::header_items;
use crate::features::request_editor::kv_table::{self, KvLabels};
use mitm_core::proxy::rewrite::preview_rewrite;
use crate::ui::theme::{self, Tone};
use super::ACTIONS;
use crate::ui::icons;
use crate::widgets::section::section;

const MATCH_FIELDS: [(&str, &str); 6] =
    [("all", "All traffic"), ("url", "URL"), ("host", "Host"), ("path", "Path"), ("method", "Method"), ("header", "Header")];
const OPERATORS: [(&str, &str); 5] =
    [("contains", "contains"), ("equals", "equals"), ("starts_with", "starts with"), ("ends_with", "ends with"), ("regex", "regex")];

/// Edits one rule in place; the fields shown follow the action type. Returns whether anything changed.
pub fn show(ui: &mut egui::Ui, rule: &mut RewriteRule, mock_headers: &mut Vec<HeaderItem>) -> bool {
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
    if rule.action_type == "full_response" {
        changed |= mock_fields(ui, rule, mock_headers);
    }
    changed
}

fn mock_fields(ui: &mut egui::Ui, rule: &mut RewriteRule, headers: &mut Vec<HeaderItem>) -> bool {
    let mut changed = false;
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label("Status");
        let mut status = rule.mock_status_code.unwrap_or(200);
        if ui.add(egui::DragValue::new(&mut status).range(100..=599)).changed() {
            rule.mock_status_code = Some(status);
            changed = true;
        }
    });
    ui.add_space(4.0);
    section(ui, "Headers");
    let labels = KvLabels { key: "Header Name", value: "Header Value" };
    if kv_table::show(ui, "rewrite_mock_headers", headers, labels, &[]) {
        let pairs: Vec<(String, String)> =
            headers.iter().filter(|h| h.enabled && !h.key.trim().is_empty()).map(|h| (h.key.clone(), h.value.clone())).collect();
        rule.mock_headers_json = serde_json::to_string(&pairs).ok();
        changed = true;
    }
    ui.add_space(4.0);
    section(ui, "Body");
    ui.allocate_ui(egui::vec2(ui.available_width(), 220.0), |ui| {
        changed |= code_editor(ui, "rewrite_mock_body", rule.mock_body.get_or_insert_with(String::new));
    });
    changed
}

/// The editable rows for a rule's mock headers, read from the stored `[name, value]` pairs.
pub fn mock_header_rows(rule: &RewriteRule) -> Vec<HeaderItem> {
    let pairs: Vec<(String, String)> = rule.mock_headers_json.as_deref().and_then(|j| serde_json::from_str(j).ok()).unwrap_or_default();
    header_items(&pairs)
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

/// A sample of the rule's target, with the pattern's matches highlighted, and what the rule turns it into.
pub fn preview(ui: &mut egui::Ui, rule: &RewriteRule, sample: &mut String) {
    ui.add_space(8.0);
    section(ui, "Preview");
    let highlight_rule = rule.clone();
    let mut layouter = move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
        let text = buf.as_str();
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let plain = egui::text::TextFormat::simple(font.clone(), ui.visuals().text_color());
        let matched = egui::text::TextFormat {
            background: Tone::Warn.tint(ui.visuals(), 70),
            ..plain.clone()
        };
        let mut job = egui::text::LayoutJob::default();
        let mut at = 0;
        for range in preview_rewrite(&highlight_rule, text).map(|p| p.matches).unwrap_or_default() {
            job.append(&text[at..range.start], 0.0, plain.clone());
            job.append(&text[range.clone()], 0.0, matched.clone());
            at = range.end;
        }
        job.append(&text[at..], 0.0, plain);
        job.wrap.max_width = wrap_width;
        ui.fonts_mut(|f| f.layout_job(job))
    };
    ui.add(
        egui::TextEdit::multiline(sample)
            .hint_text("Sample URL, body or Name: value headers")
            .font(egui::TextStyle::Monospace)
            .layouter(&mut layouter)
            .desired_rows(3)
            .desired_width(f32::INFINITY),
    );
    ui.label(RichText::new(icons::CARET_DOWN).weak());
    match preview_rewrite(rule, sample) {
        Ok(result) => {
            let mut output = result.output.as_str();
            ui.add(egui::TextEdit::multiline(&mut output).font(egui::TextStyle::Monospace).desired_rows(3).desired_width(f32::INFINITY));
        }
        Err(error) => {
            ui.label(RichText::new(error).color(theme::color(ui, Tone::Danger)));
        }
    }
}
