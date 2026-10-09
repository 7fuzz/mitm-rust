use std::time::Duration;
use eframe::egui::{self, RichText};
use mitm_core::instance::{self, Claim, OtherInstance};
use tokio::runtime::Runtime;
use crate::app::jobs::Jobs;
use crate::ui::theme::{self, Tone};

const TAKE_OVER_ATTEMPTS: u32 = 50;

pub struct Conflict {
    other: OtherInstance,
    take_over: Jobs<bool>,
    waiting: bool,
    failed: bool,
}

impl Conflict {
    pub fn new(other: OtherInstance) -> Self {
        Self { other, take_over: Jobs::default(), waiting: false, failed: false }
    }

    /// Returns true once this launch owns the instance lock.
    pub fn show(&mut self, ui: &mut egui::Ui, rt: &Runtime) -> bool {
        if let Some(claimed) = self.take_over.drain().next() {
            self.waiting = false;
            self.failed = !claimed;
            if claimed {
                return true;
            }
        }

        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height() * 0.35).max(24.0));
                ui.label(RichText::new("mitm is already running").heading());
                ui.label(RichText::new(started_label(&self.other)).weak());
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!self.waiting, |ui| {
                        if ui.button("Switch to it").clicked() {
                            instance::request_focus();
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Take over").on_hover_text("Close the other window and run here").clicked() {
                            self.start_take_over(rt, ui.ctx());
                        }
                        if ui.button("Close").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    if self.waiting {
                        ui.spinner();
                    }
                });
                if self.failed {
                    ui.add_space(8.0);
                    ui.label(RichText::new("The other instance didn't close").color(theme::color(ui, Tone::Danger)));
                }
            });
        });
        false
    }

    fn start_take_over(&mut self, rt: &Runtime, ctx: &egui::Context) {
        self.waiting = true;
        self.failed = false;
        let reply = self.take_over.reply(ctx);
        let ctx = ctx.clone();
        rt.spawn_blocking(move || {
            instance::request_quit();
            for _ in 0..TAKE_OVER_ATTEMPTS {
                if let Claim::Primary = instance::try_claim(handlers(&ctx)) {
                    return reply.send(true);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            reply.send(false);
        });
    }
}

pub fn handlers(ctx: &egui::Context) -> instance::Handlers {
    let quit = ctx.clone();
    let focus = ctx.clone();
    instance::Handlers {
        on_quit: Some(Box::new(move || quit.send_viewport_cmd(egui::ViewportCommand::Close))),
        on_focus: Some(Box::new(move || {
            focus.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            focus.send_viewport_cmd(egui::ViewportCommand::Focus);
            focus.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Informational));
        })),
    }
}

fn started_label(other: &OtherInstance) -> String {
    let started = chrono::DateTime::from_timestamp_millis(other.started_at_ms)
        .map(|at| at.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_default();
    format!("pid {} · since {}", other.pid, started)
}
