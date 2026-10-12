use eframe::egui::{self, Color32, CornerRadius, Shadow, Stroke, Ui, Visuals};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Success,
    Info,
    Warn,
    Caution,
    Danger,
    Accent,
    Source,
    Neutral,
}

impl Tone {
    pub fn color(self, visuals: &Visuals) -> Color32 {
        let (dark, light) = match self {
            Tone::Success => (Color32::from_rgb(74, 222, 128), Color32::from_rgb(21, 128, 61)),
            Tone::Info => (Color32::from_rgb(96, 165, 250), Color32::from_rgb(29, 78, 216)),
            Tone::Warn => (Color32::from_rgb(245, 158, 11), Color32::from_rgb(180, 83, 9)),
            Tone::Caution => (Color32::from_rgb(251, 146, 60), Color32::from_rgb(194, 65, 12)),
            Tone::Danger => (Color32::from_rgb(244, 63, 94), Color32::from_rgb(190, 18, 60)),
            Tone::Accent => (Color32::from_rgb(192, 132, 252), Color32::from_rgb(126, 34, 206)),
            Tone::Source => (Color32::from_rgb(165, 180, 252), Color32::from_rgb(67, 56, 202)),
            Tone::Neutral => (Color32::from_rgb(156, 163, 175), Color32::from_rgb(75, 85, 99)),
        };
        if visuals.dark_mode { dark } else { light }
    }

    pub fn tint(self, visuals: &Visuals, alpha: u8) -> Color32 {
        let c = self.color(visuals);
        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
    }
}

/// Surface and text colours; `Tone` covers the semantic ones.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub base: Color32,
    pub panel: Color32,
    pub raised: Color32,
    pub hover: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub text_strong: Color32,
    pub text_weak: Color32,
    pub accent: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            base: Color32::from_rgb(0x0F, 0x11, 0x15),
            panel: Color32::from_rgb(0x15, 0x18, 0x1D),
            raised: Color32::from_rgb(0x1C, 0x20, 0x27),
            hover: Color32::from_rgb(0x25, 0x2A, 0x32),
            border: Color32::from_rgb(0x27, 0x2C, 0x35),
            border_strong: Color32::from_rgb(0x3A, 0x40, 0x4B),
            text: Color32::from_rgb(0xD7, 0xDA, 0xE0),
            text_strong: Color32::from_rgb(0xF3, 0xF4, 0xF7),
            text_weak: Color32::from_rgb(0x8A, 0x90, 0x9B),
            accent: Color32::from_rgb(0x6E, 0x8B, 0xFF),
        }
    }

    pub fn light() -> Self {
        Self {
            base: Color32::from_rgb(0xF4, 0xF5, 0xF7),
            panel: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            raised: Color32::from_rgb(0xF0, 0xF2, 0xF5),
            hover: Color32::from_rgb(0xE6, 0xE9, 0xEE),
            border: Color32::from_rgb(0xDD, 0xE1, 0xE7),
            border_strong: Color32::from_rgb(0xC5, 0xCB, 0xD4),
            text: Color32::from_rgb(0x1E, 0x22, 0x28),
            text_strong: Color32::from_rgb(0x0B, 0x0D, 0x10),
            text_weak: Color32::from_rgb(0x69, 0x70, 0x7C),
            accent: Color32::from_rgb(0x3B, 0x5B, 0xDB),
        }
    }
}

/// Styles both themes up front, so the System/Light/Dark switch needs no extra plumbing.
pub fn apply_style(ctx: &egui::Context) {
    ctx.style_mut_of(egui::Theme::Dark, |s| set_style(s, &Palette::dark()));
    ctx.style_mut_of(egui::Theme::Light, |s| set_style(s, &Palette::light()));
}

fn set_style(style: &mut egui::Style, palette: &Palette) {
    set_visuals(&mut style.visuals, palette);
    set_spacing(&mut style.spacing);
    set_text_styles(style);
}

fn set_spacing(s: &mut egui::style::Spacing) {
    s.item_spacing = egui::vec2(8.0, 6.0);
    s.button_padding = egui::vec2(8.0, 4.0);
    s.interact_size.y = 24.0;
    s.window_margin = egui::Margin::same(12);
    s.indent = 16.0;
    s.combo_width = 160.0;
    s.scroll = egui::style::ScrollStyle::floating();
}

fn set_text_styles(style: &mut egui::Style) {
    use egui::{FontFamily, FontId, TextStyle};
    style.text_styles = [
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
        (TextStyle::Heading, FontId::new(15.0, strong_family())),
        (TextStyle::Name("section".into()), FontId::new(11.0, strong_family())),
    ]
    .into();
}

fn set_visuals(v: &mut Visuals, p: &Palette) {
    let shadow_alpha = if v.dark_mode { 110 } else { 30 };
    let shadow = Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(shadow_alpha) };

    v.panel_fill = p.panel;
    v.window_fill = p.raised;
    v.faint_bg_color = p.raised;
    v.extreme_bg_color = p.base;
    v.text_edit_bg_color = Some(p.base);
    v.code_bg_color = p.base;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(6);
    v.window_shadow = shadow;
    v.popup_shadow = shadow;
    v.selection.bg_fill = p.accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.hyperlink_color = p.accent;
    v.text_cursor.stroke = Stroke::new(2.0, p.accent);
    v.weak_text_color = Some(p.text_weak);
    v.warn_fg_color = Tone::Warn.color(v);
    v.error_fg_color = Tone::Danger.color(v);
    v.striped = false;

    let radius = CornerRadius::same(4);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.panel;
    w.noninteractive.weak_bg_fill = p.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    // Outlined rather than frameless, so buttons still read as buttons at rest.
    w.inactive.weak_bg_fill = Color32::TRANSPARENT;
    w.inactive.bg_fill = p.raised;
    w.inactive.bg_stroke = Stroke::new(1.0, p.border);
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.hovered.weak_bg_fill = p.hover;
    w.hovered.bg_fill = p.hover;
    w.hovered.bg_stroke = Stroke::new(1.0, p.border_strong);
    w.hovered.fg_stroke = Stroke::new(1.0, p.text_strong);
    w.active.weak_bg_fill = p.hover;
    w.active.bg_fill = p.hover;
    w.active.bg_stroke = Stroke::new(1.0, p.accent);
    w.active.fg_stroke = Stroke::new(1.0, p.text_strong);
    w.open.weak_bg_fill = p.raised;
    w.open.bg_fill = p.raised;
    w.open.bg_stroke = Stroke::new(1.0, p.border);
    w.open.fg_stroke = Stroke::new(1.0, p.text);
    for state in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        state.corner_radius = radius;
        state.expansion = 0.0;
    }
}

pub fn color(ui: &Ui, tone: Tone) -> Color32 {
    tone.color(ui.visuals())
}

pub fn method_tone(method: &str) -> Tone {
    match method.to_uppercase().as_str() {
        "GET" => Tone::Success,
        "POST" => Tone::Info,
        "PUT" => Tone::Warn,
        "PATCH" => Tone::Accent,
        "DELETE" => Tone::Danger,
        _ => Tone::Neutral,
    }
}

pub fn status_tone(code: u16) -> Tone {
    match code {
        200..=299 => Tone::Success,
        300..=399 => Tone::Info,
        400..=499 => Tone::Caution,
        500..=599 => Tone::Danger,
        _ => Tone::Neutral,
    }
}

pub const STRONG_FAMILY: &str = "strong";

pub fn strong_family() -> egui::FontFamily {
    egui::FontFamily::Name(STRONG_FAMILY.into())
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let embedded: [(&str, &'static [u8]); 4] = [
        ("Inter", include_bytes!("../../assets/fonts/Inter-Regular.ttf")),
        ("Inter-SemiBold", include_bytes!("../../assets/fonts/Inter-SemiBold.ttf")),
        ("JetBrainsMono", include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf")),
        ("Phosphor", include_bytes!("../../assets/fonts/Phosphor-Regular.ttf")),
    ];
    for (name, bytes) in embedded {
        fonts.font_data.insert(name.to_string(), std::sync::Arc::new(egui::FontData::from_static(bytes)));
    }
    // Phosphor comes after the text font: it maps a-z to the blank glyphs its ligatures are built from.
    // Hack covers arrows, triangles and dots the text fonts lack; the emoji fonts cover user data.
    let fallback = ["Hack", "NotoEmoji-Regular", "emoji-icon-font"];
    let chain = |first: &[&str]| first.iter().chain(&fallback).map(|s| s.to_string()).collect::<Vec<_>>();
    fonts.families.insert(egui::FontFamily::Proportional, chain(&["Inter", "Phosphor"]));
    fonts.families.insert(egui::FontFamily::Monospace, chain(&["JetBrainsMono", "Phosphor"]));
    fonts.families.insert(strong_family(), chain(&["Inter-SemiBold", "Phosphor", "Inter"]));
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_fonts_lay_out_text() {
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        apply_style(&ctx);
        let mut output = ctx.run_ui(Default::default(), |ui| {
            ui.heading("Traffic");
            ui.label(crate::ui::icons::COPY);
            ui.label(egui::RichText::new("GET /v1/users").monospace());
            ui.label(egui::RichText::new("PROXY").text_style(egui::TextStyle::Name("section".into())));
        });
        assert!(!output.textures_delta.set.is_empty());
        output.textures_delta.clear();
    }
}
