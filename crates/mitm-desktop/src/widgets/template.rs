use std::ops::Range;
use std::sync::Arc;
use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{Color32, Galley, TextBuffer, TextStyle, Ui};
use crate::theme::Tone;

const MAX_HIGHLIGHT_BYTES: usize = 256 * 1024;

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
    let variable = TextFormat {
        font_id: font,
        color: Tone::Accent.color(visuals),
        background: Tone::Accent.tint(visuals, 30),
        ..Default::default()
    };
    let mut job = LayoutJob::default();
    if text.len() > MAX_HIGHLIGHT_BYTES {
        job.append(text, 0.0, plain);
        return job;
    }
    for (range, is_variable) in spans(text) {
        job.append(&text[range], 0.0, if is_variable { variable.clone() } else { plain.clone() });
    }
    job
}

fn spans(text: &str) -> Vec<(Range<usize>, bool)> {
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
            spans.push((plain_start..open, false));
        }
        spans.push((open..close + 2, true));
        plain_start = close + 2;
        cursor = plain_start;
    }
    if plain_start < text.len() || spans.is_empty() {
        spans.push((plain_start..text.len(), false));
    }
    spans
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
}
