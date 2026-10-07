use std::sync::Arc;
use eframe::egui::{self, Color32, RichText};
use mitm_core::MitmCore;
use crate::state::UiState;

pub fn render_settings_view(ui: &mut egui::Ui, state: &mut UiState, core: &Arc<MitmCore>) {
    ui.heading("Proxy Listeners & System Settings");
    ui.add_space(4.0);

    ui.strong("Configured Listeners (Multi-Source)");
    ui.label(RichText::new("Each listener binds to an IP and port with a named label so you can easily identify where traffic originated.").weak());
    ui.add_space(6.0);

    let mut modified = false;
    let mut to_remove = None;

    egui::Grid::new("listeners_table_grid")
        .striped(true)
        .num_columns(5)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.strong("Enabled");
            ui.strong("Label");
            ui.strong("Address (IP:Port)");
            ui.strong("ID");
            ui.strong("");
            ui.end_row();

            for (idx, listener) in state.listeners.iter_mut().enumerate() {
                if ui.checkbox(&mut listener.enabled, "").changed() {
                    modified = true;
                }

                if ui.add(egui::TextEdit::singleline(&mut listener.label).desired_width(120.0)).changed() {
                    modified = true;
                }

                if ui.add(egui::TextEdit::singleline(&mut listener.address).desired_width(180.0)).changed() {
                    modified = true;
                }

                ui.label(listener.id.to_string());

                if ui.small_button("🗑").clicked() {
                    to_remove = Some(idx);
                }
                ui.end_row();
            }
        });

    if let Some(idx) = to_remove {
        state.listeners.remove(idx);
        modified = true;
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);

    ui.strong("Add New Listener");
    ui.horizontal(|ui| {
        ui.label("Label:");
        ui.add(egui::TextEdit::singleline(&mut state.new_listener_label).hint_text("e.g. Mobile, Docker").desired_width(120.0));

        ui.label("Address:");
        ui.add(egui::TextEdit::singleline(&mut state.new_listener_address).hint_text("127.0.0.1:8081").desired_width(160.0));

        let can_add = !state.new_listener_label.trim().is_empty() && !state.new_listener_address.trim().is_empty();
        if ui.add_enabled(can_add, egui::Button::new("➕ Add Listener")).clicked() {
            let next_id = state.listeners.iter().map(|l| l.id).max().unwrap_or(0) + 1;
            state.listeners.push(mitm_core::state::ListenerConfig {
                id: next_id,
                label: state.new_listener_label.trim().to_string(),
                address: state.new_listener_address.trim().to_string(),
                enabled: true,
            });
            state.new_listener_label.clear();
            modified = true;
        }
    });

    if modified {
        let listeners_clone = state.listeners.clone();
        let core_clone = Arc::clone(core);
        tokio::spawn(async move {
            let _ = core_clone.set_listener_configs(listeners_clone).await;
        });
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(6.0);

    ui.strong("System Info");
    egui::Grid::new("system_info_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.strong("Proxy Status:");
            if state.proxy_running {
                ui.label(RichText::new("● Active").color(Color32::GREEN).strong());
            } else {
                ui.label(RichText::new("○ Stopped").color(Color32::GRAY));
            }
            ui.end_row();

            ui.strong("Active Listeners:");
            ui.label(&state.active_listeners_summary);
            ui.end_row();

            ui.strong("Total Flows in Memory:");
            ui.label(format!("{}/{}", state.flows.len(), state.max_flows));
            ui.end_row();

            ui.strong("Database Location:");
            ui.label(core.db_path.to_string_lossy());
            ui.end_row();
        });
}
