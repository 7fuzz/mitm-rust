use eframe::egui::{self, Color32, RichText};
use crate::format::{body_bytes, display_body, hex_dump};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneTab {
    Body,
    Headers,
    Params,
    Cookies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyFormat {
    Pretty,
    Raw,
    Hex,
}

pub struct PaneState {
    pub tab: PaneTab,
    pub format: BodyFormat,
    rendered: Option<((u64, usize, BodyFormat), String)>,
}

impl Default for PaneState {
    fn default() -> Self {
        Self {
            tab: PaneTab::Body,
            format: BodyFormat::Pretty,
            rendered: None,
        }
    }
}

pub struct PaneData<'a> {
    /// Identifies the body for the formatted-text cache; change it when the body source changes.
    pub cache_key: u64,
    pub title: &'a str,
    pub headers: &'a [(String, String)],
    pub body: &'a str,
    pub content_type: &'a str,
    pub params: Option<Vec<(String, String)>>,
    pub cookies: Option<Vec<(String, String)>>,
}

pub fn show(ui: &mut egui::Ui, id_salt: &str, pane: &mut PaneState, data: &PaneData) {
    ui.horizontal(|ui| {
        if !data.title.is_empty() {
            ui.label(RichText::new(data.title).strong());
            ui.separator();
        }
        tab_label(ui, pane, PaneTab::Body, "Body".to_string());
        tab_label(ui, pane, PaneTab::Headers, format!("Headers ({})", data.headers.len()));
        if let Some(params) = &data.params {
            tab_label(ui, pane, PaneTab::Params, format!("Params ({})", params.len()));
        }
        if let Some(cookies) = &data.cookies {
            tab_label(ui, pane, PaneTab::Cookies, format!("Cookies ({})", cookies.len()));
        }

        if pane.tab == PaneTab::Body {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for (format, label) in [(BodyFormat::Hex, "Hex"), (BodyFormat::Raw, "Raw"), (BodyFormat::Pretty, "Pretty")] {
                    ui.selectable_value(&mut pane.format, format, label);
                }
            });
        }
    });
    ui.separator();

    egui::ScrollArea::both()
        .id_salt(id_salt)
        .auto_shrink([false, false])
        .show(ui, |ui| match pane.tab {
            PaneTab::Body => body(ui, pane, data),
            PaneTab::Headers => key_values(ui, id_salt, data.headers),
            PaneTab::Params => key_values(ui, id_salt, data.params.as_deref().unwrap_or_default()),
            PaneTab::Cookies => key_values(ui, id_salt, data.cookies.as_deref().unwrap_or_default()),
        });
}

fn tab_label(ui: &mut egui::Ui, pane: &mut PaneState, tab: PaneTab, label: String) {
    ui.selectable_value(&mut pane.tab, tab, label);
}

fn body(ui: &mut egui::Ui, pane: &mut PaneState, data: &PaneData) {
    if data.body.is_empty() {
        empty(ui, "No body content");
        return;
    }
    let key = (data.cache_key, data.body.len(), pane.format);
    if pane.rendered.as_ref().is_none_or(|(cached, _)| *cached != key) {
        let text = match pane.format {
            BodyFormat::Pretty => display_body(data.body, data.content_type),
            BodyFormat::Raw => data.body.to_string(),
            BodyFormat::Hex => hex_dump(&body_bytes(data.body)),
        };
        pane.rendered = Some((key, text));
    }
    if let Some((_, text)) = &pane.rendered {
        ui.add(egui::Label::new(RichText::new(text.as_str()).monospace()).extend());
    }
}

fn key_values(ui: &mut egui::Ui, id_salt: &str, rows: &[(String, String)]) {
    if rows.is_empty() {
        empty(ui, "Nothing here");
        return;
    }
    egui::Grid::new((id_salt, "kv"))
        .striped(true)
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            for (key, value) in rows {
                ui.label(RichText::new(key).color(Color32::from_rgb(147, 197, 253)).strong());
                ui.label(RichText::new(value).monospace());
                ui.end_row();
            }
        });
}

fn empty(ui: &mut egui::Ui, text: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(text).weak().italics());
}
