use std::sync::{Arc, Mutex};
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

pub fn markdown(ui: &mut egui::Ui, text: &str) {
    // The cache isn't Clone, which egui's temp storage requires.
    let cache = ui.data_mut(|d| d.get_temp_mut_or_default::<Arc<Mutex<CommonMarkCache>>>(egui::Id::new("markdown_cache")).clone());
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    CommonMarkViewer::new().show(ui, &mut cache, text);
}
