use std::time::Duration;
use eframe::egui::{self, RichText};
use crate::ui::icons;
use crate::ui::theme::{Palette, Tone};

const SUCCESS_SECS: f64 = 2.5;
const ERROR_SECS: f64 = 8.0;
const MAX_WIDTH: f32 = 360.0;
/// Clears the status bar below the dock.
const BOTTOM_OFFSET: f32 = 34.0;

#[derive(Clone)]
struct Toast {
    text: String,
    error: bool,
    shown_at: f64,
}

impl Toast {
    fn remaining(&self, now: f64) -> f64 {
        let lifetime = if self.error { ERROR_SECS } else { SUCCESS_SECS };
        lifetime - (now - self.shown_at)
    }
}

fn queue_id() -> egui::Id {
    egui::Id::new("toasts")
}

pub fn push(ctx: &egui::Context, result: Result<String, String>) {
    let now = ctx.input(|i| i.time);
    let (text, error) = match result {
        Ok(text) => (text, false),
        Err(text) => (text, true),
    };
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Toast>>(queue_id()).push(Toast { text, error, shown_at: now }));
    ctx.request_repaint();
}

/// Draws the queue bottom-right, newest at the bottom. Repaints are only scheduled while a toast is visible.
pub fn show(ctx: &egui::Context) {
    let now = ctx.input(|i| i.time);
    let mut toasts: Vec<Toast> = ctx.data(|d| d.get_temp(queue_id())).unwrap_or_default();
    toasts.retain(|t| t.remaining(now) > 0.0);
    if toasts.is_empty() {
        ctx.data_mut(|d| d.remove::<Vec<Toast>>(queue_id()));
        return;
    }

    let visuals = ctx.global_style().visuals.clone();
    let palette = Palette::of(&visuals);
    let mut dismissed = None;
    egui::Area::new(queue_id())
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -BOTTOM_OFFSET))
        .show(ctx, |ui| {
            for (index, toast) in toasts.iter().enumerate() {
                let tone = if toast.error { Tone::Danger } else { Tone::Success };
                let icon = if toast.error { icons::WARNING } else { icons::CHECK };
                let card = egui::Frame::new()
                    .fill(palette.raised)
                    .stroke(egui::Stroke::new(1.0, palette.border))
                    .corner_radius(6)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .shadow(visuals.popup_shadow)
                    .show(ui, |ui| {
                        ui.set_max_width(MAX_WIDTH);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(icon).color(tone.color(&visuals)));
                            ui.add(egui::Label::new(&toast.text).truncate().selectable(false));
                        });
                    })
                    .response
                    .interact(egui::Sense::click());
                if card.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    dismissed = Some(index);
                }
            }
        });
    if let Some(index) = dismissed {
        toasts.remove(index);
    }

    let next_expiry = toasts.iter().map(|t| t.remaining(now)).fold(f64::INFINITY, f64::min);
    if next_expiry.is_finite() {
        ctx.request_repaint_after(Duration::from_secs_f64(next_expiry.max(0.05)));
    }
    ctx.data_mut(|d| d.insert_temp(queue_id(), toasts));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, time: f64) -> egui::FullOutput {
        let input = egui::RawInput { time: Some(time), ..Default::default() };
        let mut output = ctx.run_ui(input, |ui| show(ui.ctx()));
        output.textures_delta.clear();
        output
    }

    fn queued(ctx: &egui::Context) -> usize {
        ctx.data(|d| d.get_temp::<Vec<Toast>>(queue_id())).map_or(0, |q| q.len())
    }

    #[test]
    fn successes_expire_before_errors_and_repaints_stop() {
        let ctx = egui::Context::default();
        frame(&ctx, 0.0);
        push(&ctx, Ok("Saved".into()));
        push(&ctx, Err("Couldn't save".into()));
        frame(&ctx, 0.1);
        assert_eq!(queued(&ctx), 2);
        frame(&ctx, 3.0);
        assert_eq!(queued(&ctx), 1);
        let output = frame(&ctx, 9.0);
        assert_eq!(queued(&ctx), 0);
        let repaint = output.viewport_output.values().map(|v| v.repaint_delay).min().unwrap();
        assert!(repaint > Duration::from_secs(60));
    }
}
