use base64::Engine;
use eframe::egui::{self, RichText};
use mitm_core::http::body::MultipartFieldSpec;
use crate::util::format::format_bytes;
use crate::ui::theme::{self, Tone};
use crate::widgets::template;
use super::body::{convert, is_multipart, is_urlencoded, read_multipart, read_urlencoded, store_multipart, store_urlencoded, text_field};
use super::kv_table::{self, KvLabels};
use crate::ui::icons;
use crate::widgets::button::icon_button;

pub fn show(ui: &mut egui::Ui, id_salt: &str, body_type: &str, content: &mut String) -> bool {
    match body_type {
        "none" => {
            ui.weak("No body");
            false
        }
        t if is_urlencoded(t) || is_multipart(t) || t == "binary" => egui::ScrollArea::vertical()
            .id_salt((id_salt, "rows"))
            .auto_shrink([false, false])
            .show(ui, |ui| structured(ui, id_salt, body_type, content))
            .inner,
        _ => code_editor(ui, id_salt, content),
    }
}

fn structured(ui: &mut egui::Ui, id_salt: &str, body_type: &str, content: &mut String) -> bool {
    if is_urlencoded(body_type) {
        let mut rows = read_urlencoded(content);
        let changed = kv_table::show(ui, id_salt, &mut rows, KvLabels { key: "Key", value: "Value" }, &[]);
        if changed {
            *content = store_urlencoded(&rows);
        }
        changed
    } else if is_multipart(body_type) {
        let mut fields = read_multipart(content);
        let changed = multipart_table(ui, id_salt, &mut fields);
        if changed {
            *content = store_multipart(&fields);
        }
        changed
    } else {
        binary(ui, id_salt, content)
    }
}

/// Switches body type, converting the content only when asked. Otherwise each kind gets back the content it had
/// last this session, so a misclick loses nothing; JSON and raw are both text and keep theirs.
pub fn change_type(ctx: &egui::Context, owner: &str, from: &str, to: &str, content: &mut String, convert_content: bool) {
    let converted = convert_content.then(|| convert(from, to, content));
    let (from_kind, to_kind) = (content_kind(from), content_kind(to));
    if from_kind != to_kind {
        let stash = |kind: &str| egui::Id::new(("body_stash", owner, kind));
        ctx.data_mut(|d| {
            d.insert_temp(stash(from_kind), std::mem::take(content));
            *content = d.remove_temp::<String>(stash(to_kind)).unwrap_or_default();
        });
    }
    if let Some(converted) = converted {
        *content = converted;
    }
}

fn content_kind(body_type: &str) -> &'static str {
    match body_type {
        "json" | "raw" => "text",
        t if is_multipart(t) => "multipart",
        t if is_urlencoded(t) => "urlencoded",
        "binary" => "binary",
        _ => "none",
    }
}

/// The body type picked from the Convert menu; forms and text convert to each other, files and no body do not.
pub fn convert_menu(ui: &mut egui::Ui, body_type: &str, types: &[(&'static str, &'static str)]) -> Option<&'static str> {
    let convertible = |t: &str| matches!(t, "json" | "raw") || is_multipart(t) || is_urlencoded(t);
    if !convertible(body_type) {
        return None;
    }
    let mut picked = None;
    ui.menu_button(format!("Convert {}", icons::CARET_DOWN), |ui| {
        for (id, label) in types {
            if *id != body_type && convertible(id) && ui.button(format!("To {}", label)).clicked() {
                picked = Some(*id);
                ui.close();
            }
        }
    });
    picked
}

pub fn code_editor(ui: &mut egui::Ui, id_salt: &str, content: &mut String) -> bool {
    let visible = ui.available_size();
    egui::ScrollArea::vertical()
        .id_salt((id_salt, "code"))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut layouter = template::layouter(egui::TextStyle::Monospace, None);
            template::show(ui, content, |text| {
                egui::TextEdit::multiline(text)
                    .code_editor()
                    .layouter(&mut layouter)
                    .desired_width(f32::INFINITY)
                    .min_size(visible)
            })
            .changed()
        })
        .inner
}

fn multipart_table(ui: &mut egui::Ui, id_salt: &str, fields: &mut Vec<MultipartFieldSpec>) -> bool {
    let mut changed = false;
    let mut remove = None;

    for (index, field) in fields.iter_mut().enumerate() {
        ui.push_id((id_salt, index), |ui| {
            ui.horizontal(|ui| {
                changed |= ui.checkbox(&mut field.enabled, "").changed();
                changed |= ui
                    .add(egui::TextEdit::singleline(&mut field.key).hint_text("Key").desired_width(140.0))
                    .changed();

                let captured = field.r#type == "base64";
                if captured {
                    ui.label(RichText::new("File").weak());
                    let name = field.file_name.clone().unwrap_or_else(|| "file".to_string());
                    ui.label(RichText::new(format!("{} (captured)", name)).monospace());
                } else {
                    let before = field.r#type.clone();
                    egui::ComboBox::from_id_salt("kind")
                        .width(60.0)
                        .selected_text(if field.r#type == "file_path" { "File" } else { "Text" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut field.r#type, "text".to_string(), "Text");
                            ui.selectable_value(&mut field.r#type, "file_path".to_string(), "File");
                        });
                    changed |= field.r#type != before;
                    let hint = if field.r#type == "file_path" { "/path/to/file" } else { "Value" };
                    let width = (ui.available_width() - 28.0).max(80.0);
                    let mut layouter = template::singleline_layouter(egui::TextStyle::Body, None);
                    changed |= template::show(ui, &mut field.value, |text| {
                        egui::TextEdit::singleline(text).hint_text(hint).desired_width(width).layouter(&mut layouter)
                    })
                    .changed();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(ui, icons::TRASH, "Remove").clicked() {
                        remove = Some(index);
                    }
                });
            });
        });
    }

    if let Some(index) = remove {
        fields.remove(index);
        changed = true;
    }
    if ui.button("Add Field").clicked() {
        fields.push(text_field(String::new(), String::new()));
        changed = true;
    }
    changed
}

fn binary(ui: &mut egui::Ui, id_salt: &str, content: &mut String) -> bool {
    let size = content
        .strip_prefix("base64:")
        .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64.trim()).ok())
        .map(|bytes| bytes.len());
    match size {
        Some(n) => ui.label(format!("Binary body: {}", format_bytes(n as u64))),
        None => ui.label(RichText::new("No file loaded").weak()),
    };

    let path_id = ui.id().with((id_salt, "binary_path"));
    let error_id = ui.id().with((id_salt, "binary_error"));
    let mut path: String = ui.data_mut(|d| d.get_temp(path_id)).unwrap_or_default();
    let mut error: Option<String> = ui.data_mut(|d| d.get_temp(error_id)).flatten();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut path).hint_text("/path/to/file").desired_width(280.0));
        if ui.button("Load File").clicked() {
            match std::fs::read(path.trim()) {
                Ok(bytes) => {
                    *content = format!("base64:{}", base64::engine::general_purpose::STANDARD.encode(bytes));
                    error = None;
                    changed = true;
                }
                Err(e) => error = Some(format!("Failed to read {}: {}", path.trim(), e)),
            }
        }
        if size.is_some() && ui.button("Clear").clicked() {
            content.clear();
            changed = true;
        }
    });
    if let Some(message) = &error {
        ui.colored_label(theme::color(ui, Tone::Danger), message);
    }
    ui.data_mut(|d| {
        d.insert_temp(path_id, path);
        d.insert_temp(error_id, error);
    });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_type_swaps_content_instead_of_converting() {
        let ctx = egui::Context::default();
        let mut content = "{\"a\": \"1\"}".to_string();

        change_type(&ctx, "tab", "json", "form", &mut content, false);
        assert_eq!(content, "");
        change_type(&ctx, "tab", "form", "json", &mut content, false);
        assert_eq!(content, "{\"a\": \"1\"}");

        change_type(&ctx, "tab", "json", "raw", &mut content, false);
        assert_eq!(content, "{\"a\": \"1\"}");
    }

    #[test]
    fn converting_replaces_the_content_and_keeps_the_original_for_switching_back() {
        let ctx = egui::Context::default();
        let mut content = "{\"a\": \"1\"}".to_string();

        change_type(&ctx, "tab", "json", "form", &mut content, true);
        assert_eq!(read_multipart(&content).len(), 1);
        change_type(&ctx, "tab", "form", "json", &mut content, false);
        assert_eq!(content, "{\"a\": \"1\"}");
    }
}
