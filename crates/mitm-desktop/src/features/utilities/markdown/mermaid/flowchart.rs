use std::collections::HashMap;
use eframe::egui::{self, Align2, Pos2, Rect, Sense, Stroke, Vec2};
use super::{arrowhead, chip_label, dashed, text_size, Style};

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Rect,
    Rounded,
    Stadium,
    Circle,
    Diamond,
    Hexagon,
}

struct Node {
    label: String,
    shape: Shape,
    rank: usize,
    rect: Rect,
}

#[derive(Clone, Copy, PartialEq)]
enum LinkStyle {
    Solid,
    Dashed,
    Thick,
}

struct Edge {
    from: usize,
    to: usize,
    label: String,
    style: LinkStyle,
    arrow: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Dir {
    Down,
    Up,
    Right,
    Left,
}

#[derive(Default)]
struct Graph {
    nodes: Vec<Node>,
    index: HashMap<String, usize>,
    edges: Vec<Edge>,
    dir: Dir,
}

impl Default for Dir {
    fn default() -> Self {
        Dir::Down
    }
}

impl Graph {
    fn node(&mut self, id: &str) -> usize {
        if let Some(&i) = self.index.get(id) {
            return i;
        }
        let i = self.nodes.len();
        self.index.insert(id.to_string(), i);
        self.nodes.push(Node {
            label: id.to_string(),
            shape: Shape::Rect,
            rank: 0,
            rect: Rect::ZERO,
        });
        i
    }
}

fn parse(code: &str) -> Graph {
    let mut g = Graph::default();
    for raw in code.lines() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let mut words = line.split_whitespace();
        match words.next() {
            Some("graph") | Some("flowchart") => {
                g.dir = match words.next().unwrap_or("TD") {
                    "LR" => Dir::Right,
                    "RL" => Dir::Left,
                    "BT" => Dir::Up,
                    _ => Dir::Down,
                };
                continue;
            }
            Some("subgraph") | Some("end") => continue,
            _ => {}
        }
        for stmt in line.split(';') {
            parse_statement(stmt.trim(), &mut g);
        }
    }
    assign_ranks(&mut g);
    g
}

fn strip_comment(line: &str) -> &str {
    line.find("%%").map_or(line, |at| &line[..at])
}

fn parse_statement(stmt: &str, g: &mut Graph) {
    if stmt.is_empty() {
        return;
    }
    let bytes = stmt.as_bytes();
    let mut i = skip_ws(bytes, 0);
    let Some((id, label, shape, next)) = parse_node(stmt, i) else { return };
    let mut prev = g.node(&id);
    if let Some(label) = label {
        g.nodes[prev].label = label;
    }
    if let Some(shape) = shape {
        g.nodes[prev].shape = shape;
    }
    i = skip_ws(bytes, next);
    while i < bytes.len() {
        let Some((style, arrow, label, after_link)) = parse_link(stmt, i) else { break };
        let link_label = label;
        i = skip_ws(bytes, after_link);
        let Some((id, node_label, node_shape, next)) = parse_node(stmt, i) else { break };
        let to = g.node(&id);
        if let Some(node_label) = node_label {
            g.nodes[to].label = node_label;
        }
        if let Some(node_shape) = node_shape {
            g.nodes[to].shape = node_shape;
        }
        g.edges.push(Edge { from: prev, to, label: link_label, style, arrow });
        prev = to;
        i = skip_ws(bytes, next);
    }
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    i
}

fn is_id_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Parses `id`, an optional `shape[label]`, returning the id, label, shape and the index past it.
fn parse_node(s: &str, start: usize) -> Option<(String, Option<String>, Option<Shape>, usize)> {
    let bytes = s.as_bytes();
    let mut i = start;
    let id_start = i;
    while i < bytes.len() && is_id_char(bytes[i]) {
        i += 1;
    }
    if i == id_start {
        return None;
    }
    let id = s[id_start..i].to_string();
    let (open, close, shape) = match bytes.get(i) {
        Some(b'[') if bytes.get(i + 1) == Some(&b'[') => ("[[", "]]", Shape::Rect),
        Some(b'(') if bytes.get(i + 1) == Some(&b'(') => ("((", "))", Shape::Circle),
        Some(b'(') if bytes.get(i + 1) == Some(&b'[') => ("([", "])", Shape::Stadium),
        Some(b'{') if bytes.get(i + 1) == Some(&b'{') => ("{{", "}}", Shape::Hexagon),
        Some(b'[') => ("[", "]", Shape::Rect),
        Some(b'(') => ("(", ")", Shape::Rounded),
        Some(b'{') => ("{", "}", Shape::Diamond),
        _ => return Some((id, None, None, i)),
    };
    i += open.len();
    let label_start = i;
    let close_at = s[i..].find(close)? + i;
    let label = unquote(&s[label_start..close_at]);
    i = close_at + close.len();
    Some((id, Some(label), Some(shape), i))
}

fn unquote(text: &str) -> String {
    let t = text.trim();
    t.strip_prefix('"').and_then(|t| t.strip_suffix('"')).unwrap_or(t).to_string()
}

const ARROW_CHARS: &[u8] = b"-.=<>xo";

/// Parses a link such as `-->`, `-.->`, `==>`, `-->|label|`, or `-- text -->`.
fn parse_link(s: &str, start: usize) -> Option<(LinkStyle, bool, String, usize)> {
    let bytes = s.as_bytes();
    if !matches!(bytes.get(start), Some(b'-') | Some(b'=') | Some(b'<')) {
        return None;
    }
    let mut i = start;
    while i < bytes.len() && ARROW_CHARS.contains(&bytes[i]) {
        i += 1;
    }
    let first = &s[start..i];
    let mut label = String::new();
    let mut end_run = first;

    if bytes.get(i) == Some(&b'|') {
        let close = s[i + 1..].find('|')? + i + 1;
        label = s[i + 1..close].trim().to_string();
        i = close + 1;
    } else if matches!(first, "--" | "-." | "==") && !first.contains('>') {
        let text_start = i;
        while i < bytes.len() && !ARROW_CHARS.contains(&bytes[i]) && bytes[i] != b'|' {
            i += 1;
        }
        label = s[text_start..i].trim().to_string();
        let run_start = i;
        while i < bytes.len() && ARROW_CHARS.contains(&bytes[i]) {
            i += 1;
        }
        end_run = &s[run_start..i];
    }

    let style = if first.contains('.') {
        LinkStyle::Dashed
    } else if first.contains('=') {
        LinkStyle::Thick
    } else {
        LinkStyle::Solid
    };
    let arrow = first.contains('>') || end_run.contains('>') || end_run.contains('x') || end_run.contains('o');
    Some((style, arrow, unquote(&label), i))
}

fn assign_ranks(g: &mut Graph) {
    let n = g.nodes.len();
    for _ in 0..n {
        let mut changed = false;
        for e in &g.edges {
            if e.from == e.to {
                continue;
            }
            let want = g.nodes[e.from].rank + 1;
            if g.nodes[e.to].rank < want {
                g.nodes[e.to].rank = want.min(n.saturating_sub(1));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

const H_PAD: f32 = 14.0;
const V_PAD: f32 = 10.0;
const RANK_GAP: f32 = 55.0;
const NODE_GAP: f32 = 26.0;
const MARGIN: f32 = 14.0;

pub(super) fn render(ui: &mut egui::Ui, code: &str) {
    let mut g = parse(code);
    if g.nodes.is_empty() {
        super::fallback(ui, code);
        return;
    }
    let style = Style::of(ui);
    layout(ui, &mut g, &style);

    let bounds = g.nodes.iter().fold(Rect::NOTHING, |acc, n| acc.union(n.rect));
    let size = bounds.size() + Vec2::splat(MARGIN * 2.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let offset = rect.min - bounds.min + Vec2::splat(MARGIN);
    let painter = ui.painter_at(rect);

    for e in &g.edges {
        draw_edge(&painter, &g.nodes[e.from], &g.nodes[e.to], e, offset, &style);
    }
    for node in &g.nodes {
        draw_node(&painter, node, offset, &style);
    }
}

fn layout(ui: &egui::Ui, g: &mut Graph, style: &Style) {
    for node in &mut g.nodes {
        let text = text_size(ui, &node.label, &style.font);
        let (mut w, mut h) = (text.x + H_PAD * 2.0, text.y + V_PAD * 2.0);
        match node.shape {
            Shape::Circle => {
                let d = w.max(h).max(44.0);
                w = d;
                h = d;
            }
            Shape::Diamond | Shape::Hexagon => {
                w += text.x * 0.5;
                h += 8.0;
            }
            _ => {}
        }
        node.rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h));
    }

    let max_rank = g.nodes.iter().map(|n| n.rank).max().unwrap_or(0);
    let horizontal = matches!(g.dir, Dir::Right | Dir::Left);

    // Thickness of each rank band is the largest node across it.
    let mut band = vec![0.0f32; max_rank + 1];
    for node in &g.nodes {
        let t = if horizontal { node.rect.width() } else { node.rect.height() };
        band[node.rank] = band[node.rank].max(t);
    }
    let mut band_start = vec![0.0f32; max_rank + 1];
    let mut acc = 0.0;
    for r in 0..=max_rank {
        band_start[r] = acc;
        acc += band[r] + RANK_GAP;
    }

    let mut cursor = vec![0.0f32; max_rank + 1];
    let mut lane_len = vec![0.0f32; max_rank + 1];
    // First pass measures each lane's length so we can centre them.
    for node in &g.nodes {
        let len = if horizontal { node.rect.height() } else { node.rect.width() };
        lane_len[node.rank] += len + NODE_GAP;
    }
    let full = lane_len.iter().cloned().fold(0.0f32, f32::max);

    for node in &mut g.nodes {
        let r = node.rank;
        let along_len = if horizontal { node.rect.height() } else { node.rect.width() };
        let lane_offset = (full - lane_len[r]) / 2.0;
        let along = lane_offset + cursor[r] + along_len / 2.0;
        cursor[r] += along_len + NODE_GAP;
        let across = band_start[r] + band[r] / 2.0;
        let center = if horizontal {
            let x = if g.dir == Dir::Left { -across } else { across };
            Pos2::new(x, along)
        } else {
            let y = if g.dir == Dir::Up { -across } else { across };
            Pos2::new(along, y)
        };
        node.rect = Rect::from_center_size(center, node.rect.size());
    }
}

fn draw_node(painter: &egui::Painter, node: &Node, offset: Vec2, style: &Style) {
    let rect = node.rect.translate(offset);
    let stroke = Stroke::new(1.5, style.node_stroke);
    match node.shape {
        Shape::Diamond => {
            let c = rect.center();
            let pts = vec![
                Pos2::new(c.x, rect.top()),
                Pos2::new(rect.right(), c.y),
                Pos2::new(c.x, rect.bottom()),
                Pos2::new(rect.left(), c.y),
            ];
            painter.add(egui::Shape::convex_polygon(pts, style.node_fill, stroke));
        }
        Shape::Hexagon => {
            let inset = rect.height() * 0.4;
            let pts = vec![
                Pos2::new(rect.left() + inset, rect.top()),
                Pos2::new(rect.right() - inset, rect.top()),
                Pos2::new(rect.right(), rect.center().y),
                Pos2::new(rect.right() - inset, rect.bottom()),
                Pos2::new(rect.left() + inset, rect.bottom()),
                Pos2::new(rect.left(), rect.center().y),
            ];
            painter.add(egui::Shape::convex_polygon(pts, style.node_fill, stroke));
        }
        shape => {
            let radius = match shape {
                Shape::Circle => rect.height() / 2.0,
                Shape::Stadium => rect.height() / 2.0,
                Shape::Rounded => 10.0,
                _ => 4.0,
            };
            painter.rect(rect, radius, style.node_fill, stroke, egui::StrokeKind::Inside);
        }
    }
    painter.text(rect.center(), Align2::CENTER_CENTER, &node.label, style.font.clone(), style.text);
}

fn draw_edge(painter: &egui::Painter, from: &Node, to: &Node, edge: &Edge, offset: Vec2, style: &Style) {
    let a = from.rect.translate(offset);
    let b = to.rect.translate(offset);
    let start = border_point(a, b.center());
    let end = border_point(b, a.center());
    let width = if edge.style == LinkStyle::Thick { 2.5 } else { 1.3 };
    let stroke = Stroke::new(width, style.line);
    if edge.style == LinkStyle::Dashed {
        dashed(painter, start, end, stroke);
    } else {
        painter.line_segment([start, end], stroke);
    }
    if edge.arrow {
        let dir = (end - start).normalized();
        arrowhead(painter, end, dir, style.line);
    }
    if !edge.label.is_empty() {
        let mid = start + (end - start) * 0.5;
        chip_label(painter, mid, &edge.label, &style.small, style);
    }
}

/// The point on `rect`'s border along the line from its centre toward `target`.
fn border_point(rect: Rect, target: Pos2) -> Pos2 {
    let c = rect.center();
    let d = target - c;
    if d == Vec2::ZERO {
        return c;
    }
    let hx = rect.width() / 2.0;
    let hy = rect.height() / 2.0;
    let tx = if d.x.abs() > 1e-3 { hx / d.x.abs() } else { f32::INFINITY };
    let ty = if d.y.abs() > 1e-3 { hy / d.y.abs() } else { f32::INFINITY };
    let t = tx.min(ty);
    c + d * t
}
