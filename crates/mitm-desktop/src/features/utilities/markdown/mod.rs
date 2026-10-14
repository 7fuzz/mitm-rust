mod mermaid;
mod view;

use serde::{Deserialize, Serialize};

pub use view::show;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Editor,
    Split,
    Preview,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct MarkdownPrefs {
    pub split: f32,
    pub view: ViewMode,
    pub stacked: bool,
    pub mermaid: bool,
}

impl Default for MarkdownPrefs {
    fn default() -> Self {
        Self { split: 0.5, view: ViewMode::Split, stacked: false, mermaid: true }
    }
}

#[derive(Default)]
pub struct MarkdownState {
    pub prefs: MarkdownPrefs,
    pub source: String,
    /// Remembered from the last opened file, used as the default name when saving.
    pub file_name: Option<String>,
}

impl MarkdownState {
    pub fn loaded(&mut self, name: String, content: String) {
        self.file_name = Some(name);
        self.source = content;
    }

    pub fn save_name(&self) -> String {
        self.file_name.clone().unwrap_or_else(|| "document.md".to_string())
    }
}

/// A run of the source: plain Markdown, or the body of a ```` ```mermaid ```` fence.
pub(crate) enum Segment<'a> {
    Markdown(&'a str),
    Mermaid(&'a str),
}

/// Splits the source so mermaid fences can be rendered as diagrams and the rest by the Markdown viewer.
pub(crate) fn segments(src: &str) -> Vec<Segment<'_>> {
    let lines = line_ranges(src);
    let mut out = Vec::new();
    let mut md_start = 0;
    let mut i = 0;
    while i < lines.len() {
        let (ls, le) = lines[i];
        if fence_lang(&src[ls..le]).as_deref() == Some("mermaid") {
            push_markdown(&mut out, &src[md_start..ls]);
            let code_start = lines.get(i + 1).map_or(le, |r| r.0);
            let mut j = i + 1;
            while j < lines.len() && fence_lang(&src[lines[j].0..lines[j].1]).as_deref() != Some("") {
                j += 1;
            }
            let code_end = if j > i + 1 { lines[j - 1].1 } else { code_start };
            out.push(Segment::Mermaid(&src[code_start..code_end]));
            md_start = lines.get(j).map_or(src.len(), |r| r.1);
            i = j;
        }
        i += 1;
    }
    push_markdown(&mut out, &src[md_start..]);
    out
}

fn push_markdown<'a>(out: &mut Vec<Segment<'a>>, text: &'a str) {
    if !text.trim().is_empty() {
        out.push(Segment::Markdown(text));
    }
}

fn line_ranges(src: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for (i, c) in src.char_indices() {
        if c == '\n' {
            ranges.push((start, i));
            start = i + 1;
        }
    }
    ranges.push((start, src.len()));
    ranges
}

/// The info string of a fence line (lowercased first word), or None if the line is not a fence.
/// A bare ```` ``` ```` returns `Some("")`, which marks a closing fence.
fn fence_lang(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("```")?;
    Some(rest.trim().split_whitespace().next().unwrap_or("").to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<&'static str> {
        segments(src)
            .iter()
            .map(|s| match s {
                Segment::Markdown(_) => "md",
                Segment::Mermaid(_) => "mermaid",
            })
            .collect()
    }

    #[test]
    fn extracts_mermaid_between_markdown() {
        let src = "# Title\n\n```mermaid\ngraph TD\nA-->B\n```\n\nAfter\n";
        assert_eq!(kinds(src), ["md", "mermaid", "md"]);
        let body = match &segments(src)[1] {
            Segment::Mermaid(code) => code.to_string(),
            _ => unreachable!(),
        };
        assert_eq!(body, "graph TD\nA-->B");
    }

    #[test]
    fn leaves_other_code_fences_in_markdown() {
        let src = "```rust\nfn main() {}\n```\n";
        assert_eq!(kinds(src), ["md"]);
    }

    #[test]
    fn unterminated_mermaid_takes_the_rest() {
        let src = "intro\n\n```mermaid\ngraph TD\nA-->B\n";
        assert_eq!(kinds(src), ["md", "mermaid"]);
    }

    /// Paints both diagram kinds in both themes, catching panics in the layout and painter code.
    #[test]
    fn mermaid_diagrams_render_in_both_themes() {
        let flow = "flowchart LR\n  A[Start] -->|go| B{Choice}\n  B --> C((End))\n  B -.no.-> A";
        let seq = "sequenceDiagram\n  participant A\n  participant B\n  A->>B: hi\n  B-->>A: bye\n  A->>A: think\n  Note over A,B: paired";
        let ctx = eframe::egui::Context::default();
        crate::ui::theme::install_fonts(&ctx);
        crate::ui::theme::apply_style(&ctx);
        let input = || eframe::egui::RawInput {
            screen_rect: Some(eframe::egui::Rect::from_min_size(eframe::egui::Pos2::ZERO, eframe::egui::vec2(1000.0, 700.0))),
            ..Default::default()
        };
        for theme in [eframe::egui::Theme::Dark, eframe::egui::Theme::Light] {
            ctx.set_theme(theme);
            for _ in 0..2 {
                let mut out = ctx.run_ui(input(), |ui| {
                    mermaid::render(ui, "flow", flow);
                    mermaid::render(ui, "seq", seq);
                    mermaid::render(ui, "bad", "pie title x");
                });
                out.textures_delta.clear();
            }
        }
    }
}
