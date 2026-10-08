use eframe::egui;
use crate::theme::{self, Tone};
use crate::widgets::code_view::{self, Lang};
use crate::widgets::split;
use super::{IndentOpt, JsonToolsState, Mode};

pub fn show(ui: &mut egui::Ui, state: &mut JsonToolsState) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.prefs.mode, Mode::Format, "Format");
        ui.selectable_value(&mut state.prefs.mode, Mode::Minify, "Minify");
        ui.selectable_value(&mut state.prefs.mode, Mode::Escape, "Escape");
        if state.prefs.mode == Mode::Format {
            ui.separator();
            egui::ComboBox::from_id_salt("json_indent")
                .selected_text(state.prefs.indent.label())
                .show_ui(ui, |ui| {
                    for indent in IndentOpt::ALL {
                        ui.selectable_value(&mut state.prefs.indent, indent, indent.label());
                    }
                });
            ui.checkbox(&mut state.prefs.sort_keys, "Sort keys");
        }
    });
    ui.separator();

    let mut fraction = state.prefs.split;
    let [mut left, mut right] = split::side_by_side(ui, "json_tools_split", &mut fraction);
    state.prefs.split = fraction;

    left.label("Input");
    egui::ScrollArea::both().id_salt("json_input").auto_shrink([false, false]).show(&mut left, |ui| {
        ui.add_sized(
            ui.available_size(),
            egui::TextEdit::multiline(&mut state.input).code_editor().desired_width(f32::INFINITY),
        );
    });

    let out = state.output().clone();
    output_pane(&mut right, &out);
}

fn output_pane(ui: &mut egui::Ui, out: &Result<String, String>) {
    match out {
        Ok(text) => {
            ui.horizontal(|ui| {
                ui.label("Output");
                if ui.small_button("📋").on_hover_text("Copy output").clicked() {
                    ui.ctx().copy_text(text.clone());
                }
            });
            egui::ScrollArea::both().id_salt("json_output").auto_shrink([false, false]).show(ui, |ui| {
                if !text.is_empty() {
                    let job = code_view::layout_job(ui.style(), text, Lang::Json);
                    ui.add(egui::Label::new(job).selectable(true).wrap());
                }
            });
        }
        Err(e) => {
            ui.label("Output");
            ui.colored_label(theme::color(ui, Tone::Danger), e);
        }
    }
}
