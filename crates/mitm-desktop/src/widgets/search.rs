use eframe::egui;

pub fn search_field(ui: &mut egui::Ui, query: &mut String, hint: &str) {
    let has_text = !query.is_empty();
    let (_, clear) = egui::Sides::new().shrink_left().show(
        ui,
        |ui| ui.add(egui::TextEdit::singleline(query).hint_text(format!("🔍 {}", hint)).desired_width(f32::INFINITY)),
        |ui| has_text && ui.small_button("✖").on_hover_text("Clear").clicked(),
    );
    if clear {
        query.clear();
    }
}
