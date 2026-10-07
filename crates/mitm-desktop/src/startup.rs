use std::sync::Arc;
use std::time::Duration;
use eframe::egui::{self, RichText};
use mitm_core::db::InitError;
use mitm_core::events::{CoreEvent, EventBus, MigrationProgressPayload};
use mitm_core::MitmCore;
use tokio::runtime::Runtime;
use tokio::sync::broadcast::{self, error::TryRecvError};
use crate::jobs::Jobs;

pub struct Starting {
    events: broadcast::Receiver<CoreEvent>,
    result: Jobs<Result<Arc<MitmCore>, InitError>>,
    progress: Option<MigrationProgressPayload>,
}

impl Starting {
    pub fn spawn(rt: &Runtime, ctx: &egui::Context) -> Self {
        let bus = EventBus::default();
        let events = bus.subscribe();
        let result = Jobs::default();
        let reply = result.reply(ctx);
        rt.spawn_blocking(move || reply.send(MitmCore::init_with_events(bus).map(Arc::new)));
        Self { events, result, progress: None }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<Result<Arc<MitmCore>, InitError>> {
        loop {
            match self.events.try_recv() {
                Ok(CoreEvent::DbMigrationProgress(progress)) => self.progress = Some(progress),
                Ok(_) | Err(TryRecvError::Lagged(_)) => {}
                Err(_) => break,
            }
        }
        if let Some(result) = self.result.drain().next() {
            return Some(result);
        }
        ui.ctx().request_repaint_after(Duration::from_millis(50));

        let progress = self.progress.as_ref().filter(|p| !p.is_complete && p.total > 0)?;
        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height() * 0.4).max(24.0));
                ui.set_max_width(420.0);
                ui.label(RichText::new("Upgrading database").heading());
                ui.add_space(8.0);
                let fraction = progress.step as f32 / progress.total as f32;
                ui.add(egui::ProgressBar::new(fraction).text(format!("{} / {}", progress.step, progress.total)));
                ui.label(RichText::new(&progress.name).monospace().weak().small());
            });
        });
        None
    }
}
