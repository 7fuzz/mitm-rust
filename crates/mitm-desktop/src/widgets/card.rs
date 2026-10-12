use eframe::egui;
use crate::ui::theme::Palette;

/// A raised block for one self-contained item: a settings group, a rule, a credential, an endpoint.
pub fn card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    let palette = Palette::of(ui.visuals());
    egui::Frame::new()
        .fill(palette.raised)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(6)
        .inner_margin(10)
        .show(ui, add_contents)
}
