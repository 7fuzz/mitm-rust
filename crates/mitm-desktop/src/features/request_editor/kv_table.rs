use eframe::egui::{self, RichText};
use mitm_core::http::body::UrlEncodedParamSpec;
use mitm_core::repeater::{HeaderItem, ParamItem};
use crate::ui::theme::{self, Tone};
use crate::widgets::template;
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
            let dim = !*enabled;
            let is_cookie = key.trim().eq_ignore_ascii_case("cookie");
            let expanded_id = ui.id().with("expanded");
            let mut expanded = ui.data(|d| d.get_temp(expanded_id).unwrap_or(false));
            let top_aligned = expanded && !is_cookie;
            let row = |ui: &mut egui::Ui| {
                changed |= ui.checkbox(enabled, "").changed();
                changed |= text_field(ui, key, labels.key, dim, key_width).changed();
                let long = !is_cookie && overflows(ui, value, (ui.available_width() - 28.0).max(80.0));
                let toggle = is_cookie || long;
                let reserved = if toggle { 56.0 } else { 28.0 };
                let value_width = (ui.available_width() - reserved).max(80.0);
                let field = if long && expanded {
                    wrapped_field(ui, value, labels.value, dim, value_width)
                } else {
                    text_field(ui, value, labels.value, dim, value_width)
                };
                changed |= field.changed();
                let hover = match (is_cookie, expanded) {
                    (true, _) => "Edit cookies",
                    (false, false) => "Expand",
                    (false, true) => "Collapse",
                };
                if toggle && ui.small_button(if expanded { "⏷" } else { "⏵" }).on_hover_text(hover).clicked() {
                    expanded = !expanded;
                    ui.data_mut(|d| d.insert_temp(expanded_id, expanded));
                }
                if ui.small_button(RichText::new("🗑").color(theme::color(ui, Tone::Danger))).on_hover_text("Delete row").clicked() {
                    remove = Some(index);
                }
            };
            if top_aligned {
                ui.horizontal_top(row);
            } else {
                ui.horizontal(row);
            }
            if expanded && is_cookie {
                changed |= cookie_rows(ui, value, dim, key_width);
            }
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

fn cookie_rows(ui: &mut egui::Ui, header: &mut String, dim: bool, key_width: f32) -> bool {
    let mut cookies = parse_cookies(header);
    let mut changed = false;
    let mut remove = None;
    let indent = ui.spacing().interact_size.y + ui.spacing().item_spacing.x;
    for (index, (name, value)) in cookies.iter_mut().enumerate() {
        ui.push_id(("cookie", index), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(indent);
                changed |= text_field(ui, name, "Name", dim, key_width - indent).changed();
                let value_width = (ui.available_width() - 56.0).max(80.0);
                changed |= text_field(ui, value, "Value", dim, value_width).changed();
                if ui.small_button(RichText::new("🗑").color(theme::color(ui, Tone::Danger))).on_hover_text("Delete cookie").clicked() {
                    remove = Some(index);
                }
            });
        });
    }
    ui.horizontal(|ui| {
        ui.add_space(indent);
        if ui.small_button("➕ Cookie").clicked() {
            cookies.push((String::new(), String::new()));
            changed = true;
        }
    });
    if let Some(index) = remove {
        cookies.remove(index);
        changed = true;
    }
    if changed {
        *header = join_cookies(&cookies);
    }
    changed
}

fn parse_cookies(header: &str) -> Vec<(String, String)> {
    header
        .split(';')
        .map(str::trim)
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (name.trim().to_string(), value.trim().to_string())
        })
        .collect()
}

fn join_cookies(cookies: &[(String, String)]) -> String {
    cookies.iter().map(|(name, value)| format!("{}={}", name, value)).collect::<Vec<_>>().join("; ")
}

/// Matches `TextEdit`'s default horizontal margin, which its desired width includes.
const TEXT_EDIT_MARGIN_X: f32 = 8.0;

fn overflows(ui: &egui::Ui, text: &str, field_width: f32) -> bool {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font, egui::Color32::PLACEHOLDER));
    galley.size().x > field_width - TEXT_EDIT_MARGIN_X
}

/// Header values are a single line, so Enter adds nothing and pasted line breaks are dropped.
fn wrapped_field(ui: &mut egui::Ui, text: &mut String, hint: &str, dim: bool, width: f32) -> egui::Response {
    let color = dim.then(|| ui.visuals().weak_text_color());
    let mut layouter = template::layouter(egui::TextStyle::Monospace, color);
    let response = template::show(ui, text, |text| {
        egui::TextEdit::multiline(text)
            .hint_text(hint)
            .desired_width(width)
            .desired_rows(1)
            .return_key(None)
            .font(egui::TextStyle::Monospace)
            .layouter(&mut layouter)
    });
    if response.changed() && text.contains(['\n', '\r']) {
        text.retain(|c| c != '\n' && c != '\r');
    }
    response
}

fn text_field(ui: &mut egui::Ui, text: &mut String, hint: &str, dim: bool, width: f32) -> egui::Response {
    let color = dim.then(|| ui.visuals().weak_text_color());
    let mut layouter = template::singleline_layouter(egui::TextStyle::Monospace, color);
    template::show(ui, text, |text| {
        egui::TextEdit::singleline(text).hint_text(hint).desired_width(width).font(egui::TextStyle::Monospace).layouter(&mut layouter)
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookies_round_trip_through_pairs() {
        let cookies = parse_cookies(" sid=abc; theme = dark ;flag; ");
        assert_eq!(cookies, [
            ("sid".to_string(), "abc".to_string()),
            ("theme".to_string(), "dark".to_string()),
            ("flag".to_string(), String::new()),
        ]);
        assert_eq!(join_cookies(&cookies), "sid=abc; theme=dark; flag=");
    }

    #[test]
    fn values_keep_embedded_equals_signs() {
        assert_eq!(parse_cookies("t=a=b=="), [("t".to_string(), "a=b==".to_string())]);
    }

    #[test]
    fn a_new_empty_cookie_survives_the_next_parse() {
        let header = join_cookies(&[("a".to_string(), "1".to_string()), (String::new(), String::new())]);
        assert_eq!(parse_cookies(&header).len(), 2);
    }
}
