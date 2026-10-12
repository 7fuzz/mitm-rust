mod conflict;
mod recovery;
mod startup;

use std::sync::Arc;
use eframe::egui;
use mitm_core::instance::{self, Claim};
use tokio::runtime::Runtime;
use crate::app::MitmApp;
use conflict::Conflict;
use recovery::Recovery;
use startup::Starting;

pub struct Shell {
    rt: Arc<Runtime>,
    screen: Screen,
}

enum Screen {
    Conflict(Conflict),
    Starting(Starting),
    Running(Box<MitmApp>),
    Recovery(Recovery),
}

impl Shell {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        crate::ui::theme::install_fonts(&cc.egui_ctx);
        crate::ui::theme::apply_style(&cc.egui_ctx);
        let rt = Arc::new(Runtime::new().expect("Failed to initialize Tokio runtime"));
        close_on_signal(&rt, &cc.egui_ctx);
        // Nothing that binds ports starts until this launch owns the instance lock.
        let screen = match instance::try_claim(conflict::handlers(&cc.egui_ctx)) {
            Claim::Conflict(other) => Screen::Conflict(Conflict::new(other)),
            Claim::Primary | Claim::Unguarded => Screen::Starting(Starting::spawn(&rt, &cc.egui_ctx)),
        };
        Self { rt, screen }
    }
}

/// Ctrl+C or SIGTERM closes the window the normal way, so eframe saves prefs and `on_exit` flushes
/// pending edits. Taking over SIGINT disables the default kill, so a second Ctrl+C forces the exit.
fn close_on_signal(rt: &Runtime, ctx: &egui::Context) {
    let ctx = ctx.clone();
    rt.spawn(async move {
        shutdown_signal().await;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        ctx.request_repaint();
        let _ = tokio::signal::ctrl_c().await;
        std::process::exit(130);
    });
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    match signal(SignalKind::terminate()) {
        Ok(mut term) => {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = term.recv() => {}
            }
        }
        Err(_) => {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

impl eframe::App for Shell {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Screen::Running(app) = &mut self.screen {
            app.save(storage);
        }
    }

    fn on_exit(&mut self) {
        if let Screen::Running(app) = &mut self.screen {
            app.on_exit();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let next = match &mut self.screen {
            Screen::Running(app) => {
                app.ui(ui, frame);
                None
            }
            Screen::Starting(starting) => starting.show(ui).map(|result| match result {
                Ok(core) => Screen::Running(Box::new(MitmApp::new(ui.ctx(), frame.storage(), Arc::clone(&self.rt), core))),
                Err(error) => Screen::Recovery(Recovery::new(error)),
            }),
            Screen::Conflict(conflict) => {
                conflict.show(ui, &self.rt).then(|| Screen::Starting(Starting::spawn(&self.rt, ui.ctx())))
            }
            Screen::Recovery(recovery) => {
                recovery.show(ui, &self.rt).then(|| Screen::Starting(Starting::spawn(&self.rt, ui.ctx())))
            }
        };
        if let Some(next) = next {
            self.screen = next;
            ui.ctx().request_repaint();
        }
    }
}
