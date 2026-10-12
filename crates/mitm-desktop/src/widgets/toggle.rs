use eframe::egui;
use crate::ui::theme::Palette;

/// An on/off switch; clicking it flips `on` and marks the response changed.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let height = ui.spacing().interact_size.y * 0.75;
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(height * 1.8, height), egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let palette = Palette::of(ui.visuals());
        let t = ui.ctx().animate_bool(response.id, *on);
        let track = egui::Color32::from(egui::lerp(egui::Rgba::from(palette.border_strong)..=egui::Rgba::from(palette.accent), t));
        let radius = rect.height() / 2.0;
        ui.painter().rect_filled(rect, radius, track);
        let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        ui.painter().circle_filled(egui::pos2(x, rect.center().y), radius - 2.5, palette.panel);
    }
    response
}

/// A switch followed by its label; clicking either flips it.
pub fn labelled_toggle(ui: &mut egui::Ui, on: &mut bool, label: &str) -> egui::Response {
    ui.horizontal(|ui| {
        let mut response = toggle(ui, on);
        let text = ui.add(egui::Label::new(label).selectable(false).sense(egui::Sense::click()));
        if text.clicked() {
            *on = !*on;
            response.mark_changed();
        }
        response | text
    })
    .inner
}
