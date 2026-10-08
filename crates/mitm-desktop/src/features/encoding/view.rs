use eframe::egui::{self};
use crate::theme::{self, Tone};
use crate::widgets::code_view::{self, Lang};
use crate::widgets::split;
use super::{Algorithm, Computed, EncodingPrefs, EncodingState, HexFmt};

pub fn show(ui: &mut egui::Ui, state: &mut EncodingState) {
    let swap = toolbar(ui, &mut state.prefs);
    ui.separator();

    let mut fraction = state.prefs.split;
    let [mut left, mut right] = split::side_by_side(ui, "enc_split", &mut fraction);
    state.prefs.split = fraction;

    input_pane(&mut left, &mut state.input);
    let out = state.output().clone();
    output_pane(&mut right, &out);

    if swap {
        if let Computed::Text(Ok(text)) = &out {
            state.input = text.clone();
            state.prefs.decode = !state.prefs.decode;
            state.cache = None;
        }
    }
}

fn toolbar(ui: &mut egui::Ui, prefs: &mut EncodingPrefs) -> bool {
    let mut swap = false;
    let jwt = prefs.algorithm == Algorithm::Jwt;
    ui.horizontal_wrapped(|ui| {
        ui.add_enabled_ui(!jwt, |ui| {
            ui.selectable_value(&mut prefs.decode, false, "Encode");
            ui.selectable_value(&mut prefs.decode, true, "Decode");
        });
        ui.separator();
        egui::ComboBox::from_id_salt("enc_algo")
            .selected_text(prefs.algorithm.label())
            .show_ui(ui, |ui| {
                for algo in Algorithm::ALL {
                    ui.selectable_value(&mut prefs.algorithm, algo, algo.label());
                }
            });

        match prefs.algorithm {
            Algorithm::Hex => {
                egui::ComboBox::from_id_salt("enc_hex_fmt")
                    .selected_text(prefs.hex_format.label())
                    .show_ui(ui, |ui| {
                        for fmt in HexFmt::ALL {
                            ui.selectable_value(&mut prefs.hex_format, fmt, fmt.label());
                        }
                    });
                ui.checkbox(&mut prefs.hex_upper, "Upper");
            }
            Algorithm::Url => {
                ui.checkbox(&mut prefs.url_full, "Full").on_hover_text("Percent-encode every byte");
            }
            _ => {}
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(!jwt, egui::Button::new("⇄ Swap"))
                .on_hover_text("Feed the output back in and flip direction")
                .clicked()
            {
                swap = true;
            }
        });
    });
    swap
}

fn input_pane(ui: &mut egui::Ui, input: &mut String) {
    ui.label("Input");
    egui::ScrollArea::both().id_salt("enc_input").auto_shrink([false, false]).show(ui, |ui| {
        ui.add_sized(
            ui.available_size(),
            egui::TextEdit::multiline(input).code_editor().desired_width(f32::INFINITY),
        );
    });
}

fn output_pane(ui: &mut egui::Ui, out: &Computed) {
    match out {
        Computed::Text(Ok(text)) => {
            output_header(ui, text);
            code_block(ui, "enc_output", text, code_view::detect("", text));
        }
        Computed::Text(Err(e)) | Computed::Jwt(Err(e)) => {
            ui.label("Output");
            ui.colored_label(theme::color(ui, Tone::Danger), e);
        }
        Computed::Jwt(Ok(parts)) => {
            egui::ScrollArea::vertical().id_salt("enc_jwt").auto_shrink([false, false]).show(ui, |ui| {
                jwt_section(ui, "Header", &parts.header, Lang::Json);
                jwt_section(ui, "Payload", &parts.payload, Lang::Json);
                jwt_section(ui, "Signature", &parts.signature, Lang::Plain);
            });
        }
    }
}

fn output_header(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.label("Output");
        if ui.small_button("📋").on_hover_text("Copy output").clicked() {
            ui.ctx().copy_text(text.to_string());
        }
    });
}

fn jwt_section(ui: &mut egui::Ui, label: &str, text: &str, lang: Lang) {
    ui.horizontal(|ui| {
        ui.strong(label);
        if ui.small_button("📋").clicked() {
            ui.ctx().copy_text(text.to_string());
        }
    });
    let job = code_view::layout_job(ui.style(), text, lang);
    ui.add(egui::Label::new(job).selectable(true).wrap());
    ui.add_space(6.0);
}

fn code_block(ui: &mut egui::Ui, id: &str, text: &str, lang: Lang) {
    egui::ScrollArea::both().id_salt(id).auto_shrink([false, false]).show(ui, |ui| {
        if text.is_empty() {
            return;
        }
        let job = code_view::layout_job(ui.style(), text, lang);
        ui.add(egui::Label::new(job).selectable(true).wrap());
    });
}
