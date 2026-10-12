use eframe::egui::{self, RichText};
use crate::ui::theme::{self, Tone};
use crate::widgets::diff;
use crate::widgets::split;
use super::{DiffStatus, JsonDiffState};

pub fn show(ui: &mut egui::Ui, state: &mut JsonDiffState) {
    ui.horizontal(|ui| {
        if ui.button("⇄ Swap").clicked() {
            state.swap();
        }
        if ui.button("Clear").clicked() {
            state.clear();
        }
    });
    ui.separator();

    let mut vert_fraction = state.prefs.split;
    let [mut top_ui, mut bottom_ui] = split::stacked(ui, "json_diff_vert_split", &mut vert_fraction);
    state.prefs.split = vert_fraction;

    let status = state.diff();
    inputs_split(&mut top_ui, state, &status);
    diff_pane(&mut bottom_ui, &status);
}

fn inputs_split(ui: &mut egui::Ui, state: &mut JsonDiffState, status: &DiffStatus) {
    let mut horiz_fraction = state.prefs.input_split;
    let [mut left_ui, mut right_ui] = split::side_by_side(ui, "json_diff_inputs_split", &mut horiz_fraction);
    state.prefs.input_split = horiz_fraction;

    let (left_err, right_err) = match status {
        DiffStatus::LeftError(e) => (Some(e.as_str()), None),
        DiffStatus::RightError(e) => (None, Some(e.as_str())),
        DiffStatus::BothError(l, r) => (Some(l.as_str()), Some(r.as_str())),
        _ => (None, None),
    };
    input_editor(&mut left_ui, "Left", &mut state.left, "json_diff_left", left_err);
    input_editor(&mut right_ui, "Right", &mut state.right, "json_diff_right", right_err);
}

fn input_editor(ui: &mut egui::Ui, label: &str, text: &mut String, id_salt: &str, error: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(label);
        if let Some(err) = error {
            ui.colored_label(theme::color(ui, Tone::Danger), err);
        }
    });
    egui::ScrollArea::both().id_salt(id_salt).auto_shrink([false, false]).show(ui, |ui| {
        let edit = egui::TextEdit::multiline(text).code_editor().desired_width(f32::INFINITY);
        // The diff below was computed from last frame's text.
        if ui.add_sized(ui.available_size(), edit).changed() {
            ui.ctx().request_repaint();
        }
    });
}

fn diff_pane(ui: &mut egui::Ui, status: &DiffStatus) {
    ui.horizontal(|ui| {
        ui.label("Diff");
        match status {
            DiffStatus::Identical => {
                ui.label(RichText::new("✔").color(theme::color(ui, Tone::Success)).strong());
            }
            DiffStatus::Diff { added, removed, .. } => {
                ui.label(RichText::new(format!("+{added} -{removed}")).weak());
            }
            _ => {}
        }
    });
    ui.separator();
    if let DiffStatus::Diff { lines, .. } = status {
        diff::show_fill(ui, "json_diff_output", lines);
    }
}
