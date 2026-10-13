use eframe::egui::containers::menu::{MenuConfig, SubMenuButton};
use eframe::egui::{self, PopupCloseBehavior, RichText};
use crate::ui::icons;
use crate::util::copy::{CopyAction, CopyFormat, CopyParts, CopySettings};
use crate::widgets::flash;

const SETTINGS_ID: &str = "copy_settings";

/// The copy actions every request/response view offers; returns the one chosen, with the saved Copy Options.
pub fn items(ui: &mut egui::Ui, has_response: bool) -> Option<CopyAction> {
    let mut chosen = None;
    let mut item = |ui: &mut egui::Ui, label: &str, format: CopyFormat, enabled: bool| {
        if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
            chosen = Some(format);
            ui.close();
        }
    };
    section(ui, "REQUEST + RESPONSE");
    item(ui, "Copy cURL and Response", CopyFormat::CurlAndResponse, has_response);
    item(ui, "Copy URL, Body, and Response", CopyFormat::UrlBodyAndResponse, has_response);
    ui.separator();
    section(ui, "REQUEST");
    item(ui, "Copy URL", CopyFormat::Url, true);
    item(ui, "Copy URL and Body", CopyFormat::UrlAndBody, true);
    item(ui, "Copy Headers", CopyFormat::RequestHeaders, true);
    item(ui, "Copy Body", CopyFormat::RequestBody, true);
    item(ui, "Copy cURL", CopyFormat::Curl, true);
    item(ui, "Copy Full Request", CopyFormat::Request, true);
    ui.separator();
    section(ui, "RESPONSE");
    item(ui, "Copy Body", CopyFormat::ResponseBody, has_response);
    item(ui, "Copy Headers", CopyFormat::ResponseHeaders, has_response);
    item(ui, "Copy Full Response", CopyFormat::Response, has_response);
    ui.separator();
    if let Some(parts) = custom(ui) {
        chosen = Some(CopyFormat::Custom(parts));
    }
    settings_menu(ui);
    let settings = ui.data_mut(|d| d.get_persisted(egui::Id::new(SETTINGS_ID))).unwrap_or_default();
    chosen.map(|format| CopyAction { format, settings })
}

/// A "Copy" toolbar menu; `add_contents` returns true once it copied, which flashes a confirmation.
pub fn button(ui: &mut egui::Ui, id_salt: &str, add_contents: impl FnOnce(&mut egui::Ui) -> bool) {
    let flash_id = ui.id().with((id_salt, "copied"));
    let title = match flash::current(ui, flash_id) {
        Some(message) => format!("{} {}", icons::CHECK, message),
        None => format!("Copy {}", icons::CARET_DOWN),
    };
    if ui.menu_button(title, add_contents).inner.unwrap_or(false) {
        flash::set(ui, flash_id, "Copied");
    }
}

fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).weak().small().strong());
}

fn settings_menu(ui: &mut egui::Ui) {
    let id = egui::Id::new(SETTINGS_ID);
    let saved: CopySettings = ui.data_mut(|d| d.get_persisted(id)).unwrap_or_default();
    let mut settings = saved.clone();
    let config = MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside);
    SubMenuButton::new("Copy Options").config(config).ui(ui, |ui| {
        ui.checkbox(&mut settings.markdown_fence, "Wrap in ``` for Markdown");
        ui.checkbox(&mut settings.pretty_json, "Pretty JSON");
        ui.checkbox(&mut settings.truncate, "Truncate long values");
        ui.add_enabled_ui(settings.truncate, |ui| {
            ui.horizontal(|ui| {
                ui.label("Keep");
                ui.add(egui::DragValue::new(&mut settings.max_chars).range(10..=100_000).suffix(" chars"));
            });
            ui.checkbox(&mut settings.show_truncated_size, "Show how much was cut");
        });
    });
    if settings != saved {
        ui.data_mut(|d| d.insert_persisted(id, settings));
    }
}

fn custom(ui: &mut egui::Ui) -> Option<CopyParts> {
    let id = egui::Id::new("copy_custom_options");
    let saved: CopyParts = ui.data_mut(|d| d.get_persisted(id)).unwrap_or_default();
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
