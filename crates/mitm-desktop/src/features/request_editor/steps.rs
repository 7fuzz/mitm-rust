use eframe::egui::{self, RichText};
use crate::theme::{self, Tone};

pub struct StepOption {
    pub id: String,
    pub label: String,
}

pub fn show(ui: &mut egui::Ui, pre: &mut Vec<String>, post: &mut Vec<String>, options: &[StepOption]) -> bool {
    let pre_changed = step_list(ui, "PRE-REQUESTS", "pre", pre, options);
    ui.add_space(12.0);
    let post_changed = step_list(ui, "POST-REQUESTS", "post", post, options);
    pre_changed || post_changed
}

fn step_list(ui: &mut egui::Ui, title: &str, id_salt: &str, ids: &mut Vec<String>, options: &[StepOption]) -> bool {
    let label_of = |id: &str| options.iter().find(|o| o.id == id).map_or(id.to_string(), |o| o.label.clone());
    let mut action: Option<(usize, isize)> = None;

    ui.label(RichText::new(title).weak().size(10.0).strong());
    for (index, id) in ids.iter().enumerate() {
        ui.push_id((id_salt, index), |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{}.", index + 1)).weak().monospace());
                ui.add(egui::Label::new(RichText::new(label_of(id)).monospace()).truncate());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(RichText::new("🗑").color(theme::color(ui, Tone::Danger))).on_hover_text("Remove").clicked() {
                        action = Some((index, 0));
                    }
                    if ui.add_enabled(index + 1 < ids.len(), egui::Button::new("⏷").small()).clicked() {
                        action = Some((index, 1));
                    }
                    if ui.add_enabled(index > 0, egui::Button::new("⏶").small()).clicked() {
                        action = Some((index, -1));
                    }
                });
            });
        });
    }

    let mut changed = false;
    if let Some((index, delta)) = action {
        match delta {
            0 => {
                ids.remove(index);
            }
            d => ids.swap(index, (index as isize + d) as usize),
        }
        changed = true;
    }

    egui::ComboBox::from_id_salt((id_salt, "add"))
        .selected_text("Add request…")
        .width(320.0)
        .show_ui(ui, |ui| {
            for option in options {
                if ui.selectable_label(false, &option.label).clicked() {
                    ids.push(option.id.clone());
                    changed = true;
                }
            }
        });
    changed
}
