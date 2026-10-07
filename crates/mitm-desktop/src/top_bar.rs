use eframe::egui::{self, RichText};
use mitm_core::state::ProxyMode;
use crate::theme::{self, Tone};
use egui_dock::DockState;
use crate::command::{AppCommand, Commands};
use crate::dock::Tab;
use crate::state::AppState;

pub fn show(ui: &mut egui::Ui, state: &AppState, dock: &DockState<Tab>, cmds: &mut Commands) {
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

        add_panel_menu(ui, dock, cmds);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::widgets::global_theme_preference_switch(ui);
            ui.label(RichText::new(format!("{} flows in memory", state.traffic.flows.len())).weak());
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
        ui.label(RichText::new("⚠").color(theme::color(ui, Tone::Danger)).strong()).on_hover_text(problems.join("\n"));
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
    let absent: Vec<Tab> = Tab::ALL.into_iter().filter(|t| dock.find_tab(t).is_none()).collect();

    ui.menu_button("➕ Add Panel ▾", |ui| {
        if absent.is_empty() {
            ui.label(RichText::new("All panels are open").weak());
        }
        for tab in absent {
            if ui.button(tab.name()).clicked() {
                cmds.push(AppCommand::OpenTab(tab));
                ui.close();
            }
        }
    });
}
