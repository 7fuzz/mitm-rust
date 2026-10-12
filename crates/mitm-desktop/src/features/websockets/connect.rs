use eframe::egui;
use mitm_core::repeater::HeaderItem;
use crate::app::command::Commands;
use crate::features::request_editor::kv_table::{self, KvLabels};

#[derive(Default)]
pub struct WsConnect {
    open: bool,
    focus: bool,
    url: String,
    headers: Vec<HeaderItem>,
}

impl WsConnect {
    pub fn open(&mut self) {
        *self = Self { open: true, focus: true, url: "wss://".to_string(), ..Self::default() };
    }
}

pub fn show(ctx: &egui::Context, state: &mut WsConnect, cmds: &mut Commands) {
    if !state.open {
        return;
    }
    let mut close = false;
    let modal = egui::Modal::new(egui::Id::new("ws_connect")).show(ctx, |ui| {
        ui.set_width(480.0);
        ui.heading("Connect WebSocket");
        ui.add_space(6.0);
        let edit = ui.add(
            egui::TextEdit::singleline(&mut state.url)
                .hint_text("wss://")
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY),
        );
        if std::mem::take(&mut state.focus) {
            edit.request_focus();
        }
        let submitted = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

        ui.add_space(6.0);
        ui.label("Handshake headers");
        kv_table::show(ui, "ws_connect_headers", &mut state.headers, KvLabels { key: "Header", value: "Value" }, &[]);

        ui.add_space(8.0);
        let valid = is_ws_url(&state.url);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_enabled(valid, crate::widgets::button::primary(ui, "Connect")).clicked() || (valid && submitted) {
                cmds.push(super::Cmd::Connect { url: state.url.trim().to_string(), headers: enabled_headers(&state.headers) });
                close = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });
    if close || modal.should_close() {
        *state = WsConnect::default();
    }
}

fn is_ws_url(url: &str) -> bool {
    let url = url.trim();
    ["ws://", "wss://"].iter().any(|scheme| url.len() > scheme.len() && url.starts_with(scheme))
}

fn enabled_headers(headers: &[HeaderItem]) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| (h.key.trim().to_string(), h.value.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_ws_urls_with_a_host() {
        assert!(is_ws_url("wss://echo.example"));
        assert!(is_ws_url(" ws://localhost:8080/ws "));
        assert!(!is_ws_url("wss://"));
        assert!(!is_ws_url("https://example.com"));
    }
}
