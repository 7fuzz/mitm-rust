use eframe::egui::{self, RichText};
use crate::util::format::format_count;

/// One underline tab: the selected one in normal text over an accent line, the rest weak.
pub fn subtab(ui: &mut egui::Ui, selected: bool, label: &str, count: Option<usize>) -> egui::Response {
    let visuals = ui.visuals();
    let color = if selected { visuals.text_color() } else { visuals.weak_text_color() };
    let accent = visuals.selection.stroke.color;
    let label = RichText::new(label).color(color);
    let button = match count {
        Some(n) => egui::Button::new((label, RichText::new(format_count(n)).small().family(egui::FontFamily::Monospace).weak())),
        None => egui::Button::new(label),
    };
    let response = ui.add(button.frame(false));
    if selected {
        let rect = response.rect;
        ui.painter().hline(rect.x_range(), rect.bottom(), egui::Stroke::new(2.0, accent));
    }
    response
}

/// A row of underline tabs over values of `T`; returns true when the selection changed.
pub fn subtabs<T: PartialEq + Copy>(ui: &mut egui::Ui, current: &mut T, tabs: &[(T, &str, Option<usize>)]) -> bool {
    let mut changed = false;
    for &(value, label, count) in tabs {
        if subtab(ui, *current == value, label, count).clicked() && *current != value {
            *current = value;
            changed = true;
        }
    }
    changed
}
