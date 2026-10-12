use eframe::egui::{self, CornerRadius, RichText};
use crate::ui::theme::Palette;

/// Joined buttons for one choice among a few; the chosen one is filled.
pub fn segmented<T: PartialEq + Copy>(ui: &mut egui::Ui, current: &mut T, options: &[(T, &str)]) -> bool {
    let palette = Palette::of(ui.visuals());
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let last = options.len().saturating_sub(1);
        for (index, &(value, label)) in options.iter().enumerate() {
            let selected = *current == value;
            let radius = CornerRadius {
                nw: if index == 0 { 4 } else { 0 },
                sw: if index == 0 { 4 } else { 0 },
                ne: if index == last { 4 } else { 0 },
                se: if index == last { 4 } else { 0 },
            };
            let text = if selected { RichText::new(label).color(palette.text_strong) } else { RichText::new(label) };
            let mut button = egui::Button::new(text).corner_radius(radius);
            if selected {
                button = button.fill(palette.hover).stroke(egui::Stroke::new(1.0, palette.border_strong));
            }
            if ui.add(button).clicked() && !selected {
                *current = value;
                changed = true;
            }
        }
    });
    changed
}
