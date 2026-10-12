use eframe::egui::{self, RichText};
use crate::ui::icons;

fn armed_id() -> egui::Id {
    egui::Id::new("search_focus_armed")
}

/// Ctrl+F: the dock arms this only while drawing the focused tab, so that tab's first search field takes focus.
pub fn arm(ctx: &egui::Context, armed: bool) {
    ctx.data_mut(|d| d.insert_temp(armed_id(), armed));
}

pub fn claim_focus(response: &egui::Response) {
    let armed = response.ctx.data_mut(|d| std::mem::take(d.get_temp_mut_or_default::<bool>(armed_id())));
    if armed {
        response.request_focus();
    }
}

pub fn search_field(ui: &mut egui::Ui, query: &mut String, hint: &str) -> egui::Response {
    search_field_sized(ui, query, hint, f32::INFINITY)
}

/// A `base` well with the search glyph and, once there's text, a clear button inside it.
pub fn search_field_sized(ui: &mut egui::Ui, query: &mut String, hint: &str, width: f32) -> egui::Response {
    let id = ui.id().with(("search_field", hint));
    let visuals = ui.visuals().clone();
    let focused = ui.memory(|m| m.has_focus(id));
    let stroke = if focused { visuals.selection.stroke } else { visuals.widgets.inactive.bg_stroke };
    let row = ui.text_style_height(&egui::TextStyle::Body);
    let pad_y = ((ui.spacing().interact_size.y - row) / 2.0).max(1.0).round() as i8;
    let width = if width.is_finite() { width } else { ui.available_width() };
    let mut clear = false;
    let frame = egui::Frame::new()
        .fill(visuals.extreme_bg_color)
        .stroke(stroke)
        .corner_radius(visuals.widgets.inactive.corner_radius)
        .inner_margin(egui::Margin::symmetric(6, 0));
    // The frame's margin includes its stroke; leaving that out made the field outgrow resizable panels.
    let content_width = (width - frame.total_margin().sum().x).max(0.0);
    let edit = frame
        .show(ui, |ui| {
            ui.set_width(content_width);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(RichText::new(icons::MAGNIFYING_GLASS).weak());
                let clear_width = if query.is_empty() { 0.0 } else { row + 4.0 };
                let edit = ui.add(
                    egui::TextEdit::singleline(query)
                        .id(id)
                        .hint_text(hint)
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::symmetric(0, pad_y))
                        .desired_width(ui.available_width() - clear_width),
                );
                claim_focus(&edit);
                if !query.is_empty() {
                    clear = ui.add(egui::Button::new(RichText::new(icons::X).weak()).frame(false)).on_hover_text("Clear").clicked();
                }
                edit
            })
            .inner
        })
        .inner;
    if clear {
        query.clear();
    }
    edit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_field_fits_the_width_it_is_given() {
        let ctx = egui::Context::default();
        crate::ui::theme::apply_style(&ctx);
        let mut query = "abc".to_string();
        for _ in 0..3 {
            let mut output = ctx.run_ui(Default::default(), |ui| {
                let available = ui.available_width();
                let response = search_field(ui, &mut query, "Filter");
                assert!(ui.min_rect().width() <= available + 0.01, "{} > {available}", ui.min_rect().width());
                let _ = response;
            });
            output.textures_delta.clear();
        }
    }
}
