use eframe::egui::{self, RichText};
use mitm_core::encoding::UrlEncodedParamSpec;
use mitm_core::repeater::{HeaderItem, ParamItem};
use crate::widgets::pills::ROSE;
use super::url_params::new_id;

pub trait KeyValueRow {
    fn fields(&mut self) -> (&mut bool, &mut String, &mut String);
    fn create(key: String, value: String) -> Self;
}

macro_rules! key_value_row {
    ($ty:ty, $id:expr) => {
        impl KeyValueRow for $ty {
            fn fields(&mut self) -> (&mut bool, &mut String, &mut String) {
                (&mut self.enabled, &mut self.key, &mut self.value)
            }

            fn create(key: String, value: String) -> Self {
                Self { id: $id, key, value, enabled: true }
            }
        }
    };
}

key_value_row!(HeaderItem, new_id());
key_value_row!(ParamItem, new_id());
key_value_row!(UrlEncodedParamSpec, Some(new_id()));

pub struct KvLabels<'a> {
    pub key: &'a str,
    pub value: &'a str,
}

pub fn show<R: KeyValueRow>(
    ui: &mut egui::Ui,
    id_salt: &str,
    rows: &mut Vec<R>,
    labels: KvLabels,
    suggestions: &[(&str, &str)],
) -> bool {
    let mut changed = false;
    let mut remove = None;
    let key_width = (ui.available_width() * 0.35).clamp(120.0, 260.0);

    for (index, row) in rows.iter_mut().enumerate() {
        let (enabled, key, value) = row.fields();
        ui.push_id((id_salt, index), |ui| {
            ui.horizontal(|ui| {
                changed |= ui.checkbox(enabled, "").changed();
                let dim = !*enabled;
                changed |= ui
                    .add(text_field(key, labels.key, dim).desired_width(key_width))
                    .changed();
                let value_width = (ui.available_width() - 28.0).max(80.0);
                changed |= ui
                    .add(text_field(value, labels.value, dim).desired_width(value_width))
                    .changed();
                if ui.small_button(RichText::new("🗑").color(ROSE)).on_hover_text("Delete row").clicked() {
                    remove = Some(index);
                }
            });
        });
    }

    if let Some(index) = remove {
        rows.remove(index);
        changed = true;
    }

    ui.horizontal(|ui| {
        if ui.button("➕ Add Row").clicked() {
            rows.push(R::create(String::new(), String::new()));
            changed = true;
        }
        if !suggestions.is_empty() {
            ui.menu_button("Add Standard Header ⏷", |ui| {
                let existing: Vec<String> = rows.iter_mut().map(|r| r.fields().1.to_lowercase()).collect();
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    for (key, value) in suggestions {
                        let present = existing.contains(&key.to_lowercase());
                        if ui.add_enabled(!present, egui::Button::new(*key)).clicked() {
                            rows.push(R::create(key.to_string(), value.to_string()));
                            changed = true;
                            ui.close();
                        }
                    }
                });
            });
        }
    });

    changed
}

fn text_field<'a>(text: &'a mut String, hint: &'a str, dim: bool) -> egui::TextEdit<'a> {
    let edit = egui::TextEdit::singleline(text).hint_text(hint).font(egui::TextStyle::Monospace);
    if dim {
        edit.text_color(egui::Color32::GRAY)
    } else {
        edit
    }
}

pub fn standard_headers(content_type: &'static str) -> Vec<(&'static str, &'static str)> {
    vec![
        ("Accept", "*/*"),
        ("Accept-Encoding", "gzip, deflate, br, zstd"),
        ("Accept-Language", "en-US,en;q=0.9"),
        ("Authorization", "Bearer "),
        ("Cache-Control", "no-cache"),
        ("Connection", "keep-alive"),
        ("Content-Type", content_type),
        ("Cookie", ""),
        ("If-Modified-Since", ""),
        ("If-None-Match", ""),
        ("Origin", ""),
        ("Pragma", "no-cache"),
        ("Range", "bytes=0-"),
        ("Referer", ""),
        ("Sec-Ch-Ua", "\"Chromium\";v=\"124\", \"Google Chrome\";v=\"124\", \"Not-A.Brand\";v=\"99\""),
        ("Sec-Ch-Ua-Mobile", "?0"),
        ("Sec-Ch-Ua-Platform", "\"Windows\""),
        ("Sec-Fetch-Dest", "empty"),
        ("Sec-Fetch-Mode", "cors"),
        ("Sec-Fetch-Site", "same-site"),
        ("Upgrade-Insecure-Requests", "1"),
        ("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"),
        ("X-Forwarded-For", "127.0.0.1"),
        ("X-Requested-With", "XMLHttpRequest"),
    ]
}
