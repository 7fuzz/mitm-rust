use eframe::egui::{self, RichText};
use mitm_core::repeater::HeaderItem;
use crate::format::{format_bytes, CopyFormat, Exchange};
use crate::widgets::message_pane::{self, PaneData, PaneState};
use crate::widgets::pills::status_text;

pub struct ResponseData<'a> {
    pub cache_key: u64,
    pub status_code: u16,
    pub status_text: &'a str,
    pub duration_ms: u64,
    pub size: u64,
    pub headers: Vec<(String, String)>,
    pub body: &'a str,
}

pub struct RequestCopy {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl RequestCopy {
    fn actions(&self) -> Vec<(&'static str, String)> {
        let header_lines: Vec<String> = self.headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
        let mut raw = format!("{} {}\n{}", self.method, self.url, header_lines.join("\n"));
        if !self.body.is_empty() {
            raw.push_str(&format!("\n\n{}", self.body));
        }
        let curl_headers: String = header_lines.iter().map(|h| format!(" -H '{}'", h)).collect();
        let curl_body = if self.body.is_empty() {
            String::new()
        } else {
            format!(" -d '{}'", self.body.replace('\'', "'\\''"))
        };
        vec![
            ("Copy URL", self.url.clone()),
            ("Copy Headers", header_lines.join("\n")),
            ("Copy Body", self.body.clone()),
            ("Copy Raw Request", raw),
            ("Copy as cURL", format!("curl -X {} '{}'{}{}", self.method, self.url, curl_headers, curl_body)),
        ]
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
    request: Option<&RequestCopy>,
    exchange: Option<&Exchange>,
    toolbar: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.set_min_height(24.0);
        match &response {
            Some(r) => {
                ui.label(status_text(r.status_code));
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
            copy_menu(ui, id_salt, response.as_ref(), request, exchange);
            toolbar(ui);
        });
    });
    ui.separator();

    let Some(r) = response else {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(RichText::new("Send the request to see the response").weak().italics());
        });
        return;
    };
    let data = PaneData {
        cache_key: r.cache_key,
        title: "",
        headers: &r.headers,
        body: r.body,
        content_type: content_type(&r.headers),
        params: None,
        cookies: None,
    };
    message_pane::show(ui, id_salt, pane, &data);
}

fn content_type(headers: &[(String, String)]) -> &str {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map_or("", |(_, v)| v.as_str())
}

fn copy_menu(
    ui: &mut egui::Ui,
    id_salt: &str,
    response: Option<&ResponseData>,
    request: Option<&RequestCopy>,
    exchange: Option<&Exchange>,
) {
    let flash_id = ui.id().with((id_salt, "copied"));
    let now = ui.input(|i| i.time);
    let flash: Option<(String, f64)> = ui.data_mut(|d| d.get_temp(flash_id));
    let title = match &flash {
        Some((message, at)) if now - at < 1.8 => {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
            format!("✔ {}", message)
        }
        _ => "📋 Copy ⏷".to_string(),
    };

    let mut copied: Option<(&str, String)> = None;
    ui.menu_button(title, |ui| {
        if let Some(request) = request {
            section(ui, "REQUEST");
            for (label, text) in request.actions() {
                if ui.button(label).clicked() {
                    copied = Some((label, text));
                }
            }
            ui.separator();
        }

        section(ui, "RESPONSE");
        let status_line = response.map(|r| format!("HTTP/1.1 {} {}", r.status_code, r.status_text).trim().to_string());
        let header_text = response.map(|r| r.headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("\n"));
        let has_body = response.is_some_and(|r| !r.body.is_empty());
        if ui.add_enabled(has_body, egui::Button::new("Copy Body")).clicked() {
            copied = response.map(|r| ("Copy Body", r.body.to_string()));
        }
        if ui.add_enabled(response.is_some(), egui::Button::new("Copy Headers")).clicked() {
            copied = Some(("Copy Headers", format!("{}\n{}", status_line.clone().unwrap_or_default(), header_text.clone().unwrap_or_default())));
        }
        if ui.add_enabled(response.is_some(), egui::Button::new("Copy Full Response")).clicked() {
            let mut full = format!("{}\n{}", status_line.unwrap_or_default(), header_text.unwrap_or_default());
            if let Some(r) = response.filter(|r| !r.body.is_empty()) {
                full.push_str(&format!("\n\n{}", r.body));
            }
            copied = Some(("Copy Full Response", full));
        }

        ui.separator();
        section(ui, "REQUEST + RESPONSE");
        let mut formats = vec![
            ("Copy cURL and Response", CopyFormat::CurlAndResponse),
            ("Copy URL, Body, and Response", CopyFormat::UrlBodyAndResponse),
        ];
        if request.is_none() {
            formats.insert(0, ("Copy cURL", CopyFormat::Curl));
        }
        for (label, format) in formats {
            if ui.add_enabled(exchange.is_some(), egui::Button::new(label)).clicked() {
                copied = exchange.map(|e| (label, e.copy_text(format)));
            }
        }
    });

    if let Some((label, text)) = copied {
        ui.ctx().copy_text(text);
        let message = label.trim_start_matches("Copy ").to_string() + " copied";
        ui.data_mut(|d| d.insert_temp(flash_id, (message, now)));
        ui.close();
    }
}

fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).weak().size(10.0).strong());
}
