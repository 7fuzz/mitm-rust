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
            ui.strong("Status");
            ui.strong("Enabled");
            ui.strong("Label");
            ui.strong("Address (IP:Port)");
            ui.strong("ID");
            ui.strong("");
            ui.end_row();

            for (idx, listener) in proxy.listeners.iter_mut().enumerate() {
                let status_label = if proxy.listener_errors.contains_key(&listener.id) {
                    RichText::new("●").color(theme::color(ui, Tone::Danger))
                } else if proxy.running_listeners.contains(&listener.id) {
                    RichText::new("●").color(theme::color(ui, Tone::Success))
                } else {
                    RichText::new("○").weak()
                };
                let tooltip = proxy.listener_errors.get(&listener.id).map(|s| s.as_str()).unwrap_or_else(|| {
                    if proxy.running_listeners.contains(&listener.id) {
                        "Bound and listening"
                    } else {
                        "Stopped"
                    }
                });
                ui.label(status_label).on_hover_text(tooltip);

                if ui.checkbox(&mut listener.enabled, "").changed() {
                    cmds.push(AppCommand::SetListenerEnabled {
                        id: listener.id,
                        enabled: listener.enabled,
                    });
                }

                if ui.add(egui::TextEdit::singleline(&mut listener.label).desired_width(120.0)).lost_focus() {
                    modified = true;
                }

                if ui.add(egui::TextEdit::singleline(&mut listener.address).desired_width(180.0)).lost_focus() {
                    modified = true;
                }

                ui.label(listener.id.to_string());

                if listener.id != 0 {
                    if ui.small_button("🗑").clicked() {
                        to_remove = Some(idx);
                    }
                } else {
                    ui.label("");
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
        ui.add(egui::TextEdit::singleline(&mut form.label).hint_text("e.g. Mobile, Docker").desired_width(110.0));

        ui.label("Host:");
        ui.add(egui::TextEdit::singleline(&mut form.host).hint_text("0.0.0.0").desired_width(90.0));

        let host_hint = match form.host.trim() {
            "" | "0.0.0.0" => "all interfaces",
            "127.0.0.1" | "localhost" => "this machine",
            _ => "",
        };
        if !host_hint.is_empty() {
            ui.label(RichText::new(host_hint).weak());
        }

        ui.label("Port:");
        ui.add(egui::TextEdit::singleline(&mut form.port).hint_text("8082").desired_width(50.0));

        let port_num = form.port.trim().parse::<u16>().ok().filter(|&p| p > 0);
        let overlap = port_num.and_then(|p| find_listener_overlap(&proxy.listeners, &form.host, p));

        if let Some(id) = overlap {
            ui.label(RichText::new(format!("covered by #{}", id)).color(theme::color(ui, Tone::Warn)));
        }

        let can_add = !form.label.trim().is_empty() && port_num.is_some() && overlap.is_none();
        if ui.add_enabled(can_add, egui::Button::new("➕ Add Listener")).clicked() {
            let port = port_num.unwrap();
            let effective_host = match form.host.trim() {
                "" => "0.0.0.0",
                "localhost" => "127.0.0.1",
                h => h,
            };
            let address = if effective_host.contains(':') && !effective_host.starts_with('[') {
                format!("[{}]:{}", effective_host, port)
            } else {
                format!("{}:{}", effective_host, port)
            };
            let next_id = proxy.listeners.iter().map(|l| l.id).max().unwrap_or(0) + 1;
            proxy.listeners.push(mitm_core::state::ListenerConfig {
                id: next_id,
                label: form.label.trim().to_string(),
                address,
                enabled: true,
            });
            form.label.clear();
            modified = true;
        }
    });

    if modified {
        cmds.push(AppCommand::SaveListeners(proxy.listeners.clone()));
    }

    ui.add_space(8.0);
    let id = ui.make_persistent_id("host_addresses_collapsing");
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            ui.strong("Host addresses");
            if ui.small_button("⟳").on_hover_text("Refresh host addresses").clicked() {
                proxy.reload_host_ips();
            }
        })
        .body(|ui| {
            if proxy.host_ips.is_empty() {
                proxy.reload_host_ips();
            }
            egui::Grid::new("host_ips_grid")
                .striped(true)
                .num_columns(4)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.strong("Status");
                    ui.strong("IP");
                    ui.strong("Interface");
                    ui.strong("");
                    ui.end_row();

                    for ip_info in &proxy.host_ips {
                        let (dot, tone, tooltip) = if ip_info.is_up {
                            ("●", Tone::Success, "Interface up")
                        } else {
                            ("○", Tone::Neutral, "Interface down")
                        };
                        ui.label(RichText::new(dot).color(theme::color(ui, tone))).on_hover_text(tooltip);

                        let ip_resp = ui.selectable_label(false, RichText::new(&ip_info.ip).monospace());
                        let iface_resp = ui.label(&ip_info.interface);

                        let ip = &ip_info.ip;
                        ip_resp.context_menu(|ui| {
                            if ui.button("Copy").clicked() {
                                ui.ctx().copy_text(ip.clone());
                                ui.close();
                            }
                            if ui.button("Use in new listener").clicked() {
                                form.host = ip.clone();
                                ui.close();
                            }
                        });
                        iface_resp.context_menu(|ui| {
                            if ui.button("Copy").clicked() {
                                ui.ctx().copy_text(ip.clone());
                                ui.close();
                            }
                            if ui.button("Use in new listener").clicked() {
                                form.host = ip.clone();
                                ui.close();
                            }
                        });

                        if ip_info.is_link_local {
                            ui.label(RichText::new("link-local").weak().small());
                        } else {
                            ui.label("");
                        }
                        ui.end_row();
                    }
                });
        });

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

fn find_listener_overlap(listeners: &[mitm_core::state::ListenerConfig], host: &str, port: u16) -> Option<u32> {
    let candidate_host = match host.trim() {
        "" => "0.0.0.0",
        h if h.eq_ignore_ascii_case("localhost") => "127.0.0.1",
        h => h,
    };
    let candidate_ip: std::net::IpAddr = candidate_host.parse().ok()?;
    for listener in listeners {
        if let Ok(addr) = listener.address.parse::<std::net::SocketAddr>() {
            if addr.port() == port {
                let ip = addr.ip();
                let overlap = ip == candidate_ip
                    || (ip.is_unspecified() && candidate_ip.is_unspecified())
                    || (ip.is_unspecified() && ip.is_ipv4() == candidate_ip.is_ipv4())
                    || (candidate_ip.is_unspecified() && ip.is_ipv4() == candidate_ip.is_ipv4());
                if overlap {
                    return Some(listener.id);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use mitm_core::state::ListenerConfig;

    #[test]
    fn detects_overlapping_listeners() {
        let listeners = vec![
            ListenerConfig { id: 0, label: "default".into(), address: "127.0.0.1:8080".into(), enabled: true },
            ListenerConfig { id: 1, label: "all".into(), address: "0.0.0.0:8085".into(), enabled: true },
        ];
        assert_eq!(find_listener_overlap(&listeners, "127.0.0.1", 8080), Some(0));
        assert_eq!(find_listener_overlap(&listeners, "", 8080), Some(0));
        assert_eq!(find_listener_overlap(&listeners, "localhost", 8080), Some(0));
        assert_eq!(find_listener_overlap(&listeners, "127.0.0.1", 8085), Some(1));
        assert_eq!(find_listener_overlap(&listeners, "192.168.1.1", 8085), Some(1));
        assert_eq!(find_listener_overlap(&listeners, "127.0.0.1", 8081), None);
        assert_eq!(find_listener_overlap(&listeners, "192.168.1.1", 8080), None);
    }
}
