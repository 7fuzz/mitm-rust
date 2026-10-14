//! A small native renderer for the common Mermaid diagram types, so `mermaid` code
//! fences in the preview become diagrams instead of raw code. Flowcharts and sequence
//! diagrams cover the bulk of real-world usage; anything else falls back to the source.

mod flowchart;
mod sequence;

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Stroke, Vec2};
use crate::ui::theme::{self, Palette, Tone};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Flowchart,
    Sequence,
    Unknown,
}

fn detect(code: &str) -> Kind {
    for raw in code.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        let head = line.split_whitespace().next().unwrap_or("");
        return match head {
            "sequenceDiagram" => Kind::Sequence,
            "graph" | "flowchart" => Kind::Flowchart,
            _ => Kind::Unknown,
        };
    }
    Kind::Unknown
}

pub fn render(ui: &mut egui::Ui, id: &str, code: &str) {
    let kind = detect(code);
    if kind == Kind::Unknown {
        fallback(ui, code);
        return;
    }
    let palette = Palette::of(ui.visuals());
    egui::Frame::new()
        .fill(palette.base)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            // Bound the scroll area to the pane; without this it expands to the diagram's
            // width, widening the whole preview instead of scrolling within it.
            let width = ui.available_width();
            egui::ScrollArea::horizontal().id_salt(id).max_width(width).show(ui, |ui| match kind {
                Kind::Flowchart => flowchart::render(ui, code),
                Kind::Sequence => sequence::render(ui, code),
                Kind::Unknown => {}
            });
        });
}

fn fallback(ui: &mut egui::Ui, code: &str) {
    egui::Frame::new()
        .fill(Palette::of(ui.visuals()).raised)
        .stroke(Stroke::new(1.0, Palette::of(ui.visuals()).border))
        .corner_radius(6.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.colored_label(theme::color(ui, Tone::Warn), "Unsupported Mermaid diagram");
            ui.add_space(4.0);
            ui.add(egui::Label::new(egui::RichText::new(code.trim_end()).monospace()).selectable(true));
        });
}

/// Colours and fonts the diagram painters share, resolved for the active theme.
struct Style {
    node_fill: Color32,
    node_stroke: Color32,
    text: Color32,
    line: Color32,
    label_bg: Color32,
    note_fill: Color32,
    font: FontId,
    small: FontId,
}

impl Style {
    fn of(ui: &egui::Ui) -> Self {
        let palette = Palette::of(ui.visuals());
        Self {
            node_fill: palette.raised,
            node_stroke: palette.border_strong,
            text: palette.text_strong,
            line: palette.text_weak,
            label_bg: palette.panel,
            note_fill: Tone::Caution.color(ui.visuals()).gamma_multiply(0.22),
            font: FontId::proportional(13.0),
            small: FontId::proportional(11.0),
        }
    }
}

fn text_size(ui: &egui::Ui, text: &str, font: &FontId) -> Vec2 {
    ui.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font.clone(), Color32::PLACEHOLDER)).size()
}

/// A filled triangle at `tip`, pointing along `dir` (a unit vector).
fn arrowhead(painter: &egui::Painter, tip: Pos2, dir: Vec2, color: Color32) {
    const LEN: f32 = 10.0;
    const HALF: f32 = 5.0;
    let perp = Vec2::new(-dir.y, dir.x);
    let base = tip - dir * LEN;
    let points = vec![tip, base + perp * HALF, base - perp * HALF];
    painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}

/// An open (unfilled) arrowhead, two strokes meeting at `tip`.
fn open_arrowhead(painter: &egui::Painter, tip: Pos2, dir: Vec2, stroke: Stroke) {
    const LEN: f32 = 10.0;
    const HALF: f32 = 5.0;
    let perp = Vec2::new(-dir.y, dir.x);
    let base = tip - dir * LEN;
    painter.line_segment([tip, base + perp * HALF], stroke);
    painter.line_segment([tip, base - perp * HALF], stroke);
}

/// A segment drawn as dashes.
fn dashed(painter: &egui::Painter, from: Pos2, to: Pos2, stroke: Stroke) {
    painter.add(egui::Shape::dashed_line(&[from, to], stroke, 6.0, 4.0));
}

/// Draws `text` centred at `pos` over a small filled chip, so it stays legible on top of lines.
fn chip_label(painter: &egui::Painter, pos: Pos2, text: &str, font: &FontId, style: &Style) {
    if text.is_empty() {
        return;
    }
    let size = painter.layout_no_wrap(text.to_owned(), font.clone(), style.text).size();
    let rect = egui::Rect::from_center_size(pos, size + Vec2::new(8.0, 2.0));
    painter.rect_filled(rect, 3.0, style.label_bg);
    painter.text(pos, Align2::CENTER_CENTER, text, font.clone(), style.text);
}
