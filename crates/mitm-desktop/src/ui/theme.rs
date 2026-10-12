use eframe::egui::{Color32, Ui, Visuals};

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

/// Buttons use the proportional fonts, which lack arrows, triangles and dots like `⇩ ▾ ●`; Hack has them.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    if let Some(proportional) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        proportional.push("Hack".to_string());
    }
    ctx.set_fonts(fonts);
}
