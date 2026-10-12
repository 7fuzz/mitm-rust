use eframe::egui::{self, RichText};
use mitm_core::repeater::ExtractRuleItem;
use crate::ui::theme::{self, Tone};
use super::url_params::new_id;
use crate::ui::icons;

const MODES: [(&str, &str, &str); 6] = [
    ("json", "JSON Path", "data.token"),
    ("after_string", "After String", "token=||64"),
    ("before_string", "Before String", "&expires=||64"),
    ("between_string", "Between Strings", "token=\"||\""),
    ("header", "Header Value", "Authorization"),
    ("body_regex", "Regex Pattern", "token=([^;]+)"),
];

pub fn show(ui: &mut egui::Ui, id_salt: &str, rules: &mut Vec<ExtractRuleItem>) -> bool {
    let mut changed = false;
    let mut remove = None;

    for (index, rule) in rules.iter_mut().enumerate() {
        ui.push_id((id_salt, index), |ui| {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    changed |= ui.checkbox(&mut rule.enabled, "").changed();
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut rule.target_variable)
                                .hint_text("Target variable")
                                .font(egui::TextStyle::Monospace)
                                .desired_width(160.0),
                        )
                        .changed();
                    ui.label(RichText::new("← extract from").weak());
                    changed |= mode_combo(ui, rule);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(RichText::new(icons::TRASH).color(theme::color(ui, Tone::Danger))).on_hover_text("Delete rule").clicked() {
                            remove = Some(index);
                        }
                    });
                });
                changed |= expression_fields(ui, rule);
            });
        });
    }

    if let Some(index) = remove {
        rules.remove(index);
        changed = true;
    }
    if ui.button("Add Rule").on_hover_text("Fill an environment variable from each response").clicked() {
        rules.push(ExtractRuleItem {
            id: new_id(),
            r#type: "json".to_string(),
            expression: "data.token".to_string(),
            target_variable: "AUTH_TOKEN".to_string(),
            enabled: true,
        });
        changed = true;
    }
    changed
}

fn mode_combo(ui: &mut egui::Ui, rule: &mut ExtractRuleItem) -> bool {
    let current = MODES.iter().find(|(id, ..)| *id == rule.r#type).map_or("JSON Path", |(_, label, _)| *label);
    let mut picked = None;
    egui::ComboBox::from_id_salt("mode").selected_text(current).show_ui(ui, |ui| {
        for (id, label, default_expr) in MODES {
            if ui.selectable_label(rule.r#type == id, label).clicked() && rule.r#type != id {
                picked = Some((id, default_expr));
            }
        }
    });
    let Some((id, default_expr)) = picked else { return false };
    rule.r#type = id.to_string();
    rule.expression = default_expr.to_string();
    true
}

fn expression_fields(ui: &mut egui::Ui, rule: &mut ExtractRuleItem) -> bool {
    let (first, second) = rule.expression.split_once("||").unwrap_or((&rule.expression, ""));
    let (mut first, mut second) = (first.to_string(), second.to_string());
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.add_space(24.0);
        match rule.r#type.as_str() {
            "after_string" | "before_string" => {
                let hint = if rule.r#type == "after_string" { "Text after (e.g. token=)" } else { "Text before (e.g. &expires=)" };
                changed |= ui.add(field(&mut first, hint, (ui.available_width() - 150.0).max(80.0))).changed();
                ui.label(RichText::new("Max chars").weak());
                changed |= ui.add(field(&mut second, "256", 60.0)).changed();
            }
            "between_string" => {
                let half = ((ui.available_width() - 40.0) / 2.0).max(80.0);
                changed |= ui.add(field(&mut first, "Start delimiter", half)).changed();
                ui.label(RichText::new("and").weak());
                changed |= ui.add(field(&mut second, "End delimiter", half)).changed();
            }
            other => {
                let hint = match other {
                    "header" => "Response header name",
                    "body_regex" => "Regex with a capture group",
                    _ => "JSON path (e.g. data.user.token)",
                };
                let mut expression = rule.expression.clone();
                if ui.add(field(&mut expression, hint, ui.available_width())).changed() {
                    rule.expression = expression;
                    changed = true;
                }
                return;
            }
        }
        if changed {
            rule.expression = format!("{}||{}", first, second);
        }
    });
    changed
}

fn field<'a>(text: &'a mut String, hint: &'a str, width: f32) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(text)
        .hint_text(hint)
        .font(egui::TextStyle::Monospace)
        .desired_width(width.max(60.0))
}
