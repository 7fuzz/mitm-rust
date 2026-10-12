use eframe::egui::{self, CornerRadius, RichText, Visuals};
use crate::ui::theme::{method_tone, status_tone, Tone};

pub fn render_method_pill(ui: &mut egui::Ui, method: &str) {
    pill(ui, |text| text.small().strong(), &method.to_uppercase(), method_tone(method));
}

pub fn render_status_pill(ui: &mut egui::Ui, status: u16) {
    let label = if status == 0 { "-".to_string() } else { status.to_string() };
    pill(ui, |text| text.small().strong(), &label, status_tone(status));
}

pub fn render_source_pill(ui: &mut egui::Ui, source: &str) {
    let label = if source.is_empty() { "default" } else { source };
    pill(ui, |text| text.small().family(egui::FontFamily::Monospace), label, Tone::Source);
}

pub fn badge(ui: &mut egui::Ui, text: &str, tone: Tone) -> egui::Response {
    pill(ui, |text| text.small().strong(), text, tone)
}

fn pill(ui: &mut egui::Ui, style: impl FnOnce(RichText) -> RichText, label: &str, tone: Tone) -> egui::Response {
    let visuals = ui.visuals().clone();
    let text = style(RichText::new(label)).color(tone.color(&visuals));
    let (fill, stroke) = if visuals.dark_mode { (30, 70) } else { (24, 80) };
    egui::Frame::NONE
        .fill(tone.tint(&visuals, fill))
        .stroke(egui::Stroke::new(1.0, tone.tint(&visuals, stroke)))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.add(egui::Label::new(text).selectable(false));
        })
        .response
}

pub fn method_text(visuals: &Visuals, method: &str) -> RichText {
    RichText::new(method.to_uppercase())
        .small()
        .family(egui::FontFamily::Monospace)
        .strong()
        .color(method_tone(method).color(visuals))
}

pub fn status_text(visuals: &Visuals, code: u16) -> RichText {
    let label = if code == 0 { "ERR".to_string() } else { code.to_string() };
    RichText::new(label).small().family(egui::FontFamily::Monospace).strong().color(status_tone(code).color(visuals))
}
