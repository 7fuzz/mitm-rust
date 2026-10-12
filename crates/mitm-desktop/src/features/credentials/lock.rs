use eframe::egui::{self, RichText};
use mitm_core::credentials;
use crate::app::command::AppCommand;
use crate::widgets::confirm;
use super::{AutoLock, CredentialsState};
use crate::widgets::section::section;
use mitm_core::db::Db;

/// Asked for when a send needs locked credentials; the send goes out once unlocked.
#[derive(Default)]
pub(super) struct UnlockPrompt {
    pub open: bool,
    pub password: String,
    pub error: Option<String>,
    pub pending: Option<AppCommand>,
}

#[derive(Default)]
pub(super) struct MasterForm {
    current: String,
    new: String,
    repeat: String,
    status: Option<Result<String, String>>,
}

/// The password field and button; returns true once unlocked.
pub(super) fn unlock_form(ui: &mut egui::Ui, state: &mut CredentialsState, db: &Db) -> bool {
    let mut unlocked = false;
    ui.horizontal(|ui| {
        let edit = ui.add(
            egui::TextEdit::singleline(&mut state.unlock.password)
                .password(true)
                .hint_text("master password")
                .desired_width(200.0),
        );
        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.button("Unlock").clicked() || enter {
            match credentials::unlock(db, &state.unlock.password) {
                Ok(()) => {
                    state.unlock.password.clear();
                    state.unlock.error = None;
                    state.stale = true;
                    unlocked = true;
                }
                Err(e) => state.unlock.error = Some(e),
            }
        }
    });
    if let Some(error) = &state.unlock.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    unlocked
}

/// The unlock popup. Returns the send to dispatch again once the password is right.
pub fn show_unlock(ctx: &egui::Context, state: &mut CredentialsState, db: &Db) -> Option<AppCommand> {
    if !state.unlock.open {
        return None;
    }
    let mut cancel = false;
    let mut retry = None;
    let modal = egui::Modal::new(egui::Id::new("credentials_unlock")).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.heading("Credentials locked");
        ui.add_space(6.0);
        if unlock_form(ui, state, db) {
            retry = Some(state.unlock.pending.take());
        }
        ui.add_space(4.0);
        if ui.button("Cancel").clicked() {
            cancel = true;
        }
    });
    if cancel || modal.should_close() || retry.is_some() {
        state.unlock = UnlockPrompt::default();
    }
    retry.flatten()
}

pub fn settings_section(ui: &mut egui::Ui, state: &mut CredentialsState, db: &Db) {
    section(ui, "Master password");
    let form = &mut state.master;
    let password = |text: &mut String, hint: &str, ui: &mut egui::Ui| {
        ui.add(egui::TextEdit::singleline(text).password(true).hint_text(hint).desired_width(220.0));
    };
    if state.has_password {
        password(&mut form.current, "current password", ui);
    }
    password(&mut form.new, "new password", ui);
    password(&mut form.repeat, "repeat new password", ui);
    let matches = !form.new.is_empty() && form.new == form.repeat;
    let mut result = None;
    ui.horizontal(|ui| {
        let label = if state.has_password { "Change" } else { "Set" };
        if ui.add_enabled(matches, egui::Button::new(label)).clicked() {
            let current = state.has_password.then_some(form.current.as_str());
            result = Some(credentials::set_master_password(db, current, &form.new).map(|_| "Master password saved".to_string()));
        }
        if state.has_password && ui.add_enabled(!form.current.is_empty(), egui::Button::new("Remove")).clicked() {
            result = Some(credentials::remove_master_password(db, &form.current).map(|_| "Master password removed".to_string()));
        }
    });
    if let Some(result) = result {
        if result.is_ok() {
            *form = MasterForm::default();
        }
        form.status = Some(result);
        state.stale = true;
    }
    match &form.status {
        Some(Ok(message)) => {
            ui.label(RichText::new(message).weak());
        }
        Some(Err(error)) => {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        None => {}
    }

    if !state.has_password {
        return;
    }
    section(ui, "Auto-lock");
    let mut minutes = match state.prefs.auto_lock {
        AutoLock::AfterIdleMinutes(m) => m,
        AutoLock::Never => 15,
    };
    ui.horizontal(|ui| {
        let idle = matches!(state.prefs.auto_lock, AutoLock::AfterIdleMinutes(_));
        if ui.radio(idle, "After").clicked() || (idle && ui.add(egui::DragValue::new(&mut minutes).range(1..=720)).changed()) {
            state.prefs.auto_lock = AutoLock::AfterIdleMinutes(minutes);
        }
        if !idle {
            ui.add_enabled(false, egui::DragValue::new(&mut minutes));
        }
        ui.label("min idle");
    });
    if ui.radio(state.prefs.auto_lock == AutoLock::Never, "Never").clicked() {
        state.prefs.auto_lock = AutoLock::Never;
    }

    ui.add_space(12.0);
    let reset_id = ui.id().with("reset_credentials");
    if confirm::confirm_button(ui, reset_id, RichText::new("Reset credentials…"), "Wipe all values?", "Forgot the password: clears every value and note, keeps names and fields") {
        state.master.status = Some(credentials::reset_credentials(db).map(|_| "Credential values wiped".to_string()));
        state.stale = true;
    }
}
