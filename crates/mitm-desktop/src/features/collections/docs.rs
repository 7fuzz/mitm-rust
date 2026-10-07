use eframe::egui::{self, RichText};

pub fn markdown(ui: &mut egui::Ui, text: &str) {
    let mut code: Option<Vec<&str>> = None;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            match code.take() {
                Some(lines) => code_block(ui, &lines.join("\n")),
                None => code = Some(Vec::new()),
            }
            continue;
        }
        if let Some(lines) = code.as_mut() {
            lines.push(line);
            continue;
        }

        let trimmed = line.trim_start();
        let heading = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&heading) && trimmed[heading..].starts_with(' ') {
            let size = [22.0, 19.0, 16.0, 14.5, 13.5, 13.0][heading - 1];
            ui.add_space(4.0);
            ui.label(RichText::new(trimmed[heading..].trim()).size(size).strong());
        } else if let Some(item) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            let indent = (line.len() - trimmed.len()) as f32 * 4.0;
            ui.horizontal_wrapped(|ui| {
                ui.add_space(indent + 6.0);
                ui.label("•");
                ui.label(item);
            });
        } else if let Some(quote) = trimmed.strip_prefix("> ") {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(8.0);
                ui.label(RichText::new(quote).weak().italics());
            });
        } else if trimmed == "---" || trimmed == "***" {
            ui.separator();
        } else if trimmed.is_empty() {
            ui.add_space(6.0);
        } else {
            ui.label(line);
        }
    }
    if let Some(lines) = code {
        code_block(ui, &lines.join("\n"));
    }
}

fn code_block(ui: &mut egui::Ui, code: &str) {
    egui::Frame::group(ui.style()).fill(ui.visuals().extreme_bg_color).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(code).monospace());
    });
}
