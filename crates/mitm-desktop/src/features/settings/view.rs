use std::path::Path;
use eframe::egui::{self, RichText};
use crate::theme::{self, Tone};
use crate::command::{AppCommand, Commands};
use crate::features::traffic::TrafficState;
use crate::widgets::split;
use super::{ListenerForm, ProxyState, SettingsPrefs, SettingsSection};

pub fn show(
    ui: &mut egui::Ui,
    prefs: &mut SettingsPrefs,
    proxy: &mut ProxyState,
    form: &mut ListenerForm,
    traffic: &TrafficState,
    db_path: &Path,
    cmds: &mut Commands,
) {
    let [mut list, mut content] = split::side_by_side(ui, "settings_split", &mut prefs.list_fraction);
    for section in SettingsSection::ALL {
        list.selectable_value(&mut prefs.section, section, section.name());
    }
    egui::ScrollArea::vertical()
        .id_salt(("settings", prefs.section))
        .auto_shrink([false, false])
        .show(&mut content, |ui| match prefs.section {
            SettingsSection::Proxy => proxy_section(ui, proxy, form, traffic, db_path, cmds),
            SettingsSection::Certificate => {
                ui.heading("Certificate");
            }
            SettingsSection::Appearance => {
                ui.heading("Appearance");
            }
            SettingsSection::Data => {
                ui.heading("Data");
            }
            SettingsSection::Database => {
                ui.heading("Database");
            }
        });
}

fn proxy_section(
    ui: &mut egui::Ui,
    proxy: &mut ProxyState,
    form: &mut ListenerForm,
    traffic: &TrafficState,
    db_path: &Path,
    cmds: &mut Commands,
) {
    ui.strong("Configured Listeners (Multi-Source)");
    ui.label(RichText::new("Each listener binds to an IP and port with a named label so you can easily identify where traffic originated.").weak());
    ui.add_space(6.0);

    let mut modified = false;
    let mut to_remove = None;

    egui::Grid::new("listeners_table_grid")
        .striped(true)
        .num_columns(6)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.strong("Enabled");
            ui.strong("Label");
            ui.strong("Address (IP:Port)");
            ui.strong("ID");
            ui.strong("");
            ui.strong("");
            ui.end_row();

            for (idx, listener) in proxy.listeners.iter_mut().enumerate() {
                if ui.checkbox(&mut listener.enabled, "").changed() {
                    modified = true;
                }

                if ui.add(egui::TextEdit::singleline(&mut listener.label).desired_width(120.0)).lost_focus() {
                    modified = true;
                }

                if ui.add(egui::TextEdit::singleline(&mut listener.address).desired_width(180.0)).lost_focus() {
                    modified = true;
                }

                ui.label(listener.id.to_string());

                match proxy.listener_errors.get(&listener.id) {
                    Some(error) => {
                        ui.label(RichText::new("⚠").color(theme::color(ui, Tone::Danger))).on_hover_text(error);
                    }
                    None => {
                        ui.label("");
                    }
                }

                if ui.small_button("🗑").clicked() {
                    to_remove = Some(idx);
                }
                ui.end_row();
            }
        });

    if let Some(idx) = to_remove {
        proxy.listeners.remove(idx);
        modified = true;
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);

    ui.strong("Add New Listener");
    ui.horizontal(|ui| {
        ui.label("Label:");
        ui.add(egui::TextEdit::singleline(&mut form.label).hint_text("e.g. Mobile, Docker").desired_width(120.0));

        ui.label("Address:");
        ui.add(egui::TextEdit::singleline(&mut form.address).hint_text("127.0.0.1:8081").desired_width(160.0));

        let can_add = !form.label.trim().is_empty() && !form.address.trim().is_empty();
        if ui.add_enabled(can_add, egui::Button::new("➕ Add Listener")).clicked() {
            let next_id = proxy.listeners.iter().map(|l| l.id).max().unwrap_or(0) + 1;
            proxy.listeners.push(mitm_core::state::ListenerConfig {
                id: next_id,
                label: form.label.trim().to_string(),
                address: form.address.trim().to_string(),
                enabled: true,
            });
            form.label.clear();
            modified = true;
        }
    });

    if modified {
        cmds.push(AppCommand::SaveListeners(proxy.listeners.clone()));
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
            if proxy.running {
                ui.label(RichText::new("● Active").color(theme::color(ui, Tone::Success)).strong());
            } else {
                ui.label(RichText::new("○ Stopped").weak());
            }
            ui.end_row();

            ui.strong("Active Listeners:");
            ui.label(&proxy.active_summary);
            ui.end_row();

            ui.strong("Total Flows in Memory:");
            ui.label(format!("{}/{}", traffic.flows.len(), traffic.max_flows));
            ui.end_row();

            ui.strong("Database Location:");
            ui.label(db_path.to_string_lossy());
            ui.end_row();
        });
}
