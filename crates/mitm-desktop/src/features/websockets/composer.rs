use eframe::egui::{self, Key, Modifiers, RichText};
use mitm_core::state::WebSocketConn;
use serde::{Deserialize, Serialize};
use crate::command::{AppCommand, Commands};
use crate::theme::{self, Tone};
use crate::widgets::code_view::{self, Lang};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FrameMode {
    #[default]
    Text,
    Json,
    Hex,
}

impl FrameMode {
    fn label(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Json => "JSON",
            Self::Hex => "Hex",
        }
    }

    fn msg_type(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Hex => "binary",
        }
    }

    fn check(self, payload: &str) -> Result<(), String> {
        match self {
            Self::Text => Ok(()),
            Self::Json => serde_json::from_str::<serde_json::Value>(payload).map(|_| ()).map_err(|e| e.to_string()),
            Self::Hex => {
                let digits: Vec<char> = payload.chars().filter(|c| !c.is_whitespace()).collect();
                if !digits.iter().all(char::is_ascii_hexdigit) {
                    Err("Not hex".to_string())
                } else if digits.len() % 2 != 0 {
                    Err("Odd number of hex digits".to_string())
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameTemplate {
    pub mode: FrameMode,
    pub payload: String,
}

pub struct Composer {
    text: String,
    mode: FrameMode,
    to_server: bool,
}

impl Default for Composer {
    fn default() -> Self {
        Self { text: String::new(), mode: FrameMode::Text, to_server: true }
    }
}

pub fn show(
    ui: &mut egui::Ui,
    composer: &mut Composer,
    templates: &mut Vec<FrameTemplate>,
    conn: &WebSocketConn,
    cmds: &mut Commands,
) {
    let edit_id = ui.make_persistent_id("ws_composer_edit");
    let hotkey = ui.memory(|m| m.has_focus(edit_id)) && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter));
    let check = composer.mode.check(&composer.text);
    let can_send = !composer.text.trim().is_empty() && check.is_ok();
    let mut send = hotkey && can_send;

    ui.horizontal(|ui| {
        for mode in [FrameMode::Text, FrameMode::Json, FrameMode::Hex] {
            ui.selectable_value(&mut composer.mode, mode, mode.label());
        }
        if !conn.is_client_session {
            ui.separator();
            ui.selectable_value(&mut composer.to_server, true, "→ Server");
            ui.selectable_value(&mut composer.to_server, false, "→ Client");
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = ui.add_enabled(can_send, egui::Button::new("Send")).on_hover_text("Ctrl+Enter");
            let button = match (&check, composer.text.trim().is_empty()) {
                (Err(e), false) => button.on_disabled_hover_text(e),
                _ => button,
            };
            send |= button.clicked();
            templates_menu(ui, composer, templates);
        });
    });

    let mut json_layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
        let mut job = code_view::layout_job(ui.style(), buf.as_str(), Lang::Json);
        job.wrap.max_width = wrap_width;
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let rows = ((ui.available_height() - 12.0) / row_height).max(3.0) as usize;
    egui::ScrollArea::vertical().id_salt("ws_composer_scroll").auto_shrink([false, false]).show(ui, |ui| {
        let mut edit = egui::TextEdit::multiline(&mut composer.text)
            .id(edit_id)
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .desired_rows(rows);
        if composer.mode == FrameMode::Json {
            edit = edit.layouter(&mut json_layouter);
        }
        ui.add(edit);
    });

    if send {
        cmds.push(AppCommand::SendWsFrame {
            connection_id: conn.connection_id,
            msg_type: composer.mode.msg_type().to_string(),
            payload: composer.text.clone(),
            to_server: conn.is_client_session || composer.to_server,
        });
    }
}

fn templates_menu(ui: &mut egui::Ui, composer: &mut Composer, templates: &mut Vec<FrameTemplate>) {
    ui.menu_button("Templates ▾", |ui| {
        let current = FrameTemplate { mode: composer.mode, payload: composer.text.clone() };
        let can_save = !current.payload.trim().is_empty() && !templates.contains(&current);
        if ui.add_enabled(can_save, egui::Button::new("Save current")).clicked() {
            templates.push(current);
            ui.close();
        }
        if templates.is_empty() {
            return;
        }
        ui.separator();
        let mut remove = None;
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            for (index, template) in templates.iter().enumerate() {
                ui.horizontal(|ui| {
                    let label = format!("{} · {}", template.mode.label(), template_name(&template.payload));
                    if ui.button(RichText::new(label).monospace()).clicked() {
                        composer.mode = template.mode;
                        composer.text = template.payload.clone();
                        ui.close();
                    }
                    if ui.small_button(RichText::new("🗑").color(theme::color(ui, Tone::Danger))).on_hover_text("Delete template").clicked() {
                        remove = Some(index);
                    }
                });
            }
        });
        if let Some(index) = remove {
            templates.remove(index);
        }
    });
}

fn template_name(payload: &str) -> String {
    let line = payload.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    if line.chars().count() > 40 {
        format!("{}…", line.chars().take(39).collect::<String>())
    } else {
        line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_validate_their_payloads() {
        assert!(FrameMode::Text.check("anything {").is_ok());
        assert!(FrameMode::Json.check(r#"{"a": 1}"#).is_ok());
        assert!(FrameMode::Json.check("{oops").is_err());
        assert!(FrameMode::Hex.check("de ad\nbe ef").is_ok());
        assert!(FrameMode::Hex.check("abc").is_err());
        assert!(FrameMode::Hex.check("zz").is_err());
    }

    #[test]
    fn template_name_is_first_non_empty_line() {
        assert_eq!(template_name("\n  {\"type\": \"ping\"}\n"), "{\"type\": \"ping\"}");
        assert_eq!(template_name(&"x".repeat(50)).chars().count(), 40);
    }
}
