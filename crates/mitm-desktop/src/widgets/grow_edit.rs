use eframe::egui;

/// A bordered box one line tall that grows with its text up to `max_lines`, then scrolls. `add_edit` gets the
/// width inside the border and adds a frameless, marginless `TextEdit`; the box draws the border and the focus
/// outline itself so the scroll area sits inside it.
pub fn show(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    style: egui::TextStyle,
    width: f32,
    max_lines: f32,
    add_edit: impl FnOnce(&mut egui::Ui, f32) -> egui::Response,
) -> egui::Response {
    let row = ui.text_style_height(&style);
    // Padding that makes a single line exactly as tall as the buttons beside it.
    let pad_y = ((ui.spacing().interact_size.y - row) / 2.0).max(1.0).round() as i8;
    let margin = egui::Margin::symmetric(4, pad_y);
    let frame = egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.inactive.bg_stroke)
        .corner_radius(ui.visuals().widgets.inactive.corner_radius)
        .inner_margin(margin)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt(id_salt)
                .max_height(row * max_lines)
                .min_scrolled_height(0.0)
                .auto_shrink([true, true])
                .show(ui, |ui| add_edit(ui, width - margin.sum().x))
                .inner
        });
    let response = frame.inner;
    if response.has_focus() {
        let visuals = ui.visuals();
        ui.painter().rect_stroke(
            frame.response.rect,
            visuals.widgets.inactive.corner_radius,
            visuals.selection.stroke,
            egui::StrokeKind::Inside,
        );
    }
    response
}
