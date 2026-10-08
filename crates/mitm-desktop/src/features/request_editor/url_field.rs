use eframe::egui;
use crate::widgets::template;

const MAX_LINES: f32 = 3.0;
const MARGIN: f32 = 4.0;

/// The request URL: wraps and grows to three lines, then scrolls. Line breaks are dropped, since a URL
/// has none; Enter neither adds one nor ends editing.
pub fn url_field(ui: &mut egui::Ui, url: &mut String, hint: &str, width: f32) -> egui::Response {
    let row = ui.text_style_height(&egui::TextStyle::Monospace);
    let mut layouter = template::layouter(egui::TextStyle::Monospace, None);
    let frame = egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.inactive.bg_stroke)
        .corner_radius(ui.visuals().widgets.inactive.corner_radius)
        .inner_margin(MARGIN)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("url_field")
                .max_height(row * MAX_LINES)
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
                            .desired_width(width - 2.0 * MARGIN)
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
