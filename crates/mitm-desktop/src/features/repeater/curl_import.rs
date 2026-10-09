use eframe::egui::{self, RichText};
use mitm_core::http::curl::{parse_curl, ParsedCurl};
use crate::app::command::{AppCommand, Commands};
use crate::ui::theme::{self, Tone};

#[derive(Default)]
pub struct CurlImport {
    open: bool,
    focus: bool,
    text: String,
    parsed: Option<Result<ParsedCurl, String>>,
}

impl CurlImport {
    pub fn open(&mut self) {
        *self = Self { open: true, focus: true, ..Self::default() };
    }
}

pub fn show(ctx: &egui::Context, state: &mut CurlImport, cmds: &mut Commands) {
    if !state.open {
        return;
    }
    let mut close = false;
    let modal = egui::Modal::new(egui::Id::new("curl_import")).show(ctx, |ui| {
        ui.set_width(520.0);
        ui.heading("Import cURL");
        ui.add_space(6.0);
        let edit = ui.add(
            egui::TextEdit::multiline(&mut state.text)
                .hint_text("curl …")
                .font(egui::TextStyle::Monospace)
                .desired_rows(8)
                .desired_width(f32::INFINITY),
        );
        if std::mem::take(&mut state.focus) {
            edit.request_focus();
        }
        if edit.changed() {
            state.parsed = (!state.text.trim().is_empty()).then(|| parse_curl(state.text.trim()));
        }

        ui.add_space(6.0);
        match &state.parsed {
            Some(Ok(curl)) => {
                let summary = format!("{} {}", curl.method, curl.url);
                ui.add(egui::Label::new(RichText::new(summary).monospace().weak()).truncate());
            }
            Some(Err(e)) => {
                ui.label(RichText::new(e).color(theme::color(ui, Tone::Danger)));
            }
            None => {}
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let parsed = state.parsed.as_ref().and_then(|p| p.as_ref().ok());
            if ui.add_enabled(parsed.is_some(), egui::Button::new("Import")).clicked() {
                if let Some(curl) = parsed {
                    cmds.push(AppCommand::RepeaterImportCurl(curl.clone()));
                }
                close = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });
    if close || modal.should_close() {
        *state = CurlImport::default();
    }
}
