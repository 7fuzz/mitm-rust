use std::collections::HashMap;
use eframe::egui::{self, Align2, Pos2, Rect, Sense, Stroke, Vec2};
use super::{arrowhead, dashed, open_arrowhead, text_size, Style};

struct Participant {
    label: String,
    x: f32,
    width: f32,
}

struct Msg {
    from: usize,
    to: usize,
    label: String,
    dashed: bool,
    filled: bool,
}

struct Note {
    from: usize,
    to: usize,
    text: String,
}

enum Event {
    Msg(Msg),
    Note(Note),
}

#[derive(Default)]
struct Seq {
    parts: Vec<Participant>,
    index: HashMap<String, usize>,
    events: Vec<Event>,
}

impl Seq {
    fn part(&mut self, id: &str) -> usize {
        let id = id.trim();
        if let Some(&i) = self.index.get(id) {
            return i;
        }
        let i = self.parts.len();
        self.index.insert(id.to_string(), i);
        self.parts.push(Participant { label: id.to_string(), x: 0.0, width: 0.0 });
        i
    }
}

const ARROWS: &[&str] = &["-->>", "->>", "-->", "--x", "--)", "->", "-x", "-)"];

fn parse(code: &str) -> Seq {
    let mut s = Seq::default();
    for raw in code.lines() {
        let line = strip_comment(raw).trim();
        if line.is_empty() || line == "sequenceDiagram" {
            continue;
        }
        let keyword = line.split_whitespace().next().unwrap_or("");
        match keyword {
            "participant" | "actor" => parse_participant(line, &mut s),
            "Note" | "note" => parse_note(line, &mut s),
            "loop" | "alt" | "opt" | "else" | "end" | "par" | "and" | "critical" | "rect" | "activate"
            | "deactivate" | "autonumber" | "box" => {}
            _ => parse_message(line, &mut s),
        }
    }
    s
}

fn strip_comment(line: &str) -> &str {
    line.find("%%").map_or(line, |at| &line[..at])
}

fn parse_participant(line: &str, s: &mut Seq) {
    let rest = line.splitn(2, char::is_whitespace).nth(1).unwrap_or("").trim();
    let (id, label) = match rest.split_once(" as ") {
        Some((id, label)) => (id.trim(), Some(label.trim().to_string())),
        None => (rest, None),
    };
    let i = s.part(id);
    if let Some(label) = label {
        s.parts[i].label = label;
    }
}

fn parse_note(line: &str, s: &mut Seq) {
    let Some((head, text)) = line.split_once(':') else { return };
    let text = text.trim().to_string();
    let words: Vec<&str> = head.split_whitespace().collect();
    // "Note over A,B" | "Note right of A" | "Note left of A"
    let targets = match words.get(1).copied() {
        Some("over") => words.get(2..).map(|w| w.join(" ")).unwrap_or_default(),
        Some("right") | Some("left") => words.get(3..).map(|w| w.join(" ")).unwrap_or_default(),
        _ => return,
    };
    let ids: Vec<usize> = targets.split(',').map(|id| s.part(id.trim())).collect();
    let Some(&from) = ids.first() else { return };
    let to = ids.last().copied().unwrap_or(from);
    s.events.push(Event::Note(Note { from: from.min(to), to: from.max(to), text }));
}

fn parse_message(line: &str, s: &mut Seq) {
    let (head, label) = match line.split_once(':') {
        Some((head, label)) => (head.trim(), label.trim().to_string()),
        None => (line, String::new()),
    };
    let Some((op, at)) = ARROWS.iter().find_map(|op| head.find(op).map(|at| (*op, at))) else { return };
    let from = head[..at].trim();
    let to = head[at + op.len()..].trim();
    if from.is_empty() || to.is_empty() {
        return;
    }
    let from = s.part(from);
    let to = s.part(to);
    s.events.push(Event::Msg(Msg {
        from,
        to,
        label,
        dashed: op.starts_with("--"),
        filled: op.ends_with(">>") || op.ends_with(')'),
    }));
}

const MARGIN: f32 = 16.0;
const BOX_H: f32 = 30.0;
const GAP: f32 = 80.0;
const ROW: f32 = 46.0;
const SELF_W: f32 = 36.0;

pub(super) fn render(ui: &mut egui::Ui, code: &str) {
    let mut s = parse(code);
    if s.parts.is_empty() {
        super::fallback(ui, code);
        return;
    }
    let style = Style::of(ui);

    let mut x = MARGIN;
    for i in 0..s.parts.len() {
        let w = (text_size(ui, &s.parts[i].label, &style.font).x + 24.0).max(70.0);
        s.parts[i].width = w;
        let half = w / 2.0;
        x += if i == 0 { half } else { half + GAP };
        s.parts[i].x = x;
        x += half;
    }

    let body_top = MARGIN + BOX_H + 18.0;
    let mut y = body_top;
    let heights: Vec<f32> = s
        .events
        .iter()
        .map(|e| match e {
            Event::Msg(m) if m.from == m.to => ROW + 14.0,
            Event::Msg(_) => ROW,
            Event::Note(n) => text_size(ui, &n.text, &style.small).y + 22.0,
        })
        .collect();
    let body_h: f32 = heights.iter().sum();
    let total_h = body_top + body_h + 14.0 + BOX_H + MARGIN;
    let total_w = x + MARGIN;

    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, total_h), Sense::hover());
    let origin = rect.min.to_vec2();
    let painter = ui.painter_at(rect);
    let life_bottom = total_h - MARGIN - BOX_H - 6.0;

    for p in &s.parts {
        let px = p.x;
        dashed(
            &painter,
            Pos2::new(px, MARGIN + BOX_H) + origin,
            Pos2::new(px, life_bottom) + origin,
            Stroke::new(1.0, style.line),
        );
        draw_box(&painter, p, MARGIN, origin, &style);
        draw_box(&painter, p, total_h - MARGIN - BOX_H, origin, &style);
    }

    for (event, h) in s.events.iter().zip(&heights) {
        match event {
            Event::Msg(m) => draw_message(&painter, &s.parts, m, y, origin, &style),
            Event::Note(n) => draw_note(&painter, &s.parts, n, y, *h, origin, &style),
        }
        y += h;
    }
}

fn draw_box(painter: &egui::Painter, p: &Participant, top: f32, origin: Vec2, style: &Style) {
    let rect = Rect::from_min_size(Pos2::new(p.x - p.width / 2.0, top), Vec2::new(p.width, BOX_H)).translate(origin);
    painter.rect(rect, 4.0, style.node_fill, Stroke::new(1.5, style.node_stroke), egui::StrokeKind::Inside);
    painter.text(rect.center(), Align2::CENTER_CENTER, &p.label, style.font.clone(), style.text);
}

fn draw_message(painter: &egui::Painter, parts: &[Participant], m: &Msg, y: f32, origin: Vec2, style: &Style) {
    let stroke = Stroke::new(1.3, style.line);
    if m.from == m.to {
        let x = parts[m.from].x;
        let p0 = Pos2::new(x, y) + origin;
        let p1 = Pos2::new(x + SELF_W, y) + origin;
        let p2 = Pos2::new(x + SELF_W, y + 18.0) + origin;
        let p3 = Pos2::new(x, y + 18.0) + origin;
        for seg in [[p0, p1], [p1, p2], [p2, p3]] {
            if m.dashed {
                dashed(painter, seg[0], seg[1], stroke);
            } else {
                painter.line_segment(seg, stroke);
            }
        }
        head(painter, p3, Vec2::new(-1.0, 0.0), m.filled, stroke, style);
        if !m.label.is_empty() {
            painter.text(Pos2::new(x + SELF_W + 6.0, y + 9.0) + origin, Align2::LEFT_CENTER, &m.label, style.small.clone(), style.text);
        }
        return;
    }
    let from_x = parts[m.from].x;
    let to_x = parts[m.to].x;
    let a = Pos2::new(from_x, y) + origin;
    let b = Pos2::new(to_x, y) + origin;
    if m.dashed {
        dashed(painter, a, b, stroke);
    } else {
        painter.line_segment([a, b], stroke);
    }
    let dir = Vec2::new((to_x - from_x).signum(), 0.0);
    head(painter, b, dir, m.filled, stroke, style);
    if !m.label.is_empty() {
        let mid = Pos2::new((from_x + to_x) / 2.0, y - 8.0) + origin;
        painter.text(mid, Align2::CENTER_BOTTOM, &m.label, style.small.clone(), style.text);
    }
}

fn head(painter: &egui::Painter, tip: Pos2, dir: Vec2, filled: bool, stroke: Stroke, style: &Style) {
    if filled {
        arrowhead(painter, tip, dir, style.line);
    } else {
        open_arrowhead(painter, tip, dir, stroke);
    }
}

fn draw_note(painter: &egui::Painter, parts: &[Participant], n: &Note, y: f32, h: f32, origin: Vec2, style: &Style) {
    let left = parts[n.from].x;
    let right = parts[n.to].x;
    let text_w = text_size_painter(painter, &n.text, style);
    let min_w = text_w + 24.0;
    let (cx, w) = if n.from == n.to {
        (left, min_w)
    } else {
        ((left + right) / 2.0, (right - left + parts[n.to].width).max(min_w))
    };
    let rect = Rect::from_center_size(Pos2::new(cx, y + h / 2.0), Vec2::new(w, h - 10.0)).translate(origin);
    painter.rect(rect, 3.0, style.note_fill, Stroke::new(1.0, style.node_stroke), egui::StrokeKind::Inside);
    painter.text(rect.center(), Align2::CENTER_CENTER, &n.text, style.small.clone(), style.text);
}

fn text_size_painter(painter: &egui::Painter, text: &str, style: &Style) -> f32 {
    painter.layout_no_wrap(text.to_owned(), style.small.clone(), style.text).size().x
}
