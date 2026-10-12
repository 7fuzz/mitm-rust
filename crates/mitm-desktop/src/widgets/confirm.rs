use eframe::egui::{self, RichText};
use super::button;

// First click arms the button, second click confirms; moving the pointer away disarms it.
pub fn confirm_button(ui: &mut egui::Ui, id: egui::Id, idle: RichText, armed: &str, hover: &str) -> bool {
    let mut armed_now: bool = ui.data_mut(|d| d.get_temp(id)).unwrap_or(false);
    let button = if armed_now {
        ui.add(button::danger_filled(ui, armed).small())
    } else {
        ui.small_button(idle)
    }
    .on_hover_text(hover);
    let mut confirmed = false;
    if button.clicked() {
        confirmed = armed_now;
        armed_now = !armed_now;
    } else if armed_now && !button.hovered() {
        armed_now = false;
    }
    ui.data_mut(|d| d.insert_temp(id, armed_now));
    confirmed
}
