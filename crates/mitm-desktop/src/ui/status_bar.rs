use eframe::egui::{self, RichText};
use crate::app::state::AppState;
use crate::util::format::format_count;

pub fn show(ui: &mut egui::Ui, state: &AppState) {
    let proxy = &state.proxy;
    let mono = |text: String| RichText::new(text).small().family(egui::FontFamily::Monospace).weak();
    ui.horizontal_centered(|ui| {
        if proxy.running && !proxy.active_summary.is_empty() {
            ui.label(mono(proxy.active_summary.clone())).on_hover_text("Listening");
        } else {
            ui.label(RichText::new("Not listening").small().weak());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(mono(format!("{} in memory", format_count(state.traffic.flows.len()))))
                .on_hover_text("Flows loaded in the Traffic table");
        });
    });
}
