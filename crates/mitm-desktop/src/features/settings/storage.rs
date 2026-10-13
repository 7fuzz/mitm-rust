use eframe::egui::{self, RichText};
use mitm_core::db::storage::{self, StorageStats};
use mitm_core::db::Db;
use crate::app::jobs::Latest;
use crate::ui::icons;
use crate::ui::theme::{self, Tone};
use crate::util::format::{format_bytes, format_count};
use crate::widgets::button::primary;
use crate::widgets::section::section_with;

#[derive(Default)]
pub struct StorageState {
    stats: Option<StorageStats>,
    loading: Latest<Result<StorageStats, String>>,
    vacuuming: Latest<Result<(u64, u64), String>>,
    status: Option<(String, Tone)>,
    requested: bool,
}

impl StorageState {
    fn refresh(&mut self, db: &Db, ctx: &egui::Context) {
        let db = db.clone();
        self.loading.start(ctx, move || storage::storage_stats(&db));
    }

    fn vacuum(&mut self, db: &Db, ctx: &egui::Context) {
        self.status = None;
        let db = db.clone();
        self.vacuuming.start(ctx, move || storage::vacuum(&db));
    }

    fn poll(&mut self, db: &Db, ctx: &egui::Context) {
        if !std::mem::replace(&mut self.requested, true) {
            self.refresh(db, ctx);
        }
        match self.loading.take() {
            Some(Ok(stats)) => self.stats = Some(stats),
            Some(Err(e)) => self.status = Some((e, Tone::Danger)),
            None => {}
        }
        if let Some(result) = self.vacuuming.take() {
            self.status = Some(match result {
                Ok((before, after)) => {
                    (format!("Reclaimed {} ({} → {})", format_bytes(before.saturating_sub(after)), format_bytes(before), format_bytes(after)), Tone::Success)
                }
                Err(e) => (e, Tone::Danger),
            });
            self.refresh(db, ctx);
        }
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut StorageState, db: &Db) {
    state.poll(db, ui.ctx());
    let busy = state.loading.is_running() || state.vacuuming.is_running();

    section_with(ui, "Storage", |ui| {
        if ui.add_enabled(!busy, egui::Button::new(icons::ARROW_CLOCKWISE).frame_when_inactive(false)).on_hover_text("Refresh").clicked() {
            state.refresh(db, ui.ctx());
        }
        if busy {
            ui.spinner();
        }
    });
    if let Some((message, tone)) = &state.status {
        ui.label(RichText::new(message).color(theme::color(ui, *tone)));
        ui.add_space(4.0);
    }
    let Some(stats) = state.stats.clone() else { return };

    egui::Grid::new("storage_summary").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
        ui.label("File");
        ui.monospace(format_bytes(stats.file_bytes));
        ui.end_row();
        ui.label("Data");
        ui.monospace(format_bytes(stats.used_bytes()));
        ui.end_row();
        ui.label("Free space").on_hover_text("Left behind by deleted rows. Vacuum gives it back to the disk.");
        let free = RichText::new(format_bytes(stats.free_bytes)).monospace();
        ui.label(if stats.should_vacuum() { free.color(theme::color(ui, Tone::Warn)) } else { free });
        ui.end_row();
        if stats.wal_bytes > 0 {
            ui.label("Write-ahead log");
            ui.monospace(format_bytes(stats.wal_bytes));
            ui.end_row();
        }
    });
    ui.add_space(8.0);
    usage_bar(ui, &stats);
    ui.add_space(8.0);

    let label = format!("Vacuum ({} free)", format_bytes(stats.free_bytes));
    let button = if stats.should_vacuum() { primary(ui, &label) } else { egui::Button::new(label) };
    if ui.add_enabled(!busy && stats.free_bytes > 0, button).clicked() {
        state.vacuum(db, ui.ctx());
    }

    section_with(ui, "Tables", |_| {});
    let scale = stats.tables.first().map_or(1, |t| t.bytes.max(1));
    egui::Grid::new("storage_tables").num_columns(4).spacing([16.0, 4.0]).striped(false).show(ui, |ui| {
        for title in ["Table", "Rows", "Size", ""] {
            ui.label(RichText::new(title).small().weak());
        }
        ui.end_row();
        for table in &stats.tables {
            ui.monospace(&table.name);
            ui.monospace(format_count(table.rows as usize));
            ui.monospace(format_bytes(table.bytes));
            bar(ui, table.bytes as f32 / scale as f32, 120.0, ui.visuals().selection.stroke.color);
            ui.end_row();
        }
    });
}

fn usage_bar(ui: &mut egui::Ui, stats: &StorageStats) {
    let used = stats.used_bytes() as f32 / stats.file_bytes.max(1) as f32;
    let width = ui.available_width().min(360.0);
    bar(ui, used, width, ui.visuals().selection.stroke.color);
}

fn bar(ui: &mut egui::Ui, fraction: f32, width: f32, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 6.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, ui.visuals().extreme_bg_color);
    let filled = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * fraction.clamp(0.0, 1.0), rect.height()));
    painter.rect_filled(filled, 3.0, color);
}
