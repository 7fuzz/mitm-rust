use std::sync::Arc;
use eframe::egui::{self, Galley, RichText};
use crate::ui::theme::{self, Tone};
use crate::util::format::{body_bytes, display_body, hex_dump};
use crate::widgets::code_view::{self, Lang};
use crate::widgets::form_view::{self, FormField};
use crate::widgets::json_tree::JsonTree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneTab {
    Body,
    Headers,
    Params,
    Cookies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BodyFormat {
    Form,
    Preview,
    Pretty,
    Tree,
    Raw,
    Hex,
}

/// A body's identity for caching: (hash, length).
type BodyKey = (u64, usize);

// Galleys depend on theme colours and zoom, so both are part of the cache key.
type RenderKey = (u64, usize, BodyFormat, bool, u32);

pub struct PaneState {
    pub tab: PaneTab,
    pub format: BodyFormat,
    rendered: Option<(RenderKey, Arc<Galley>)>,
    tree: Option<(BodyKey, Option<JsonTree>)>,
    image: Option<PaneImage>,
    form: Option<(BodyKey, Option<Vec<FormField>>)>,
    shown: Option<BodyKey>,
}

struct PaneImage {
    key: BodyKey,
    uri: String,
    bytes: egui::load::Bytes,
}

impl Default for PaneState {
    fn default() -> Self {
        Self {
            tab: PaneTab::Body,
            format: BodyFormat::Pretty,
            rendered: None,
            tree: None,
            image: None,
            form: None,
            shown: None,
        }
    }
}

pub struct PaneData<'a> {
    pub cache_key: u64,
    pub title: &'a str,
    pub headers: &'a [(String, String)],
    pub body: &'a str,
    pub content_type: &'a str,
    pub params: Option<Vec<(String, String)>>,
    pub cookies: Option<Vec<(String, String)>>,
    /// Reads multipart or url-encoded bodies for the Form view; `None` hides it.
    pub form_parser: Option<FormParser>,
}

pub type FormParser = fn(&str, &str) -> Option<Vec<FormField>>;

/// A body the user asked to save; the caller runs the save dialog.
#[derive(Clone)]
pub struct SaveBody {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

pub fn show(ui: &mut egui::Ui, id_salt: &str, pane: &mut PaneState, data: &PaneData) -> Option<SaveBody> {
    let mut save = None;
    let file_ext = (!data.body.is_empty()).then(|| file_extension(data.content_type)).flatten();
    let is_json = !data.body.is_empty() && code_view::detect(data.content_type, data.body) == Lang::Json;
    let is_image = !data.body.is_empty() && data.content_type.trim().to_lowercase().starts_with("image/");
    let body_key = (data.cache_key, data.body.len());
    if pane.image.as_ref().is_some_and(|image| image.key != body_key) {
        forget_image(ui.ctx(), pane);
    }
    if pane.form.as_ref().is_none_or(|(key, _)| *key != body_key) {
        let parsed = data.form_parser.and_then(|parse| parse(data.content_type, data.body));
        pane.form = Some((body_key, parsed));
    }
    let is_form = pane.form.as_ref().is_some_and(|(_, fields)| fields.is_some());
    if pane.shown != Some(body_key) {
        pane.shown = Some(body_key);
        pane.format = default_format(ui.ctx(), data.content_type, is_json, is_image, is_form);
    }
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
                if let Some(ext) = file_ext {
                    if ui.small_button("Save…").clicked() {
                        save = Some(SaveBody { file_name: format!("body.{ext}"), bytes: body_bytes(data.body) });
                    }
                    if ui.small_button("Open").on_hover_text("Open in the default app").clicked() {
                        open_externally(data, id_salt, ext);
                    }
                    ui.separator();
                }
                let choices = [
                    (BodyFormat::Hex, "Hex", true),
                    (BodyFormat::Raw, "Raw", true),
                    (BodyFormat::Pretty, "Pretty", true),
                    (BodyFormat::Tree, "Tree", is_json),
                    (BodyFormat::Preview, "Preview", is_image),
                    (BodyFormat::Form, "Form", is_form),
                ];
                for (format, label, available) in choices {
                    if available && ui.selectable_value(&mut pane.format, format, label).clicked() {
                        remember_format(ui.ctx(), data.content_type, format);
                    }
                }
            });
        }
    });
    ui.separator();

    if pane.tab == PaneTab::Body && pane.format == BodyFormat::Tree && is_json {
        tree(ui, id_salt, pane, data);
        return save;
    }
    if pane.tab == PaneTab::Body && pane.format == BodyFormat::Preview && is_image {
        image_preview(ui, id_salt, pane, data);
        return save;
    }
    if let Some((_, Some(fields))) = pane.form.as_ref().filter(|_| pane.tab == PaneTab::Body && pane.format == BodyFormat::Form) {
        return form_view::show(ui, id_salt, fields).or(save);
    }

    egui::ScrollArea::both()
        .id_salt(id_salt)
        .auto_shrink([false, false])
        .show(ui, |ui| match pane.tab {
            PaneTab::Body => body(ui, pane, data),
            PaneTab::Headers => key_values(ui, id_salt, data.headers),
            PaneTab::Params => key_values(ui, id_salt, data.params.as_deref().unwrap_or_default()),
            PaneTab::Cookies => key_values(ui, id_salt, data.cookies.as_deref().unwrap_or_default()),
        });
    save
}

type FormatChoices = std::collections::HashMap<String, BodyFormat>;

fn format_choices_id() -> egui::Id {
    egui::Id::new("body_format_by_mime")
}

fn mime_of(content_type: &str) -> String {
    content_type.split(';').next().unwrap_or_default().trim().to_lowercase()
}

fn remember_format(ctx: &egui::Context, content_type: &str, format: BodyFormat) {
    ctx.data_mut(|d| d.get_persisted_mut_or_default::<FormatChoices>(format_choices_id()).insert(mime_of(content_type), format));
}

/// The last explicit choice for this content type when it still applies, else the richest view the body supports.
fn default_format(ctx: &egui::Context, content_type: &str, is_json: bool, is_image: bool, is_form: bool) -> BodyFormat {
    let available = |format: BodyFormat| match format {
        BodyFormat::Tree => is_json,
        BodyFormat::Preview => is_image,
        BodyFormat::Form => is_form,
        _ => true,
    };
    let saved = ctx.data_mut(|d| d.get_persisted::<FormatChoices>(format_choices_id()))
        .and_then(|choices| choices.get(&mime_of(content_type)).copied())
        .filter(|&format| available(format));
    saved.unwrap_or(if is_image {
        BodyFormat::Preview
    } else if is_form {
        BodyFormat::Form
    } else if is_json {
        BodyFormat::Tree
    } else {
        BodyFormat::Pretty
    })
}

/// Bodies egui can't render (html, pdf, media) or that are worth keeping as files (images).
fn file_extension(content_type: &str) -> Option<&'static str> {
    let mime = content_type.split(';').next().unwrap_or_default().trim().to_lowercase();
    let ext = match mime.as_str() {
        "text/html" => "html",
        "application/pdf" => "pdf",
        "image/svg+xml" => "svg",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "audio/mpeg" => "mp3",
        "audio/ogg" => "ogg",
        "audio/wav" | "audio/x-wav" => "wav",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/quicktime" => "mov",
        m if m.starts_with("audio/") || m.starts_with("video/") || m.starts_with("image/") => "bin",
        _ => return None,
    };
    Some(ext)
}

fn open_externally(data: &PaneData, id_salt: &str, ext: &str) {
    let path = std::env::temp_dir().join(format!("mitm-{id_salt}-{}.{ext}", data.cache_key));
    if std::fs::write(&path, body_bytes(data.body)).is_ok() {
        let _ = crate::util::files::open_external(&path);
    }
}

fn tab_label(ui: &mut egui::Ui, pane: &mut PaneState, tab: PaneTab, label: String) {
    ui.selectable_value(&mut pane.tab, tab, label);
}

fn body(ui: &mut egui::Ui, pane: &mut PaneState, data: &PaneData) {
    if data.body.is_empty() {
        empty(ui, "No body content");
        return;
    }
    let key = (
        data.cache_key,
        data.body.len(),
        pane.format,
        ui.visuals().dark_mode,
        ui.ctx().pixels_per_point().to_bits(),
    );
    if pane.rendered.as_ref().is_none_or(|(cached, _)| *cached != key) {
        let (text, lang) = match pane.format {
            BodyFormat::Pretty | BodyFormat::Tree | BodyFormat::Preview | BodyFormat::Form => {
                let text = display_body(data.body, data.content_type);
                let lang = code_view::detect(data.content_type, &text);
                (text, lang)
            }
            BodyFormat::Raw => (data.body.to_string(), Lang::Plain),
            BodyFormat::Hex => (hex_dump(&body_bytes(data.body)), Lang::Plain),
        };
        let job = code_view::layout_job(ui.style(), &text, lang);
        pane.rendered = Some((key, ui.ctx().fonts_mut(|f| f.layout_job(job))));
    }
    if let Some((_, galley)) = &pane.rendered {
        ui.add(egui::Label::new(galley.clone()));
    }
}

fn image_preview(ui: &mut egui::Ui, id_salt: &str, pane: &mut PaneState, data: &PaneData) {
    let key = (data.cache_key, data.body.len());
    let image = pane.image.get_or_insert_with(|| {
        // The extension lets the loader pick svg; raster formats are sniffed from the bytes.
        let subtype = data.content_type.split(';').next().unwrap_or_default().trim().rsplit('/').next().unwrap_or("bin");
        let ext = subtype.split('+').next().unwrap_or(subtype);
        PaneImage {
            key,
            uri: format!("bytes://{id_salt}-{}-{}.{ext}", key.0, key.1),
            bytes: Arc::<[u8]>::from(body_bytes(data.body)).into(),
        }
    });
    let source = egui::ImageSource::Bytes { uri: image.uri.clone().into(), bytes: image.bytes.clone() };
    egui::ScrollArea::both().id_salt((id_salt, "preview")).auto_shrink([false, false]).show(ui, |ui| {
        ui.add(egui::Image::new(source).max_size(ui.available_size()).shrink_to_fit());
    });
}

// Drops the decoded texture as well as the bytes, so browsing images doesn't grow memory.
fn forget_image(ctx: &egui::Context, pane: &mut PaneState) {
    if let Some(image) = pane.image.take() {
        ctx.forget_image(&image.uri);
    }
}

fn tree(ui: &mut egui::Ui, id_salt: &str, pane: &mut PaneState, data: &PaneData) {
    let key = (data.cache_key, data.body.len());
    if pane.tree.as_ref().is_none_or(|(cached, _)| *cached != key) {
        if let Some((_, Some(old))) = &pane.tree {
            old.forget_images(ui.ctx());
        }
        let parsed = serde_json::from_str(data.body).ok().map(JsonTree::new);
        pane.tree = Some((key, parsed));
    }
    match pane.tree.as_mut().and_then(|(_, tree)| tree.as_mut()) {
        Some(tree) => tree.show(ui, id_salt),
        None => empty(ui, "Not valid JSON"),
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
                ui.label(RichText::new(key).color(theme::color(ui, Tone::Info)).strong());
                ui.label(RichText::new(value).monospace());
                ui.end_row();
            }
        });
}

fn empty(ui: &mut egui::Ui, text: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(text).weak().italics());
}

#[cfg(test)]
mod tests {
    use super::file_extension;

    #[test]
    fn only_bodies_egui_cannot_show_or_media_get_file_actions() {
        assert_eq!(file_extension("text/html; charset=utf-8"), Some("html"));
        assert_eq!(file_extension("application/PDF"), Some("pdf"));
        assert_eq!(file_extension("video/x-matroska"), Some("bin"));
        assert_eq!(file_extension("application/json"), None);
        assert_eq!(file_extension(""), None);
    }
}
