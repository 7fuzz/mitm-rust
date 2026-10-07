use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use eframe::egui::{self, Color32, RichText};
use serde_json::Value;
use crate::format::{decode_base64, hex_dump, looks_like_base64};
use crate::theme::Tone;
use crate::widgets::flash;
use crate::widgets::search::search_field;

const ROOT: &str = "$";
const CHUNK: usize = 100;
const MAX_PREVIEW_CHARS: usize = 400;
const MAX_DECODED_TEXT_CHARS: usize = 64 * 1024;
const MAX_DECODED_HEX_BYTES: usize = 4096;

pub struct JsonTree {
    root: Value,
    open: HashSet<String>,
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
            open: HashSet::from([ROOT.to_string()]),
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
            if let Some(message) = flash::current(ui, flash_id) {
                ui.label(RichText::new(message).weak());
            }
            search_field(ui, &mut self.query, "Search keys and values");
        });
        self.refresh_hits();

        let mut view = View {
            open: &mut self.open,
            decoded: &mut self.decoded,
            hits: self.hits.as_ref(),
            matching_only: self.matching_only,
            copied: None,
        };
        egui::ScrollArea::both()
            .id_salt((id_salt, "json_tree"))
            .auto_shrink([false, false])
            .show(ui, |ui| view.node(ui, None, &self.root, ROOT, true));
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
            self.open.extend(hits.branches.iter().cloned());
        }
        self.hits = Some(hits);
    }
}

struct View<'a> {
    open: &'a mut HashSet<String>,
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

    fn node(&mut self, ui: &mut egui::Ui, key: Option<&str>, value: &Value, path: &str, filtered: bool) {
        if !self.is_visible(path, filtered) {
            return;
        }
        let matched = self.is_match(path);
        let children = match value {
            Value::Object(map) => map.len(),
            Value::Array(items) => items.len(),
            _ => {
                let response = ui.add(egui::Label::new(self.row(ui, key, value, matched, false)).sense(egui::Sense::click()));
                self.context_menu(&response, value, path);
                self.decoded_block(ui, path);
                return;
            }
        };

        let open = self.open.contains(path);
        let response = ui.add(egui::Label::new(self.row(ui, key, value, matched, open)).sense(egui::Sense::click()));
        if response.clicked() {
            toggle(self.open, path);
        }
        self.context_menu(&response, value, path);
        if !open {
            return;
        }
        let filtered = filtered && !matched;
        ui.indent(path, |ui| {
            if children <= CHUNK {
                self.children(ui, value, path, 0..children, filtered);
                return;
            }
            for start in (0..children).step_by(CHUNK) {
                let range = start..(start + CHUNK).min(children);
                let chunk = chunk_path(path, start);
                if !self.is_visible(&chunk, filtered) {
                    continue;
                }
                let open = self.open.contains(&chunk);
                let label = format!("{} [{} … {}]", arrow(open), range.start, range.end - 1);
                if ui.add(egui::Label::new(RichText::new(label).monospace().weak()).sense(egui::Sense::click())).clicked() {
                    toggle(self.open, &chunk);
                }
                if open {
                    ui.indent(&chunk, |ui| self.children(ui, value, path, range, filtered));
                }
            }
        });
    }

    fn children(&mut self, ui: &mut egui::Ui, value: &Value, path: &str, range: std::ops::Range<usize>, filtered: bool) {
        match value {
            Value::Object(map) => {
                for (key, child) in map.iter().skip(range.start).take(range.len()) {
                    self.node(ui, Some(key), child, &key_path(path, key), filtered);
                }
            }
            Value::Array(items) => {
                for (index, child) in items[range.clone()].iter().enumerate() {
                    let index = range.start + index;
                    self.node(ui, Some(&index.to_string()), child, &index_path(path, index), filtered);
                }
            }
            _ => {}
        }
    }

    fn row(&self, ui: &egui::Ui, key: Option<&str>, value: &Value, matched: bool, open: bool) -> egui::text::LayoutJob {
        let visuals = ui.visuals();
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let highlight = if matched { Tone::Warn.tint(visuals, 60) } else { Color32::TRANSPARENT };
        let mut job = egui::text::LayoutJob::default();
        let mut push = |text: &str, color: Color32, background: Color32| {
            job.append(text, 0.0, egui::TextFormat { font_id: font.clone(), color, background, ..Default::default() });
        };
        let is_branch = matches!(value, Value::Object(_) | Value::Array(_));
        push(if is_branch { arrow(open) } else { " " }, visuals.weak_text_color(), Color32::TRANSPARENT);
        push(" ", visuals.text_color(), Color32::TRANSPARENT);
        if let Some(key) = key {
            push(key, Tone::Info.color(visuals), highlight);
            push(": ", visuals.weak_text_color(), Color32::TRANSPARENT);
        }
        let (text, color) = match value {
            Value::Object(map) => (format!("{{{}}}", map.len()), visuals.weak_text_color()),
            Value::Array(items) => (format!("[{}]", items.len()), visuals.weak_text_color()),
            Value::String(s) => (format!("\"{}\"", truncate(s)), Tone::Success.color(visuals)),
            Value::Number(n) => (n.to_string(), Tone::Caution.color(visuals)),
            Value::Bool(b) => (b.to_string(), Tone::Accent.color(visuals)),
            Value::Null => ("null".to_string(), Tone::Accent.color(visuals)),
        };
        push(&text, color, if is_branch { Color32::TRANSPARENT } else { highlight });
        job
    }

    fn decoded_block(&mut self, ui: &mut egui::Ui, path: &str) {
        let Some(decoded) = self.decoded.get(path) else { return };
        let mut hide = false;
        ui.indent((path, "decoded"), |ui| {
            ui.horizontal_top(|ui| {
                hide = ui.small_button("✖").on_hover_text("Hide").clicked();
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
    let chunked = match value {
        Value::Object(map) => map.len() > CHUNK,
        Value::Array(items) => items.len() > CHUNK,
        _ => false,
    };
    let mut any = false;
    let mut visit = |index: usize, child_key: &str, child: &Value, child_path: String| {
        if collect_hits(hits, query, Some(child_key), child, &child_path) {
            any = true;
            if chunked {
                hits.branches.insert(chunk_path(path, index / CHUNK * CHUNK));
            }
        }
    };
    match value {
        Value::Object(map) => map.iter().enumerate().for_each(|(i, (k, v))| visit(i, k, v, key_path(path, k))),
        Value::Array(items) => items.iter().enumerate().for_each(|(i, v)| visit(i, &i.to_string(), v, index_path(path, i))),
        _ => {}
    }
    if any {
        hits.branches.insert(path.to_string());
    }
    any || hits.nodes.contains(path)
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

fn arrow(open: bool) -> &'static str {
    if open { "⏷" } else { "⏵" }
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

// `#` never appears in generated paths, so chunk ids can't collide with real nodes.
fn chunk_path(parent: &str, start: usize) -> String {
    format!("{}#{}", parent, start)
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
        tree.query = "ada".to_string();
        tree.refresh_hits();
        let hits = tree.hits.as_ref().unwrap();
        assert!(hits.nodes.contains("$.user.Name"));
        assert!(hits.nodes.contains("$.tags[1]"));
        assert!(!hits.nodes.contains("$.user.age"));
        assert!(hits.branches.contains("$.user") && hits.branches.contains("$.tags"));
        assert!(tree.open.contains("$.user"));
    }

    #[test]
    fn hits_open_the_chunk_holding_a_match() {
        let items: Vec<Value> = (0..250).map(|i| json!(i)).collect();
        let mut tree = JsonTree::new(Value::Array(items));
        tree.query = "201".to_string();
        tree.refresh_hits();
        assert!(tree.open.contains("$#200"));
        assert!(!tree.open.contains("$#0"));
    }

    #[test]
    fn decoded_bytes_are_sorted_into_image_text_or_binary() {
        let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        assert!(matches!(Decoded::new(png), Decoded::Image { .. }));
        assert!(matches!(Decoded::new(b"hello".to_vec()), Decoded::Text(t) if t == "hello"));
        assert!(matches!(Decoded::new(vec![0xff, 0xfe, 0x00]), Decoded::Binary(_)));
    }
}
