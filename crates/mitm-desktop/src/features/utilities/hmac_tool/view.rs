use eframe::egui::{self, RichText};
use super::{HmacToolState, Provider};
use crate::ui::icons;
use crate::widgets::button::icon_button;

pub fn show(ui: &mut egui::Ui, state: &mut HmacToolState) {
    ui.horizontal(|ui| {
        ui.label("Provider");
        egui::ComboBox::from_id_salt("hmac_provider")
            .selected_text(state.prefs.provider.label())
            .show_ui(ui, |ui| {
                for provider in Provider::ALL {
                    ui.selectable_value(&mut state.prefs.provider, provider, provider.label());
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Secret");
        ui.add(egui::TextEdit::singleline(&mut state.secret).desired_width(f32::INFINITY));
    });
    ui.label("Payload");
    ui.add(egui::TextEdit::multiline(&mut state.payload).code_editor().desired_rows(6).desired_width(f32::INFINITY));

    ui.add_space(8.0);
    let (name, value) = state.signature().clone();
    ui.horizontal(|ui| {
        ui.label(RichText::new(&name).strong());
        if icon_button(ui, icons::COPY, "Copy").clicked() {
            ui.ctx().copy_text(value.clone());
        }
    });
    ui.add(egui::Label::new(RichText::new(&value).monospace()).wrap());
}
