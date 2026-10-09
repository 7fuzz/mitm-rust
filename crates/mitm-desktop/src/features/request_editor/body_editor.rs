use base64::Engine;
use eframe::egui::{self, RichText};
use mitm_core::encoding::MultipartFieldSpec;
use crate::util::format::format_bytes;
use crate::ui::theme::{self, Tone};
use crate::widgets::template;
use super::body::{is_multipart, is_urlencoded, read_multipart, read_urlencoded, store_multipart, store_urlencoded, text_field};
use super::kv_table::{self, KvLabels};

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
                    if ui.small_button(RichText::new("🗑").color(theme::color(ui, Tone::Danger))).clicked() {
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
    if ui.button("➕ Add Field").clicked() {
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
        None => ui.label(RichText::new("No file loaded").weak().italics()),
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
