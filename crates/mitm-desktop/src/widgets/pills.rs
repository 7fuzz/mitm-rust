use eframe::egui::{self, Color32, CornerRadius, RichText};

pub fn render_method_pill(ui: &mut egui::Ui, method: &str) {
    let (bg_color, text_color) = match method.to_uppercase().as_str() {
        "GET" => (Color32::from_rgb(22, 101, 52), Color32::from_rgb(187, 247, 208)),
        "POST" => (Color32::from_rgb(30, 64, 175), Color32::from_rgb(191, 219, 254)),
        "PUT" => (Color32::from_rgb(133, 77, 14), Color32::from_rgb(254, 240, 138)),
        "PATCH" => (Color32::from_rgb(107, 33, 168), Color32::from_rgb(233, 213, 255)),
        "DELETE" => (Color32::from_rgb(153, 27, 27), Color32::from_rgb(254, 202, 202)),
        "HEAD" | "OPTIONS" => (Color32::from_rgb(55, 65, 81), Color32::from_rgb(229, 231, 235)),
        _ => (Color32::from_rgb(55, 65, 81), Color32::from_rgb(209, 213, 219)),
    };

    let text = RichText::new(format!(" {} ", method.to_uppercase()))
        .color(text_color)
        .size(11.0)
        .strong();

    egui::Frame::NONE
        .fill(bg_color)
        .corner_radius(CornerRadius::same(3))
        .show(ui, |ui| {
            ui.add(egui::Label::new(text).selectable(false));
        });
}

pub fn render_status_pill(ui: &mut egui::Ui, status: u16) {
    let (bg_color, text_color) = match status {
        200..=299 => (Color32::from_rgb(20, 83, 45), Color32::from_rgb(187, 247, 208)),
        300..=399 => (Color32::from_rgb(29, 78, 216), Color32::from_rgb(191, 219, 254)),
        400..=499 => (Color32::from_rgb(146, 64, 14), Color32::from_rgb(254, 215, 170)),
        500..=599 => (Color32::from_rgb(153, 27, 27), Color32::from_rgb(254, 202, 202)),
        _ => (Color32::from_rgb(55, 65, 81), Color32::from_rgb(156, 163, 175)),
    };

    let text_str = if status == 0 {
        "-".to_string()
    } else {
        status.to_string()
    };

    let text = RichText::new(format!(" {} ", text_str))
        .color(text_color)
        .size(11.0)
        .strong();

    egui::Frame::NONE
        .fill(bg_color)
        .corner_radius(CornerRadius::same(3))
        .show(ui, |ui| {
            ui.add(egui::Label::new(text).selectable(false));
        });
}

pub fn render_source_pill(ui: &mut egui::Ui, source: &str) {
    let label = if source.is_empty() { "default" } else { source };
    let text = RichText::new(format!(" {} ", label))
        .color(Color32::from_rgb(199, 210, 254))
        .size(10.0)
        .monospace();

    egui::Frame::NONE
        .fill(Color32::from_rgb(49, 46, 129))
        .corner_radius(CornerRadius::same(3))
        .show(ui, |ui| {
            ui.add(egui::Label::new(text).selectable(false));
        });
}
