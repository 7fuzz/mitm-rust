use eframe::egui::{self, Align, Layout};
use crate::widgets::template;
use super::method::method_combo;

const MAX_URL_LINES: f32 = 3.0;
const MIN_URL_WIDTH: f32 = 120.0;

pub struct RequestBar {
    pub method_changed: bool,
    pub url: egui::Response,
    pub send: bool,
}

/// Method, URL and Send on one top-aligned row, so the controls stay level with the URL's first line
/// as it grows.
pub fn show(ui: &mut egui::Ui, id_salt: &str, method: &mut String, url: &mut String, hint: &str, executing: bool) -> RequestBar {
    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
        let label = if executing { "Sending..." } else { "Send" };
        let button = crate::widgets::button::primary(ui, label);
        let send = ui.add_enabled(!executing, button).on_hover_text("Ctrl+Enter").clicked();
        let (method_changed, url) = ui
            .with_layout(Layout::left_to_right(Align::Min), |ui| {
                let method_changed = method_combo(ui, id_salt, method);
                let width = ui.available_width().max(MIN_URL_WIDTH);
                (method_changed, url_field(ui, url, hint, width))
            })
            .inner;
        RequestBar { method_changed, url, send }
    })
    .inner
}

/// One line tall, growing to three as the URL wraps, then scrolling. Line breaks are dropped, since a URL
/// has none; Enter neither adds one nor ends editing.
fn url_field(ui: &mut egui::Ui, url: &mut String, hint: &str, width: f32) -> egui::Response {
    let row = ui.text_style_height(&egui::TextStyle::Monospace);
    // Padding that makes a single line exactly as tall as the buttons beside it.
    let pad_y = ((ui.spacing().interact_size.y - row) / 2.0).max(1.0).round() as i8;
    let margin = egui::Margin::symmetric(4, pad_y);
    let mut layouter = template::layouter(egui::TextStyle::Monospace, None);
    let frame = egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.inactive.bg_stroke)
        .corner_radius(ui.visuals().widgets.inactive.corner_radius)
        .inner_margin(margin)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("url_field")
                .max_height(row * MAX_URL_LINES)
                .min_scrolled_height(0.0)
                .auto_shrink([true, true])
                .show(ui, |ui| {
                    template::show(ui, url, |text| {
                        egui::TextEdit::multiline(text)
                            .frame(egui::Frame::NONE)
                            .margin(egui::Margin::ZERO)
                            .desired_rows(1)
                            .return_key(None)
                            .hint_text(hint)
                            .layouter(&mut layouter)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(width - margin.sum().x)
                    })
                })
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
    if response.changed() && url.contains(['\n', '\r']) {
        url.retain(|c| c != '\n' && c != '\r');
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bar's height in a 600px panel, and the monospace row height.
    fn bar_height(url: &str) -> (f32, f32) {
        let ctx = egui::Context::default();
        let (mut method, mut url) = ("GET".to_string(), url.to_string());
        let mut measured = (0.0, 0.0);
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0))),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                let bar = ui.scope(|ui| show(ui, "m", &mut method, &mut url, "", false));
                measured = (bar.response.rect.height(), ui.text_style_height(&egui::TextStyle::Monospace));
            });
            output.textures_delta.clear();
        }
        measured
    }

    #[test]
    fn bar_is_one_row_until_the_url_wraps_then_stops_at_three_lines() {
        let (short, row) = bar_height("{{baseUrl}}/v2/api/auth/signin");
        assert!(short < row * 1.6, "a short URL makes the bar {short}px, a row is {row}px");
        let (long, row) = bar_height(&"{{baseUrl}}/v2/api/auth/signin?page=1&".repeat(20));
        assert!(long > row * 2.5 && long < row * 3.6, "a long URL makes the bar {long}px, a row is {row}px");
    }
}
