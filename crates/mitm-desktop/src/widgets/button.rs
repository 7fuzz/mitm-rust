use eframe::egui::{self, RichText};
use crate::ui::theme::{Palette, Tone};

/// The one main action of a panel (Send, Start, Apply, Create).
pub fn primary<'a>(ui: &egui::Ui, text: &str) -> egui::Button<'a> {
    let palette = Palette::of(ui.visuals());
    egui::Button::new(RichText::new(text).color(palette.accent_text).strong())
        .fill(palette.accent)
        .stroke(egui::Stroke::NONE)
}

/// A square, frameless toolbar action; the hover text says what it does.
pub fn icon_button(ui: &mut egui::Ui, icon: &str, hover: &str) -> egui::Response {
    let side = ui.spacing().interact_size.y;
    ui.add(egui::Button::new(icon).frame_when_inactive(false).min_size(egui::vec2(side, side))).on_hover_text(hover)
}

/// Filled in the danger colour, for the armed step of a destructive action.
pub fn danger_filled<'a>(ui: &egui::Ui, text: &str) -> egui::Button<'a> {
    let fill = Tone::Danger.color(ui.visuals());
    egui::Button::new(RichText::new(text).color(egui::Color32::WHITE).strong()).fill(fill).stroke(egui::Stroke::NONE)
}
