use eframe::egui::{self, RichText};

pub const METHODS: [&str; 7] = ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"];

pub fn method_combo(ui: &mut egui::Ui, id_salt: &str, method: &mut String) -> bool {
    let before = method.clone();
    egui::ComboBox::from_id_salt(id_salt)
        .width(86.0)
        .selected_text(RichText::new(method.as_str()).strong())
        .show_ui(ui, |ui| {
            for m in METHODS {
                ui.selectable_value(method, m.to_string(), m);
            }
        });
    *method != before
}
