use eframe::egui::{self, RichText};
use mitm_core::utils::hashing::{self, HmacAlgo};
use crate::app::command::{AppCommand, Commands};
use crate::ui::theme::{self, Tone};
use super::{HashingState, Mode};
use crate::ui::icons;

pub fn show(ui: &mut egui::Ui, state: &mut HashingState, cmds: &mut Commands) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.prefs.mode, Mode::Digest, "Digest");
        ui.selectable_value(&mut state.prefs.mode, Mode::Hmac, "HMAC");
        ui.selectable_value(&mut state.prefs.mode, Mode::Bcrypt, "Bcrypt");
        if state.prefs.mode != Mode::Bcrypt {
            ui.separator();
            ui.checkbox(&mut state.prefs.upper, "Upper");
        }
    });
    ui.separator();

    match state.prefs.mode {
        Mode::Digest => digest(ui, state),
        Mode::Hmac => hmac(ui, state),
        Mode::Bcrypt => bcrypt(ui, state, cmds),
    }
}

fn input_box(ui: &mut egui::Ui, label: &str, text: &mut String) {
    ui.label(label);
    ui.add(egui::TextEdit::multiline(text).code_editor().desired_rows(4).desired_width(f32::INFINITY));
}

fn digest(ui: &mut egui::Ui, state: &mut HashingState) {
    input_box(ui, "Input", &mut state.input);
    ui.add_space(6.0);
    let bytes = state.input.as_bytes();
    let upper = state.prefs.upper;
    hash_row(ui, "MD5", hashing::md5_hex(bytes), upper);
    hash_row(ui, "SHA-1", hashing::sha1_hex(bytes), upper);
    hash_row(ui, "SHA-256", hashing::sha256_hex(bytes), upper);
    hash_row(ui, "SHA-512", hashing::sha512_hex(bytes), upper);
}

fn hmac(ui: &mut egui::Ui, state: &mut HashingState) {
    ui.horizontal(|ui| {
        ui.label("Key");
        ui.add(egui::TextEdit::singleline(&mut state.hmac_key).desired_width(f32::INFINITY));
    });
    input_box(ui, "Input", &mut state.input);
    ui.add_space(6.0);
    let key = state.hmac_key.as_bytes();
    let bytes = state.input.as_bytes();
    let upper = state.prefs.upper;
    hash_row(ui, "SHA-1", hashing::hmac_hex(HmacAlgo::Sha1, key, bytes), upper);
    hash_row(ui, "SHA-256", hashing::hmac_hex(HmacAlgo::Sha256, key, bytes), upper);
    hash_row(ui, "SHA-512", hashing::hmac_hex(HmacAlgo::Sha512, key, bytes), upper);
}

fn hash_row(ui: &mut egui::Ui, label: &str, mut value: String, upper: bool) {
    if upper {
        value.make_ascii_uppercase();
    }
    ui.horizontal(|ui| {
        ui.add_sized([70.0, ui.spacing().interact_size.y], egui::Label::new(RichText::new(label).strong()));
        if ui.small_button(icons::COPY).on_hover_text("Copy").clicked() {
            ui.ctx().copy_text(value.clone());
        }
        ui.add(egui::Label::new(RichText::new(value).monospace()).truncate());
    });
}

fn bcrypt(ui: &mut egui::Ui, state: &mut HashingState, cmds: &mut Commands) {
    ui.heading("Generate");
    ui.horizontal(|ui| {
        ui.label("Password");
        ui.add(egui::TextEdit::singleline(&mut state.bcrypt_password).desired_width(220.0));
        ui.label("Cost");
        ui.add(egui::DragValue::new(&mut state.prefs.bcrypt_cost).range(4..=15));
        if ui.button("Generate ⟳").on_hover_text("Each run uses a fresh random salt").clicked() {
            cmds.push(AppCommand::GenerateBcrypt {
                password: state.bcrypt_password.clone(),
                cost: state.prefs.bcrypt_cost,
            });
        }
    });
    if let Some(result) = &state.bcrypt_hash {
        match result {
            Ok(hash) => {
                ui.horizontal(|ui| {
                    if ui.small_button(icons::COPY).on_hover_text("Copy").clicked() {
                        ui.ctx().copy_text(hash.clone());
                    }
                    ui.add(egui::Label::new(RichText::new(hash).monospace()).truncate());
                });
            }
            Err(e) => {
                ui.colored_label(theme::color(ui, Tone::Danger), e);
            }
        }
    }

    ui.separator();
    ui.heading("Verify");
    ui.horizontal(|ui| {
        ui.label("Password");
        ui.add(egui::TextEdit::singleline(&mut state.verify_password).desired_width(220.0));
    });
    ui.horizontal(|ui| {
        ui.label("Hash");
        ui.add(egui::TextEdit::singleline(&mut state.verify_hash).desired_width(f32::INFINITY));
    });
    ui.horizontal(|ui| {
        if ui.add_enabled(!state.verify_hash.trim().is_empty(), egui::Button::new("Verify")).clicked() {
            cmds.push(AppCommand::VerifyBcrypt {
                password: state.verify_password.clone(),
                hash: state.verify_hash.trim().to_string(),
            });
        }
        if let Some(result) = &state.verify_result {
            match result {
                Ok(true) => ui.colored_label(theme::color(ui, Tone::Success), icons::CHECK),
                Ok(false) => ui.colored_label(theme::color(ui, Tone::Danger), icons::X),
                Err(e) => ui.colored_label(theme::color(ui, Tone::Danger), e),
            };
        }
    });
}
