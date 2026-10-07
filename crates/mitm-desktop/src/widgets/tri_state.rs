use eframe::egui::{self, Color32, CornerRadius, RichText};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TriState {
    #[default]
    Neutral,
    Include,
    Exclude,
}

impl TriState {
    fn next(self) -> Self {
        match self {
            TriState::Neutral => TriState::Include,
            TriState::Include => TriState::Exclude,
            TriState::Exclude => TriState::Neutral,
        }
    }
}

pub fn tri_state_button(ui: &mut egui::Ui, label: &str, state: &mut TriState) -> bool {
    let (bg_color, stroke_color, text) = match state {
        TriState::Neutral => (
            Color32::from_rgb(31, 41, 55),
            Color32::from_rgb(75, 85, 99),
            RichText::new(label).color(Color32::from_rgb(156, 163, 175)).size(11.0),
        ),
        TriState::Include => (
            Color32::from_rgb(20, 83, 45),
            Color32::from_rgb(34, 197, 94),
            RichText::new(label).color(Color32::from_rgb(187, 247, 208)).size(11.0).strong(),
        ),
        TriState::Exclude => (
            Color32::from_rgb(127, 29, 29),
            Color32::from_rgb(239, 68, 68),
            RichText::new(label).color(Color32::from_rgb(254, 202, 202)).size(11.0).strikethrough().strong(),
        ),
    };

    let btn = egui::Button::new(text)
        .fill(bg_color)
        .stroke(egui::Stroke::new(1.0, stroke_color))
        .corner_radius(CornerRadius::same(3))
        .small();

    let clicked = ui.add(btn).clicked();
    if clicked {
        *state = state.next();
    }
    clicked
}
