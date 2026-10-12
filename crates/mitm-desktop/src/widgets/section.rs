use eframe::egui::{self, RichText};

/// A small uppercase heading that groups what follows; replaces separator + bold-title pairs.
pub fn section(ui: &mut egui::Ui, title: &str) {
    section_with(ui, title, |_| {});
}

/// A section heading with trailing actions at its right edge.
pub fn section_with(ui: &mut egui::Ui, title: &str, actions: impl FnOnce(&mut egui::Ui)) {
    if ui.cursor().top() > ui.max_rect().top() + 1.0 {
        ui.add_space(12.0);
    }
    ui.horizontal(|ui| {
        let text = RichText::new(title.to_uppercase())
            .text_style(egui::TextStyle::Name("section".into()))
            .extra_letter_spacing(0.6)
            .color(ui.visuals().weak_text_color());
        ui.add(egui::Label::new(text).selectable(false));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), actions);
    });
    ui.add_space(6.0);
}
