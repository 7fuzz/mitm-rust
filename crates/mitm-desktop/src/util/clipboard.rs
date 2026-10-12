use std::time::Duration;
use eframe::egui;
use zeroize::Zeroizing;

const CLEAR_AFTER: Duration = Duration::from_secs(30);

/// Copies a secret or code and wipes it from the clipboard after 30s, unless something else was copied since.
pub fn copy_secret(ctx: &egui::Context, value: &str) {
    ctx.copy_text(value.to_string());
    let value = Zeroizing::new(value.to_string());
    std::thread::spawn(move || {
        std::thread::sleep(CLEAR_AFTER);
        let Ok(mut clipboard) = arboard::Clipboard::new() else { return };
        if clipboard.get_text().is_ok_and(|current| current == *value) {
            let _ = clipboard.clear();
        }
    });
}
