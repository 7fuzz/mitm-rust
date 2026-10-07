use eframe::egui::{self, Color32, RichText};
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
                .color(Color32::from_rgb(245, 158, 11))
                .strong();
            ui.label(badge);
            ui.separator();
        }

        add_panel_menu(ui, dock, cmds);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(format!("{} flows in memory", state.traffic.flows.len())).weak());
        });
    });
}

fn proxy_controls(ui: &mut egui::Ui, state: &AppState, cmds: &mut Commands) {
    let proxy = &state.proxy;
    let (status, button) = if proxy.running {
        (RichText::new("● Running").color(Color32::GREEN).strong(), "⏹ Stop Proxy")
    } else {
        (RichText::new("○ Stopped").color(Color32::GRAY), "▶ Start Proxy")
    };
    ui.label(status);
    if ui.button(button).clicked() {
        cmds.push(AppCommand::ToggleProxy);
    }
    ui.separator();

    if proxy.running && !proxy.active_summary.is_empty() {
        ui.label(RichText::new(&proxy.active_summary).monospace().size(11.0));
        ui.separator();
    }
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
