use std::time::Duration;
use eframe::egui::{self, RichText};
use mitm_core::utils::totp::{self, TotpParams};
use super::{group_code, TotpState};

const NEXT_HINT_SECS: u64 = 5;

pub fn show(ui: &mut egui::Ui, state: &mut TotpState) {
    ui.add(
        egui::TextEdit::singleline(&mut state.input)
            .hint_text("secret or otpauth://…")
            .desired_width(f32::INFINITY),
    );
    let input = state.input.trim();
    if input.is_empty() {
        return;
    }
    ui.add_space(8.0);
    match totp::parse_secret(input) {
        Ok(params) => {
            if let Ok(auth) = totp::parse_otpauth(input) {
                let who = match auth.issuer {
                    Some(issuer) => format!("{issuer} · {}", auth.label),
                    None => auth.label,
                };
                if !who.is_empty() {
                    ui.label(RichText::new(who).weak());
                }
            }
            show_code(ui, &params, 32.0);
        }
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
    }
}

/// The current code with a copy button and a countdown bar; repaints when the window rolls over.
pub fn show_code(ui: &mut egui::Ui, params: &TotpParams, size: f32) {
    let now = totp::now_unix();
    let code = totp::code_at(params, now);
    let left = totp::remaining(params.period, now);

    ui.horizontal(|ui| {
        ui.label(RichText::new(group_code(&code)).monospace().size(size).strong());
        if ui.small_button("📋").on_hover_text("Copy").clicked() {
            crate::util::clipboard::copy_secret(ui.ctx(), &code);
        }
    });
    ui.horizontal(|ui| {
        let fraction = left as f32 / params.period.max(1) as f32;
        ui.add(egui::ProgressBar::new(fraction).desired_width(160.0).desired_height(4.0));
        ui.label(RichText::new(format!("{left}s")).weak().monospace());
        if left <= NEXT_HINT_SECS {
            let next = totp::code_at(params, now + params.period);
            ui.label(RichText::new(format!("next {}", group_code(&next))).weak().monospace());
        }
    });
    ui.ctx().request_repaint_after(Duration::from_secs(1));
}
