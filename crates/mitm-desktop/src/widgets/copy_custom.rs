use eframe::egui::containers::menu::{MenuConfig, SubMenuButton};
use eframe::egui::{self, PopupCloseBehavior};
use crate::format::CopyOptions;

/// A "Custom" submenu of what to include; returns the choices when its Copy button is pressed.
pub fn submenu(ui: &mut egui::Ui) -> Option<CopyOptions> {
    let id = egui::Id::new("copy_custom_options");
    let saved: CopyOptions = ui.data_mut(|d| d.get_persisted(id)).unwrap_or_default();
    let mut options = saved.clone();
    let mut copy = false;
    // Toggling a checkbox must not close the menu; only Copy or a click outside does.
    let config = MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside);
    SubMenuButton::new("Custom").config(config).ui(ui, |ui| {
        ui.checkbox(&mut options.request_headers, "Request headers");
        ui.checkbox(&mut options.request_body, "Request body");
        ui.separator();
        ui.checkbox(&mut options.response_status, "Response status");
        ui.checkbox(&mut options.status_text, "Status text");
        ui.checkbox(&mut options.response_headers, "Response headers");
        ui.checkbox(&mut options.response_body, "Response body");
        ui.separator();
        ui.checkbox(&mut options.pretty_json, "Pretty JSON");
        ui.separator();
        if ui.button("Copy").clicked() {
            copy = true;
            ui.close();
        }
    });
    if options != saved {
        ui.data_mut(|d| d.insert_persisted(id, options.clone()));
    }
    copy.then_some(options)
}
