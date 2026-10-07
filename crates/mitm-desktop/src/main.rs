mod app;
mod command;
mod dock;
mod features;
mod format;
mod jobs;
mod palette;
mod prefs;
mod shortcuts;
mod state;
mod theme;
mod top_bar;
mod widgets;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("MITM"),
        ..Default::default()
    };

    eframe::run_native(
        "MITM",
        native_options,
        Box::new(|cc| Ok(Box::new(app::MitmApp::new(cc)))),
    )
}
