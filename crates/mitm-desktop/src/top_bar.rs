use eframe::egui::{self, RichText};
use mitm_core::state::ProxyMode;
use crate::theme::{self, Tone};
use egui_dock::DockState;
use crate::command::{AppCommand, Commands};
use crate::dock::Tab;
use crate::state::AppState;
use crate::widgets::flash;

const FLASH_OK: &str = "top_bar_flash_ok";
const FLASH_ERR: &str = "top_bar_flash_err";

/// A short-lived message next to the workspace combos, for results of background saves.
pub fn flash(ui: &egui::Ui, result: Result<String, String>) {
    match result {
        Ok(message) => flash::set(ui, egui::Id::new(FLASH_OK), message),
        Err(error) => flash::set(ui, egui::Id::new(FLASH_ERR), error),
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut AppState, dock: &DockState<Tab>, cmds: &mut Commands) {
    ui.horizontal(|ui| {
        proxy_controls(ui, state, cmds);

        let paused = state.intercept.pending.len();
        if paused > 0 {
            let badge = RichText::new(format!("⚠️ {} Paused Flow(s)", paused))
                .color(theme::color(ui, Tone::Warn))
                .strong();
            ui.label(badge);
            ui.separator();
        }

        workspace_and_env_combos(ui, state, cmds);
        for (id, tone) in [(FLASH_OK, Tone::Success), (FLASH_ERR, Tone::Danger)] {
            if let Some(message) = flash::current(ui, egui::Id::new(id)) {
                ui.label(RichText::new(message).color(theme::color(ui, tone)));
            }
        }
        ui.separator();

        add_panel_menu(ui, dock, cmds);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::widgets::global_theme_preference_switch(ui);
            ui.label(RichText::new(format!("{} flows in memory", state.traffic.flows.len())).weak());
        });
    });

    new_workspace_modal(ui.ctx(), state, cmds);
}

fn workspace_and_env_combos(ui: &mut egui::Ui, state: &mut AppState, cmds: &mut Commands) {
    let ws_state = &mut state.environments;
    let active_ws_name = ws_state
        .active_workspace()
        .map(|w| w.name.as_str())
        .unwrap_or("No workspace");

    egui::ComboBox::from_id_salt("top_bar_workspace_combo")
        .selected_text(active_ws_name)
        .show_ui(ui, |ui| {
            if ws_state.workspaces.len() > 8 {
                ui.add(egui::TextEdit::singleline(&mut ws_state.workspace_search).hint_text("Search…"));
                ui.separator();
            }
            let search_lower = ws_state.workspace_search.trim().to_lowercase();
            for ws in &ws_state.workspaces {
                if !search_lower.is_empty() && !ws.name.to_lowercase().contains(&search_lower) {
                    continue;
                }
                let is_active = ws.id == ws_state.active_workspace_id;
                if ui.selectable_label(is_active, &ws.name).clicked() && !is_active {
                    cmds.push(AppCommand::SelectWorkspace(ws.id.clone()));
                }
            }
            ui.separator();
            if ui.button("➕ New…").clicked() {
                ws_state.new_workspace_modal = true;
                ws_state.new_workspace_name.clear();
                ui.close();
            }
            if ui.button("⇩ Import…").clicked() {
                cmds.push(AppCommand::OpenImport);
                ui.close();
            }
            if ui.button("⇧ Export…").clicked() {
                cmds.push(AppCommand::ExportWorkspace);
                ui.close();
            }
            if ui.button("⚙ Manage").clicked() {
                cmds.push(AppCommand::OpenTab(Tab::Environments));
                ui.close();
            }
        });

    let active_env_name = ws_state
        .active_environment()
        .map(|e| e.name.as_str())
        .unwrap_or("No environment");

    egui::ComboBox::from_id_salt("top_bar_environment_combo")
        .selected_text(active_env_name)
        .show_ui(ui, |ui| {
            let is_none_active = ws_state.active_environment_id.is_none();
            let none_label = if is_none_active { "● No environment" } else { "No environment" };
            if ui.selectable_label(is_none_active, none_label).clicked() {
                cmds.push(AppCommand::SelectEnvironment(None));
            }
            if !ws_state.environments.is_empty() {
                ui.separator();
            }
            for env in &ws_state.environments {
                let is_active = Some(&env.id) == ws_state.active_environment_id.as_ref();
                let label = if is_active {
                    format!("● {}", env.name)
                } else {
                    env.name.clone()
                };
                if ui.selectable_label(is_active, label).clicked() && !is_active {
                    cmds.push(AppCommand::SelectEnvironment(Some(env.id.clone())));
                }
            }
        });
}

fn new_workspace_modal(ctx: &egui::Context, state: &mut AppState, cmds: &mut Commands) {
    if !state.environments.new_workspace_modal {
        return;
    }
    egui::Modal::new(egui::Id::new("new_workspace_modal")).show(ctx, |ui| {
        ui.set_width(300.0);
        ui.heading("New Workspace");
        ui.add_space(6.0);
        let resp = ui.add(
            egui::TextEdit::singleline(&mut state.environments.new_workspace_name)
                .hint_text("Workspace name"),
        );
        let name = state.environments.new_workspace_name.trim().to_string();
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if enter && !name.is_empty() {
            cmds.push(AppCommand::CreateWorkspace(name.clone()));
            state.environments.new_workspace_name.clear();
            state.environments.new_workspace_modal = false;
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let can_create = !name.is_empty();
            if ui.add_enabled(can_create, egui::Button::new("Create")).clicked() {
                cmds.push(AppCommand::CreateWorkspace(name));
                state.environments.new_workspace_name.clear();
                state.environments.new_workspace_modal = false;
            }
            if ui.button("Cancel").clicked() {
                state.environments.new_workspace_name.clear();
                state.environments.new_workspace_modal = false;
            }
        });
    });
}

fn proxy_controls(ui: &mut egui::Ui, state: &AppState, cmds: &mut Commands) {
    let proxy = &state.proxy;
    let toggle_hint = if proxy.mode.is_enabled() { "Turn the proxy off" } else { "Turn the proxy on" };
    if ui.button(mode_text(ui, proxy.mode).strong()).on_hover_text(toggle_hint).clicked() {
        cmds.push(AppCommand::ToggleProxy);
    }
    ui.menu_button("⏷", |ui| {
        for mode in ProxyMode::ALL {
            if ui.selectable_label(proxy.mode == mode, mode_text(ui, mode)).clicked() {
                cmds.push(AppCommand::SetProxyMode(mode));
                ui.close();
            }
        }
    });
    let problems = proxy.problems();
    if !problems.is_empty() {
        let warning = egui::Label::new(RichText::new("⚠").color(theme::color(ui, Tone::Danger)).strong()).sense(egui::Sense::click());
        if ui.add(warning).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(problems.join("\n")).clicked() {
            cmds.push(AppCommand::OpenSettings(crate::features::settings::SettingsSection::Proxy));
        }
    }
    ui.separator();

    if proxy.running && !proxy.active_summary.is_empty() {
        ui.label(RichText::new(&proxy.active_summary).monospace().size(11.0));
        ui.separator();
    }
}

pub fn mode_label(mode: ProxyMode) -> &'static str {
    match mode {
        ProxyMode::On => "On",
        ProxyMode::Off => "Off",
        ProxyMode::BlockClient => "Block client",
        ProxyMode::BlockAll => "Block all",
    }
}

fn mode_text(ui: &egui::Ui, mode: ProxyMode) -> RichText {
    let (dot, color) = match mode {
        ProxyMode::On => ("●", theme::color(ui, Tone::Success)),
        ProxyMode::Off => ("○", ui.visuals().weak_text_color()),
        ProxyMode::BlockClient => ("●", theme::color(ui, Tone::Warn)),
        ProxyMode::BlockAll => ("●", theme::color(ui, Tone::Danger)),
    };
    RichText::new(format!("{} {}", dot, mode_label(mode))).color(color)
}

fn add_panel_menu(ui: &mut egui::Ui, dock: &DockState<Tab>, cmds: &mut Commands) {
    let absent = |tab: &Tab| dock.find_tab(tab).is_none();
    let top: Vec<Tab> = Tab::ALL.into_iter().filter(|t| !t.is_utility() && absent(t)).collect();
    let utilities: Vec<Tab> = Tab::UTILITIES.iter().copied().filter(absent).collect();

    ui.menu_button("➕ Add Panel ▾", |ui| {
        if top.is_empty() && utilities.is_empty() {
            ui.label(RichText::new("All panels are open").weak());
        }
        for tab in top {
            if ui.button(tab.name()).clicked() {
                cmds.push(AppCommand::OpenTab(tab));
                ui.close();
            }
        }
        if !utilities.is_empty() {
            ui.separator();
            ui.menu_button("Utilities ▸", |ui| {
                for tab in utilities {
                    if ui.button(tab.name()).clicked() {
                        cmds.push(AppCommand::OpenTab(tab));
                        ui.close();
                    }
                }
            });
        }
    });
}
