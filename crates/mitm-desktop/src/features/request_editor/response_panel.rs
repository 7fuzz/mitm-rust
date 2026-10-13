use eframe::egui::{self, RichText};
use mitm_core::repeater::HeaderItem;
use crate::util::copy::Exchange;
use crate::util::format::format_bytes;
use crate::widgets::copy_menu;
use crate::widgets::message_pane::{self, PaneData, PaneState, SaveBody};
use crate::widgets::pills::status_text;
use crate::ui::icons;

pub struct ResponseData<'a> {
    pub cache_key: u64,
    pub status_code: u16,
    pub status_text: &'a str,
    pub duration_ms: u64,
    pub size: u64,
    pub headers: Vec<(String, String)>,
    pub body: &'a str,
}

/// Shows or hides the run history; sits in the response header's toolbar.
pub fn history_toggle(ui: &mut egui::Ui, open: &mut bool) {
    let hint = if *open { "Hide run history" } else { "Show run history" };
    if ui.selectable_label(*open, icons::CLOCK_COUNTER_CLOCKWISE).on_hover_text(hint).clicked() {
        *open = !*open;
    }
}

pub fn stack_toggle(ui: &mut egui::Ui, stacked: &mut bool) {
    let hint = if *stacked { "Put request beside response" } else { "Put request above response" };
    if ui.selectable_label(*stacked, icons::ROWS).on_hover_text(hint).clicked() {
        *stacked = !*stacked;
    }
}

pub fn header_pairs(headers: &[HeaderItem]) -> Vec<(String, String)> {
    headers.iter().map(|h| (h.key.clone(), h.value.clone())).collect()
}

pub fn show(
    ui: &mut egui::Ui,
    id_salt: &str,
    pane: &mut PaneState,
    response: Option<ResponseData>,
    editor: Option<&Exchange>,
    exchange: Option<&Exchange>,
    toolbar: impl FnOnce(&mut egui::Ui),
) -> Option<SaveBody> {
    ui.horizontal(|ui| {
        ui.set_min_height(24.0);
        match &response {
            Some(r) => {
                ui.label(status_text(ui.visuals(), r.status_code));
                if !r.status_text.is_empty() {
                    ui.label(RichText::new(r.status_text).strong());
                }
                ui.label(RichText::new(format!("{}ms · {}", r.duration_ms, format_bytes(r.size))).weak().monospace());
            }
            None => {
                ui.label(RichText::new("No response yet").weak());
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            copy_menu(ui, id_salt, editor, exchange);
            toolbar(ui);
        });
    });
    ui.add_space(4.0);

    let Some(r) = response else {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(RichText::new("Send the request to see the response").weak());
        });
        return None;
    };
    let data = PaneData {
        cache_key: r.cache_key,
        title: "",
        headers: &r.headers,
        body: r.body,
        content_type: content_type(&r.headers),
        params: None,
        cookies: None,
        form_parser: Some(super::body::captured_form),
    };
    message_pane::show(ui, id_salt, pane, &data)
}

fn content_type(headers: &[(String, String)]) -> &str {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map_or("", |(_, v)| v.as_str())
}

/// Copies the shown run; before the first run, or from the submenu, the request as it stands in the editor.
fn copy_menu(ui: &mut egui::Ui, id_salt: &str, editor: Option<&Exchange>, run: Option<&Exchange>) {
    copy_menu::button(ui, id_salt, |ui| {
        let copied = match (run, editor) {
            (Some(run), editor) => {
                let mut copied = copy_menu::items(ui, true).map(|action| run.copy_text(&action));
                if let Some(editor) = editor {
                    ui.separator();
                    if let Some(action) = ui.menu_button("Editor request", |ui| copy_menu::items(ui, false)).inner.flatten() {
                        copied = Some(editor.copy_text(&action));
                    }
                }
                copied
            }
            (None, Some(editor)) => copy_menu::items(ui, false).map(|action| editor.copy_text(&action)),
            (None, None) => {
                ui.add_enabled_ui(false, |ui| copy_menu::items(ui, false));
                None
            }
        };
        copied.map(|text| ui.ctx().copy_text(text)).is_some()
    });
}
