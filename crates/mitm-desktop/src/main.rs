mod app;
mod command;
mod conflict;
mod dock;
mod features;
mod files;
mod format;
mod jobs;
mod palette;
mod prefs;
mod recovery;
mod shell;
mod shortcuts;
mod startup;
mod state;
mod theme;
mod top_bar;
mod widgets;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("MITM")
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/icon-256.png"))
                    .expect("embedded icon is a valid PNG"),
            ),
        ..Default::default()
    };

    eframe::run_native(
        "MITM",
        native_options,
        Box::new(|cc| Ok(Box::new(shell::Shell::new(cc)))),
    )
}
