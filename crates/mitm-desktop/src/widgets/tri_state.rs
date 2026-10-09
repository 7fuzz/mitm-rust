use eframe::egui::{self, CornerRadius, RichText};
use crate::ui::theme::Tone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
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
    let visuals = ui.visuals();
    let (fill, stroke, text) = match state {
        TriState::Neutral => (
            visuals.widgets.inactive.weak_bg_fill,
            visuals.widgets.inactive.bg_stroke.color,
            RichText::new(label).color(visuals.weak_text_color()).size(11.0),
        ),
        TriState::Include => (
            Tone::Success.tint(visuals, 38),
            Tone::Success.color(visuals),
            RichText::new(label).color(Tone::Success.color(visuals)).size(11.0).strong(),
        ),
        TriState::Exclude => (
            Tone::Danger.tint(visuals, 38),
            Tone::Danger.color(visuals),
            RichText::new(label).color(Tone::Danger.color(visuals)).size(11.0).strikethrough().strong(),
        ),
    };

    let btn = egui::Button::new(text)
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .corner_radius(CornerRadius::same(3))
        .small();

    let clicked = ui.add(btn).clicked();
    if clicked {
        *state = state.next();
    }
    clicked
}
