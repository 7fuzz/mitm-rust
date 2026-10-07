use eframe::egui::{self, RichText};
use crate::theme::{self, Tone};

// First click arms the button, second click confirms; moving the pointer away disarms it.
pub fn confirm_button(ui: &mut egui::Ui, id: egui::Id, idle: RichText, armed: &str, hover: &str) -> bool {
    let mut armed_now: bool = ui.data_mut(|d| d.get_temp(id)).unwrap_or(false);
    let label = if armed_now { RichText::new(armed).color(theme::color(ui, Tone::Danger)) } else { idle };
    let button = ui.small_button(label).on_hover_text(hover);
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
