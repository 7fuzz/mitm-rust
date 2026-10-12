use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use eframe::egui::{self, Color32, RichText};
use serde_json::Value;
use crate::util::format::{decode_base64, hex_dump, looks_like_base64};
use crate::ui::theme::Tone;
use crate::widgets::flash;
use crate::widgets::search::search_field;
use crate::ui::icons;

const ROOT: &str = "$";
const ICON_WIDTH: f32 = 14.0;
const MAX_PREVIEW_CHARS: usize = 400;
const MAX_DECODED_TEXT_CHARS: usize = 64 * 1024;
const MAX_DECODED_HEX_BYTES: usize = 4096;

pub struct JsonTree {
    root: Value,
    collapsed: HashSet<String>,
    expanded_arrays: HashSet<String>,
    query: String,
    matching_only: bool,
    hits: Option<Hits>,
    decoded: HashMap<String, Decoded>,
}

enum Decoded {
    Text(String),
    Image { uri: String, bytes: Arc<[u8]> },
    Binary(String),
}

impl Decoded {
    fn new(bytes: Vec<u8>) -> Self {
        if image::guess_format(&bytes).is_ok_and(|format| format.reading_enabled()) {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            return Decoded::Image { uri: format!("bytes://json-tree/{:x}", hasher.finish()), bytes: bytes.into() };
        }
        match String::from_utf8(bytes) {
            Ok(text) => Decoded::Text(truncate_to(&text, MAX_DECODED_TEXT_CHARS)),
            Err(err) => {
                let bytes = err.into_bytes();
                Decoded::Binary(hex_dump(&bytes[..bytes.len().min(MAX_DECODED_HEX_BYTES)]))
            }
        }
    }

    fn forget(&self, ctx: &egui::Context) {
        if let Decoded::Image { uri, .. } = self {
            ctx.forget_image(uri);
        }
    }
}

#[derive(Default)]
struct Hits {
    query: String,
    nodes: HashSet<String>,
    branches: HashSet<String>,
}

impl JsonTree {
    pub fn new(root: Value) -> Self {
        Self {
            root,
            collapsed: HashSet::new(),
            expanded_arrays: HashSet::new(),
            query: String::new(),
            matching_only: false,
            hits: None,
            decoded: HashMap::new(),
        }
    }

    pub fn forget_images(&self, ctx: &egui::Context) {
        self.decoded.values().for_each(|decoded| decoded.forget(ctx));
    }

    pub fn show(&mut self, ui: &mut egui::Ui, id_salt: &str) {
        let flash_id = ui.id().with((id_salt, "json_tree_copied"));
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.matching_only, "Matching only");
            if ui.button("Reset view").clicked() {
                self.reset_view();
            }
            if let Some(message) = flash::current(ui, flash_id) {
                ui.label(RichText::new(message).weak());
            }
            search_field(ui, &mut self.query, "Search keys and values");
        });
        self.refresh_hits();

        let mut view = View {
            collapsed: &mut self.collapsed,
            expanded_arrays: &mut self.expanded_arrays,
            decoded: &mut self.decoded,
            hits: self.hits.as_ref(),
            matching_only: self.matching_only,
            copied: None,
        };
        egui::ScrollArea::both()
            .id_salt((id_salt, "json_tree"))
            .auto_shrink([false, false])
            .show(ui, |ui| view.node(ui, None, &self.root, ROOT, true, false));
        if let Some(message) = view.copied {
            flash::set(ui, flash_id, message);
        }
    }

    fn refresh_hits(&mut self) {
        let query = self.query.trim().to_lowercase();
        if self.hits.as_ref().is_some_and(|h| h.query == query) {
            return;
        }
        let mut hits = Hits { query, ..Default::default() };
        if !hits.query.is_empty() {
            let query = hits.query.clone();
            collect_hits(&mut hits, &query, None, &self.root, ROOT);
            for branch in &hits.branches {
                self.collapsed.remove(branch);
                self.expanded_arrays.insert(branch.clone());
            }
        }
        self.hits = Some(hits);
    }

    fn reset_view(&mut self) {
        self.collapsed.clear();
        self.expanded_arrays.clear();
    }
}

struct View<'a> {
    collapsed: &'a mut HashSet<String>,
    expanded_arrays: &'a mut HashSet<String>,
    decoded: &'a mut HashMap<String, Decoded>,
    hits: Option<&'a Hits>,
    matching_only: bool,
    copied: Option<String>,
}

impl View<'_> {
    fn searching(&self) -> Option<&Hits> {
        self.hits.filter(|h| !h.query.is_empty())
    }

    fn is_match(&self, path: &str) -> bool {
        self.searching().is_some_and(|h| h.nodes.contains(path))
    }

    fn is_visible(&self, path: &str, filtered: bool) -> bool {
        match self.searching() {
            Some(h) if filtered && self.matching_only => h.nodes.contains(path) || h.branches.contains(path),
            _ => true,
        }
    }

    fn node(&mut self, ui: &mut egui::Ui, key: Option<&str>, value: &Value, path: &str, filtered: bool, comma: bool) {
        if !self.is_visible(path, filtered) {
            return;
        }
        let matched = self.is_match(path);
        let weak = ui.visuals().weak_text_color();
        let mut job = Line::new(ui);
        if let Some(key) = key {
            job.push(&format!("\"{}\"", key), Tone::Info.color(ui.visuals()), matched);
            job.push(": ", weak, false);
        }

        let (open_bracket, close_bracket, len, unit) = match value {
            Value::Object(map) => ("{", "}", map.len(), "keys"),
            Value::Array(items) => ("[", "]", items.len(), "items"),
            _ => {
                let (text, tone) = leaf_display(value);
                job.push(&text, tone.color(ui.visuals()), matched);
                job.comma(comma, weak);
                let response = row(ui, None, job.0);
                self.context_menu(&response, value, path);
                self.decoded_block(ui, path);
                return;
            }
        };

        let collapsed = len == 0 || self.collapsed.contains(path);
        job.push(open_bracket, weak, false);
        if collapsed {
            if len > 0 {
                job.push(" … ", weak, false);
            }
            job.push(close_bracket, weak, false);
            job.comma(comma, weak);
            if len > 0 {
                job.push(&format!("  {} {}", len, unit), weak, false);
            }
        }
        let response = row(ui, (len > 0).then_some(!collapsed), job.0);
        if len > 0 && response.clicked() {
            toggle(self.collapsed, path);
        }
        self.context_menu(&response, value, path);
        if collapsed {
            return;
        }

        ui.indent(path, |ui| self.children(ui, value, path, filtered && !matched));
        let mut closing = Line::new(ui);
        closing.push(close_bracket, weak, false);
        closing.comma(comma, weak);
        row(ui, None, closing.0);
    }

    fn children(&mut self, ui: &mut egui::Ui, value: &Value, path: &str, filtered: bool) {
        let first_only = matches!(value, Value::Array(_)) && !self.expanded_arrays.contains(path);
        let limit = if first_only { 1 } else { usize::MAX };
        let mut shown = Vec::new();
        let total = if filtered && self.matching_only && self.searching().is_some() {
            let mut count = 0;
            for (key, child, child_path) in entries(value, path) {
                if self.is_visible(&child_path, true) {
                    if shown.len() < limit {
                        shown.push((key, child, child_path));
                    }
                    count += 1;
                }
            }
            count
        } else {
            shown.extend(entries(value, path).take(limit));
            match value {
                Value::Object(map) => map.len(),
                Value::Array(items) => items.len(),
                _ => 0,
            }
        };

        let shown_len = shown.len();
        for (index, (key, child, child_path)) in shown.into_iter().enumerate() {
            self.node(ui, key, child, &child_path, filtered, index + 1 < total);
        }
        if total > shown_len {
            let more = RichText::new(format!("+ {} more items", total - shown_len)).color(Tone::Warn.color(ui.visuals()));
            ui.horizontal(|ui| {
                ui.add_space(ICON_WIDTH);
                if ui.add(egui::Label::new(more).sense(egui::Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    self.expanded_arrays.insert(path.to_string());
                }
            });
        }
    }

    fn decoded_block(&mut self, ui: &mut egui::Ui, path: &str) {
        let Some(decoded) = self.decoded.get(path) else { return };
        let mut hide = false;
        ui.indent((path, "decoded"), |ui| {
            ui.horizontal_top(|ui| {
                hide = ui.small_button(icons::X).on_hover_text("Hide").clicked();
                match decoded {
                    Decoded::Text(text) => ui.label(RichText::new(text).monospace()),
                    Decoded::Image { uri, bytes } => {
                        ui.add(egui::Image::from_bytes(uri.clone(), bytes.clone()).max_height(240.0).fit_to_original_size(1.0))
                    }
                    Decoded::Binary(dump) => ui.label(RichText::new(dump).monospace().weak()),
                };
            });
        });
        if hide {
            self.hide_decoded(ui.ctx(), path);
        }
    }

    fn hide_decoded(&mut self, ctx: &egui::Context, path: &str) {
        if let Some(decoded) = self.decoded.remove(path) {
            decoded.forget(ctx);
        }
    }

    fn base64_menu(&mut self, ui: &mut egui::Ui, text: &str, path: &str) {
        if self.decoded.contains_key(path) {
            if ui.button("Hide decoded").clicked() {
                self.hide_decoded(ui.ctx(), path);
                ui.close();
            }
        } else if looks_like_base64(text) && ui.button("Decode base64").clicked() {
            match decode_base64(text) {
                Some(bytes) => {
                    self.decoded.insert(path.to_string(), Decoded::new(bytes));
                }
                None => self.copied = Some("Not base64".to_string()),
            }
            ui.close();
        }
    }

    fn context_menu(&mut self, response: &egui::Response, value: &Value, path: &str) {
        response.context_menu(|ui| {
            if let Value::String(text) = value {
                self.base64_menu(ui, text, path);
            }
            for label in ["Copy value", "Copy path", "Copy as JSON"] {
                if ui.button(label).clicked() {
                    let text = match label {
                        "Copy value" => leaf_text(value).unwrap_or_else(|| value.to_string()),
                        "Copy path" => path.to_string(),
                        _ => serde_json::to_string_pretty(value).unwrap_or_default(),
                    };
                    ui.ctx().copy_text(text);
                    self.copied = Some(label.trim_start_matches("Copy ").to_string() + " copied");
                    ui.close();
                }
            }
        });
    }
}

fn collect_hits(hits: &mut Hits, query: &str, key: Option<&str>, value: &Value, path: &str) -> bool {
    let matches = |text: &str| text.to_lowercase().contains(query);
    if key.is_some_and(matches) || leaf_text(value).is_some_and(|t| matches(&t)) {
        hits.nodes.insert(path.to_string());
    }
    let mut any = false;
    for (child_key, child, child_path) in entries(value, path) {
        any |= collect_hits(hits, query, child_key, child, &child_path);
    }
    if any {
        hits.branches.insert(path.to_string());
    }
    any || hits.nodes.contains(path)
}

fn entries<'a>(value: &'a Value, path: &'a str) -> Box<dyn Iterator<Item = (Option<&'a str>, &'a Value, String)> + 'a> {
    match value {
        Value::Object(map) => Box::new(map.iter().map(move |(k, v)| (Some(k.as_str()), v, key_path(path, k)))),
        Value::Array(items) => Box::new(items.iter().enumerate().map(move |(i, v)| (None, v, index_path(path, i)))),
        _ => Box::new(std::iter::empty()),
    }
}

struct Line(egui::text::LayoutJob, egui::FontId, Color32);

impl Line {
    fn new(ui: &egui::Ui) -> Self {
        Self(Default::default(), egui::TextStyle::Monospace.resolve(ui.style()), Tone::Warn.tint(ui.visuals(), 60))
    }

    fn push(&mut self, text: &str, color: Color32, highlighted: bool) {
        let background = if highlighted { self.2 } else { Color32::TRANSPARENT };
        self.0.append(text, 0.0, egui::TextFormat { font_id: self.1.clone(), color, background, ..Default::default() });
    }

    fn comma(&mut self, comma: bool, color: Color32) {
        if comma {
            self.push(",", color, false);
        }
    }
}

fn row(ui: &mut egui::Ui, arrow: Option<bool>, job: egui::text::LayoutJob) -> egui::Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let height = ui.text_style_height(&egui::TextStyle::Monospace);
        let (rect, icon) = ui.allocate_exact_size(egui::vec2(ICON_WIDTH, height), egui::Sense::click());
        if let Some(open) = arrow {
            let font = egui::TextStyle::Monospace.resolve(ui.style());
            let glyph = if open { icons::CARET_DOWN } else { icons::CARET_RIGHT };
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, glyph, font, ui.visuals().weak_text_color());
        }
        icon | ui.add(egui::Label::new(job).sense(egui::Sense::click()))
    })
    .inner
}

fn leaf_display(value: &Value) -> (String, Tone) {
    match value {
        Value::String(s) => (format!("\"{}\"", truncate(s)), Tone::Success),
        Value::Number(n) => (n.to_string(), Tone::Caution),
        Value::Bool(b) => (b.to_string(), Tone::Accent),
        _ => ("null".to_string(), Tone::Danger),
    }
}

fn leaf_text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Object(_) | Value::Array(_) => None,
        other => Some(other.to_string()),
    }
}

fn toggle(open: &mut HashSet<String>, path: &str) {
    if !open.remove(path) {
        open.insert(path.to_string());
    }
}

fn truncate(text: &str) -> String {
    truncate_to(text, MAX_PREVIEW_CHARS)
}

fn truncate_to(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_string(),
    }
}

fn key_path(parent: &str, key: &str) -> String {
    let mut chars = key.chars();
    let identifier = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if identifier {
        format!("{}.{}", parent, key)
    } else {
        format!("{}[{}]", parent, Value::String(key.to_string()))
    }
}

fn index_path(parent: &str, index: usize) -> String {
    format!("{}[{}]", parent, index)
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn paths_quote_non_identifier_keys() {
        assert_eq!(key_path("$", "name"), "$.name");
        assert_eq!(key_path("$", "content-type"), "$[\"content-type\"]");
        assert_eq!(key_path("$", "1x"), "$[\"1x\"]");
        assert_eq!(index_path("$.items", 3), "$.items[3]");
    }

    #[test]
    fn hits_mark_matches_and_their_ancestors() {
        let mut tree = JsonTree::new(json!({ "user": { "Name": "Ada", "age": 36 }, "tags": ["x", "ada"] }));
        tree.collapsed.insert("$.user".to_string());
        tree.query = "ada".to_string();
        tree.refresh_hits();
        let hits = tree.hits.as_ref().unwrap();
        assert!(hits.nodes.contains("$.user.Name"));
        assert!(hits.nodes.contains("$.tags[1]"));
        assert!(!hits.nodes.contains("$.user.age"));
        assert!(hits.branches.contains("$.user") && hits.branches.contains("$.tags"));
        assert!(!tree.collapsed.contains("$.user"));
        assert!(tree.expanded_arrays.contains("$.tags"));
    }

    #[test]
    fn array_indexes_are_not_searchable() {
        let mut tree = JsonTree::new(json!(["a", "b"]));
        tree.query = "1".to_string();
        tree.refresh_hits();
        assert!(tree.hits.unwrap().nodes.is_empty());
    }

    #[test]
    fn reset_view_returns_to_first_item_only() {
        let mut tree = JsonTree::new(json!({ "list": [1, 2, 3] }));
        tree.expanded_arrays.insert("$.list".to_string());
        tree.collapsed.insert("$".to_string());
        tree.reset_view();
        assert!(tree.expanded_arrays.is_empty() && tree.collapsed.is_empty());
    }

    #[test]
    fn decoded_bytes_are_sorted_into_image_text_or_binary() {
        let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        assert!(matches!(Decoded::new(png), Decoded::Image { .. }));
        assert!(matches!(Decoded::new(b"hello".to_vec()), Decoded::Text(t) if t == "hello"));
        assert!(matches!(Decoded::new(vec![0xff, 0xfe, 0x00]), Decoded::Binary(_)));
    }
}
