use std::cell::RefCell;
use std::ops::Range;
use std::sync::Arc;
use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{Color32, Galley, TextBuffer, TextStyle, Ui};
use mitm_core::workspace::Environment;
use crate::theme::Tone;

const MAX_HIGHLIGHT_BYTES: usize = 256 * 1024;

thread_local! {
    static ACTIVE_ENV: RefCell<Option<Environment>> = const { RefCell::new(None) };
}

pub fn set_active_env(env: Option<&Environment>) {
    ACTIVE_ENV.with(|cell| {
        *cell.borrow_mut() = env.cloned();
    });
}

#[allow(dead_code)]
pub fn get_active_env() -> Option<Environment> {
    ACTIVE_ENV.with(|cell| cell.borrow().clone())
}

pub fn resolve_var(name: &str) -> Option<String> {
    ACTIVE_ENV.with(|cell| {
        let env_ref = cell.borrow();
        mitm_core::workspace::variable_value_in_env(env_ref.as_ref(), name)
    })
}

fn is_secret(name: &str) -> bool {
    let name = name.trim();
    ACTIVE_ENV.with(|cell| {
        cell.borrow().as_ref().is_some_and(|env| {
            env.variables
                .iter()
                .any(|v| v.enabled && v.r#type == "secret" && v.key.trim().eq_ignore_ascii_case(name))
        })
    })
}

fn hover_value(name: &str) -> String {
    match resolve_var(name) {
        Some(_) if is_secret(name) => "••••••".to_string(),
        Some(value) => value,
        None => "undefined".to_string(),
    }
}

pub fn hover_tooltip(resp: egui::Response, text: &str) -> egui::Response {
    if !resp.hovered() || !text.contains("{{") {
        return resp;
    }
    let vars: Vec<String> = spans_with_names(text)
        .into_iter()
        .filter_map(|(_, opt)| opt)
        .collect();
    if vars.is_empty() {
        return resp;
    }

    let mut lines = Vec::new();
    for v in &vars {
        lines.push(format!("{{{{{}}}}}: {}", v, hover_value(v)));
    }
    resp.on_hover_text(lines.join("\n"))
}

pub fn layouter(style: TextStyle, color: Option<Color32>) -> impl FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<Galley> {
    move |ui, buf, wrap_width| {
        let mut job = layout_job(ui, buf.as_str(), &style, color);
        job.wrap.max_width = wrap_width;
        ui.fonts_mut(|f| f.layout_job(job))
    }
}

fn layout_job(ui: &Ui, text: &str, style: &TextStyle, color: Option<Color32>) -> LayoutJob {
    let visuals = ui.visuals();
    let font = style.resolve(ui.style());
    let plain = TextFormat::simple(
        font.clone(),
        color.or(visuals.override_text_color).unwrap_or_else(|| visuals.widgets.inactive.text_color()),
    );
    let mut job = LayoutJob::default();
    if text.len() > MAX_HIGHLIGHT_BYTES {
        job.append(text, 0.0, plain);
        return job;
    }
    for (range, var_name) in spans_with_names(text) {
        if let Some(name) = var_name {
            let is_known = resolve_var(&name).is_some();
            let tone = if is_known { Tone::Info } else { Tone::Danger };
            let variable = TextFormat {
                font_id: font.clone(),
                color: tone.color(visuals),
                background: tone.tint(visuals, 30),
                ..Default::default()
            };
            job.append(&text[range], 0.0, variable);
        } else {
            job.append(&text[range], 0.0, plain.clone());
        }
    }
    job
}

pub fn spans_with_names(text: &str) -> Vec<(Range<usize>, Option<String>)> {
    let mut spans = Vec::new();
    let mut plain_start = 0;
    let mut cursor = 0;
    while let Some(open) = text[cursor..].find("{{").map(|i| cursor + i) {
        let Some(close) = text[open + 2..].find("}}").map(|i| open + 2 + i) else { break };
        let name = &text[open + 2..close];
        if name.trim().is_empty() || name.contains(['{', '\n']) {
            cursor = open + 1;
            continue;
        }
        if plain_start < open {
            spans.push((plain_start..open, None));
        }
        let trimmed_name = name.trim().to_string();
        spans.push((open..close + 2, Some(trimmed_name)));
        plain_start = close + 2;
        cursor = plain_start;
    }
    if plain_start < text.len() || spans.is_empty() {
        spans.push((plain_start..text.len(), None));
    }
    spans
}

#[cfg(test)]
fn spans(text: &str) -> Vec<(Range<usize>, bool)> {
    spans_with_names(text)
        .into_iter()
        .map(|(r, opt)| (r, opt.is_some()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variables(text: &str) -> Vec<&str> {
        spans(text).into_iter().filter(|(_, v)| *v).map(|(r, _)| &text[r]).collect()
    }

    #[test]
    fn finds_variables_and_keeps_every_byte() {
        let text = "{{BASE_URL}}/users/{{ id }}?q=é";
        let all: String = spans(text).into_iter().map(|(r, _)| &text[r]).collect();
        assert_eq!(all, text);
        assert_eq!(variables(text), ["{{BASE_URL}}", "{{ id }}"]);
    }

    #[test]
    fn ignores_empty_unclosed_and_nested_braces() {
        assert!(variables("{{}} and {{  }} and {{open").is_empty());
        assert_eq!(variables("{{{{x}}"), ["{{x}}"]);
        assert!(variables("{{a\nb}}").is_empty());
    }

    #[test]
    fn test_known_and_unknown_variables() {
        let env = Environment {
            id: "e1".to_string(),
            workspace_id: "w1".to_string(),
            name: "Dev".to_string(),
            is_active: true,
            variables: vec![
                mitm_core::workspace::EnvironmentVariable {
                    key: "BASE_URL".to_string(),
                    value: "https://example.com".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
            ],
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        set_active_env(Some(&env));
        assert_eq!(resolve_var("BASE_URL").as_deref(), Some("https://example.com"));
        assert_eq!(resolve_var("MISSING"), None);
        assert!(resolve_var("$guid").is_some());
    }

    #[test]
    fn secret_values_are_masked_in_hovers() {
        let mut env = Environment {
            id: "e1".to_string(),
            workspace_id: "w1".to_string(),
            name: "Dev".to_string(),
            is_active: true,
            variables: vec![mitm_core::workspace::EnvironmentVariable {
                key: "TOKEN".to_string(),
                value: "hunter2".to_string(),
                enabled: true,
                r#type: "secret".to_string(),
                active_index: 0,
                variants: vec![],
            }],
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        set_active_env(Some(&env));
        assert_eq!(hover_value("token"), "••••••");
        env.variables[0].r#type = "default".to_string();
        set_active_env(Some(&env));
        assert_eq!(hover_value("TOKEN"), "hunter2");
        assert_eq!(hover_value("MISSING"), "undefined");
    }
}
