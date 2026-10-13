use eframe::egui;

/// A full-width clickable row: hover fill, and an accent bar plus tint when selected.
pub fn list_row(ui: &mut egui::Ui, selected: bool, add_contents: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    list_row_sensing(ui, selected, egui::Sense::click(), add_contents)
}

/// `list_row` with its own sense, e.g. `click_and_drag` for rows that can be reordered.
pub fn list_row_sensing(ui: &mut egui::Ui, selected: bool, sense: egui::Sense, add_contents: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    let visuals = ui.visuals().clone();
    let accent = visuals.selection.stroke.color;
    let height = ui.spacing().interact_size.y + 4.0;
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), sense);
    if selected {
        ui.painter().rect_filled(rect, 4, accent.gamma_multiply(0.18));
        ui.painter().vline(rect.left() + 1.0, rect.y_range().shrink(4.0), egui::Stroke::new(2.0, accent));
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 4, visuals.widgets.hovered.weak_bg_fill);
    }
    let inner = rect.shrink2(egui::vec2(10.0, 0.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Center)));
    add_contents(&mut child);
    response
}
