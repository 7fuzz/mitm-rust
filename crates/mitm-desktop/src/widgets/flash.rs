use std::time::Duration;
use eframe::egui;

const VISIBLE_SECS: f64 = 1.8;

pub fn set(ui: &egui::Ui, id: egui::Id, message: impl Into<String>) {
    let now = ui.input(|i| i.time);
    ui.data_mut(|d| d.insert_temp(id, (message.into(), now)));
}

pub fn current(ui: &egui::Ui, id: egui::Id) -> Option<String> {
    let now = ui.input(|i| i.time);
    let (message, at): (String, f64) = ui.data_mut(|d| d.get_temp(id))?;
    if now - at >= VISIBLE_SECS {
        return None;
    }
    ui.ctx().request_repaint_after(Duration::from_millis(200));
    Some(message)
}
