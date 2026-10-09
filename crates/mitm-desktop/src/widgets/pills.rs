use eframe::egui::{self, CornerRadius, RichText, Visuals};
use crate::ui::theme::{method_tone, status_tone, Tone};

pub fn render_method_pill(ui: &mut egui::Ui, method: &str) {
    pill(ui, |text| text.size(11.0).strong(), &method.to_uppercase(), method_tone(method));
}

pub fn render_status_pill(ui: &mut egui::Ui, status: u16) {
    let label = if status == 0 { "-".to_string() } else { status.to_string() };
    pill(ui, |text| text.size(11.0).strong(), &label, status_tone(status));
}

pub fn render_source_pill(ui: &mut egui::Ui, source: &str) {
    let label = if source.is_empty() { "default" } else { source };
    pill(ui, |text| text.size(10.0).monospace(), label, Tone::Source);
}

pub fn badge(ui: &mut egui::Ui, text: &str, tone: Tone) -> egui::Response {
    pill(ui, |text| text.size(10.0).strong(), text, tone)
}

fn pill(ui: &mut egui::Ui, style: impl FnOnce(RichText) -> RichText, label: &str, tone: Tone) -> egui::Response {
    let visuals = ui.visuals().clone();
    let text = style(RichText::new(format!(" {} ", label))).color(tone.color(&visuals));
    egui::Frame::NONE
        .fill(tone.tint(&visuals, 38))
        .stroke(egui::Stroke::new(1.0, tone.tint(&visuals, 90)))
        .corner_radius(CornerRadius::same(3))
        .show(ui, |ui| {
            ui.add(egui::Label::new(text).selectable(false));
        })
        .response
}

pub fn method_text(visuals: &Visuals, method: &str) -> RichText {
    RichText::new(method.to_uppercase())
        .monospace()
        .strong()
        .size(11.0)
        .color(method_tone(method).color(visuals))
}

pub fn status_text(visuals: &Visuals, code: u16) -> RichText {
    let label = if code == 0 { "ERR".to_string() } else { code.to_string() };
    RichText::new(label).monospace().strong().size(11.0).color(status_tone(code).color(visuals))
}
