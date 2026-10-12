use std::path::Path;
use eframe::egui::{self, RichText};
use crate::ui::theme::{self, Tone};
use crate::app::command::{AppCommand, Commands};
use crate::features::traffic::TrafficState;
use crate::widgets::split;
use super::{Cmd, CertificateState, DataSectionState, ListenerForm, ProxyState, SettingsPrefs, SettingsSection};
use crate::ui::icons;
use crate::widgets::button::icon_button;
use crate::widgets::card::card;
use crate::widgets::button::danger_filled;
use crate::widgets::list_row::list_row;
use crate::widgets::segmented::segmented;
use crate::util::format::format_count;
use crate::widgets::section::{section, section_with};

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    prefs: &mut SettingsPrefs,
    proxy: &mut ProxyState,
    cert: &mut CertificateState,
    data: &mut DataSectionState,
    database: &mut crate::features::database::DatabaseState,
    form: &mut ListenerForm,
    traffic: &TrafficState,
    credentials: &mut crate::features::credentials::CredentialsState,
    db_path: &Path,
    cmds: &mut Commands,
) {
    let [mut list, mut content] = split::side_by_side(ui, "settings_split", &mut prefs.list_fraction);
    for section in SettingsSection::ALL {
        let selected = prefs.section == section;
        if list_row(&mut list, selected, |ui| {
            ui.label(section.name());
        })
        .clicked()
        {
            prefs.section = section;
        }
    }
    egui::ScrollArea::vertical()
        .id_salt(("settings", prefs.section))
        .auto_shrink([false, false])
        .show(&mut content, |ui| match prefs.section {
            SettingsSection::Proxy => {
                ui.add_enabled_ui(!proxy.busy(), |ui| proxy_section(ui, proxy, form, traffic, db_path, cmds));
            }
            SettingsSection::Certificate => certificate_section(ui, cert, cmds),
            SettingsSection::Appearance => appearance_section(ui),
            SettingsSection::Shortcuts => shortcuts_section(ui),
            SettingsSection::Data => data_section(ui, data, cmds),
            SettingsSection::Credentials => crate::features::credentials::settings_section(ui, credentials, db_path),
            SettingsSection::Database => crate::features::database::show(ui, database, db_path),
        });
}

fn appearance_section(ui: &mut egui::Ui) {
    section(ui, "Theme");
    let mut theme_preference = ui.ctx().options(|opt| opt.theme_preference);
    let choices = [
        (egui::ThemePreference::System, "System"),
        (egui::ThemePreference::Light, "Light"),
        (egui::ThemePreference::Dark, "Dark"),
    ];
    if segmented(ui, &mut theme_preference, &choices) {
        ui.ctx().set_theme(theme_preference);
    }

    section(ui, "Zoom");
    ui.horizontal(|ui| {
        egui::gui_zoom::zoom_menu_buttons(ui);
    });
    ui.add_space(6.0);
    let mut zoom = ui.ctx().zoom_factor();
    if ui.add(egui::Slider::new(&mut zoom, 0.5..=2.5).text("Factor").step_by(0.1)).changed() {
        ui.ctx().set_zoom_factor(zoom);
    }
}

fn shortcuts_section(ui: &mut egui::Ui) {
    egui::Grid::new("shortcut_list").num_columns(3).striped(false).spacing([24.0, 6.0]).show(ui, |ui| {
        for (keys, action, scope) in crate::ui::shortcuts::listing(ui.ctx()) {
            ui.monospace(keys);
            ui.label(action);
            ui.label(RichText::new(scope.unwrap_or_default()).weak());
            ui.end_row();
        }
    });
}

fn data_section(ui: &mut egui::Ui, data: &mut DataSectionState, cmds: &mut Commands) {
    section_with(ui, "Stored data", |ui| {
        if icon_button(ui, icons::ARROW_CLOCKWISE, "Refresh row counts").clicked() {
            cmds.push(Cmd::RefreshDataCounts);
        }
    });

    if let Some((msg, tone)) = &data.status {
        ui.label(RichText::new(msg).color(theme::color(ui, *tone)));
        ui.add_space(4.0);
    }

    let counts = data.counts.clone().unwrap_or_default();

    egui::Grid::new("data_categories_grid")
        .num_columns(3)
        .spacing([24.0, 8.0])
        .striped(false)
        .show(ui, |ui| {
            for title in ["Category", "Rows", ""] {
                ui.label(RichText::new(title).small().weak());
            }
            ui.end_row();

            for category in mitm_core::DataCategory::ALL {
                ui.label(category.display_name());
                let count = counts.get(category);
                ui.monospace(format_count(count as usize));
                let clear_id = ui.id().with(category);
                if crate::widgets::confirm::confirm_button(
                    ui,
                    clear_id,
                    RichText::new("Clear"),
                    "Confirm clear?",
                    &format!("Delete all {}", category.display_name().to_lowercase()),
                ) {
                    cmds.push(Cmd::ClearData(category));
                }
                ui.end_row();
            }
        });

    section(ui, "Danger zone");

    let reset = egui::Button::new(RichText::new("Factory reset…").color(theme::color(ui, Tone::Danger)));
    if ui.add_enabled(!data.resetting, reset).clicked() {
        data.factory_reset_open = true;
        data.factory_reset_input.clear();
    }

    if data.factory_reset_open {
        factory_reset_modal(ui.ctx(), data, cmds);
    }
}

fn factory_reset_modal(ctx: &egui::Context, data: &mut DataSectionState, cmds: &mut Commands) {
    let mut close = false;
    let modal = egui::Modal::new(egui::Id::new("factory_reset_modal")).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.heading("Reset all data");
        ui.add_space(8.0);
        ui.label("Wipes the database and the root CA. Traffic, rules, collections and settings are lost.");
        ui.add_space(8.0);
        ui.add(egui::TextEdit::singleline(&mut data.factory_reset_input).hint_text("Type RESET").desired_width(f32::INFINITY));
        ui.add_space(12.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_reset = data.factory_reset_input.trim() == "RESET";
            if ui.add_enabled(can_reset, danger_filled(ui, "Reset")).clicked() {
                cmds.push(AppCommand::FactoryReset);
                close = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });
    if close || modal.should_close() {
        data.factory_reset_open = false;
        data.factory_reset_input.clear();
    }
}

fn certificate_section(
    ui: &mut egui::Ui,
    cert: &CertificateState,
    cmds: &mut Commands,
) {
    section(ui, "Root certificate");

    ui.horizontal(|ui| {
        ui.menu_button(format!("Save certificate… {}", icons::CARET_DOWN), |ui| {
            if ui.button("PEM format (.crt)…").clicked() {
                cmds.push(Cmd::ExportCa(mitm_core::CaFormat::Pem));
                ui.close();
            }
            if ui.button("DER format (.cer)…").clicked() {
                cmds.push(Cmd::ExportCa(mitm_core::CaFormat::Der));
                ui.close();
            }
        });

        let flash_id = ui.id().with("ca_copy_flash");
        if ui.button("Copy PEM").clicked() {
            cmds.push(Cmd::CopyCaPem);
            crate::widgets::flash::set(ui, flash_id, "PEM copied");
        }
        if let Some(msg) = crate::widgets::flash::current(ui, flash_id) {
            ui.label(RichText::new(msg).color(theme::color(ui, Tone::Success)).strong());
        }

        if ui.button("Open folder").clicked() {
            cmds.push(Cmd::OpenCaFolder);
        }
        ui.add_space(12.0);

        let regen_id = ui.id().with("regenerate_ca");
        let confirmed = ui
            .add_enabled_ui(!cert.regenerating, |ui| {
                crate::widgets::confirm::confirm_button(
                    ui,
                    regen_id,
                    RichText::new("Regenerate CA"),
                    "Confirm regenerate?",
                    "Recreates root CA. Devices will need to trust the new certificate.",
                )
            })
            .inner;
        if confirmed {
            cmds.push(Cmd::RegenerateCa);
        }
    });

    if let Some((msg, tone)) = &cert.status {
        ui.add_space(4.0);
        ui.label(RichText::new(msg).color(theme::color(ui, *tone)));
    }

    section(ui, "Details");

    if let Some(info) = &cert.info {
        egui::Grid::new("ca_details_grid")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .striped(false)
            .show(ui, |ui| {
                detail_row(ui, "Common name", &info.common_name, false);
                detail_row(ui, "Organization", &info.organization, false);
                detail_row(ui, "Valid from", &info.not_before, false);
                detail_row(ui, "Valid until", &info.not_after, false);
                detail_row(ui, "SHA-256", &info.fingerprint, true);
                detail_row(ui, "File", &info.cert_path, true);
            });
    }

    section(ui, "Install and trust");

    let os = std::env::consts::OS;
    let cert_path = cert.info.as_ref().map_or("~/.local/share/mitm/ca/ca.crt", |i| i.cert_path.as_str());

    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), ui.make_persistent_id("trust_linux"), os == "linux")
        .show_header(ui, |ui| { ui.label("Linux"); })
        .body(|ui| {
            let p11_cmd = format!("sudo trust anchor --store \"{}\"", cert_path);
            copyable_command(ui, "linux_p11", "Fedora / Arch / RHEL / p11-kit:", &p11_cmd);
            ui.add_space(4.0);
            let deb_cmd = format!("sudo cp \"{}\" /usr/local/share/ca-certificates/mitm-ca.crt && sudo update-ca-certificates", cert_path);
            copyable_command(ui, "linux_deb", "Debian / Ubuntu:", &deb_cmd);
        });

    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), ui.make_persistent_id("trust_macos"), os == "macos")
        .show_header(ui, |ui| { ui.label("macOS"); })
        .body(|ui| {
            let mac_cmd = format!("sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain \"{}\"", cert_path);
            copyable_command(ui, "macos_trust", "System keychain:", &mac_cmd);
        });

    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), ui.make_persistent_id("trust_windows"), os == "windows")
        .show_header(ui, |ui| { ui.label("Windows"); })
        .body(|ui| {
            let win_cmd = format!("certutil -addstore -f \"Root\" \"{}\"", cert_path);
            copyable_command(ui, "win_certutil", "Certificate store (Run as Administrator):", &win_cmd);
        });

    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), ui.make_persistent_id("trust_firefox"), false)
        .show_header(ui, |ui| { ui.label("Firefox"); })
        .body(|ui| {
            ui.label(RichText::new("Option 1: Allow Firefox to trust OS root certificates:").weak().small());
            ui.monospace("about:config -> set security.enterprise_roots.enabled = true");
            ui.add_space(4.0);
            ui.label(RichText::new("Option 2: Import into Firefox Settings -> Privacy & Security -> Certificates -> View Certificates -> Authorities -> Import…").weak().small());
        });

    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), ui.make_persistent_id("trust_cli"), false)
        .show_header(ui, |ui| { ui.label("CLI / Node.js / Python"); })
        .body(|ui| {
            let node_cmd = format!("export NODE_EXTRA_CA_CERTS=\"{}\"", cert_path);
            copyable_command(ui, "cli_node", "Node.js:", &node_cmd);
            ui.add_space(4.0);
            let ssl_cmd = format!("export SSL_CERT_FILE=\"{}\"", cert_path);
            copyable_command(ui, "cli_python", "Python / OpenSSL:", &ssl_cmd);
            ui.add_space(4.0);
            let curl_cmd = format!("export CURL_CA_BUNDLE=\"{}\"", cert_path);
            copyable_command(ui, "cli_curl", "cURL:", &curl_cmd);
        });
}

fn detail_row(ui: &mut egui::Ui, label: &str, value: &str, copyable: bool) {
    ui.label(RichText::new(label).weak());
    ui.horizontal(|ui| {
        ui.label(RichText::new(value).monospace());
        if copyable && icon_button(ui, icons::COPY, &format!("Copy {}", label.to_lowercase())).clicked() {
            ui.ctx().copy_text(value.to_string());
        }
    });
    ui.end_row();
}

fn copyable_command(ui: &mut egui::Ui, id_salt: &str, label: &str, cmd: &str) {
    ui.label(RichText::new(label).weak().small());
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 60.0).max(100.0);
        ui.add(egui::TextEdit::singleline(&mut cmd.to_string()).font(egui::TextStyle::Monospace).desired_width(width));
        let flash_id = ui.id().with(id_salt);
        if icon_button(ui, icons::COPY, "Copy command").clicked() {
            ui.ctx().copy_text(cmd.to_string());
            crate::widgets::flash::set(ui, flash_id, "Copied");
        }
        if let Some(msg) = crate::widgets::flash::current(ui, flash_id) {
            ui.label(RichText::new(msg).color(theme::color(ui, Tone::Success)).small());
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
    section(ui, "Listeners");

    let mut modified = false;
    let mut to_remove = None;

    egui::Grid::new("listeners_table_grid")
        .striped(false)
        .num_columns(6)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            for title in ["", "On", "Label", "Address", "ID", ""] {
                ui.label(RichText::new(title).small().weak());
            }
            ui.end_row();

            for (idx, listener) in proxy.listeners.iter_mut().enumerate() {
                let status_label = if proxy.listener_errors.contains_key(&listener.id) {
                    RichText::new("●").color(theme::color(ui, Tone::Danger))
                } else if proxy.running_listeners.contains(&listener.id) {
                    RichText::new("●").color(theme::color(ui, Tone::Success))
                } else {
                    RichText::new("●").color(theme::color(ui, Tone::Neutral))
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
                    cmds.push(Cmd::SetListenerEnabled {
                        id: listener.id,
                        enabled: listener.enabled,
                    });
                }

                if ui.add(egui::TextEdit::singleline(&mut listener.label).desired_width(120.0)).lost_focus() {
                    modified = true;
                }

                if ui.add(egui::TextEdit::singleline(&mut listener.address).font(egui::TextStyle::Monospace).desired_width(180.0)).lost_focus() {
                    modified = true;
                }

                ui.label(RichText::new(listener.id.to_string()).monospace().weak());

                if listener.id != 0 {
                    if icon_button(ui, icons::TRASH, "Remove listener").clicked() {
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

    section(ui, "Add listener");
    card(ui, |ui| ui.horizontal_wrapped(|ui| {
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
        if ui.add_enabled(can_add, crate::widgets::button::primary(ui, "Add")).clicked() {
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
    }));

    if modified {
        cmds.push(Cmd::SaveListeners(proxy.listeners.clone()));
    }

    ui.add_space(8.0);
    let id = ui.make_persistent_id("host_addresses_collapsing");
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            ui.label("Host addresses");
            if icon_button(ui, icons::ARROW_CLOCKWISE, "Refresh host addresses").clicked() {
                proxy.reload_host_ips();
            }
        })
        .body(|ui| {
            egui::Grid::new("host_ips_grid")
                .striped(false)
                .num_columns(4)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    for title in ["", "IP", "Interface", ""] {
                        ui.label(RichText::new(title).small().weak());
                    }
                    ui.end_row();

                    for ip_info in &proxy.host_ips {
                        let (dot, tone, tooltip) = if ip_info.is_up {
                            ("●", Tone::Success, "Interface up")
                        } else {
                            ("●", Tone::Neutral, "Interface down")
                        };
                        ui.label(RichText::new(dot).color(theme::color(ui, tone))).on_hover_text(tooltip);

                        let ip_resp = ui.selectable_label(false, RichText::new(&ip_info.ip).monospace());
                        let iface_resp = ui.label(&ip_info.interface);
                        for resp in [ip_resp, iface_resp] {
                            host_ip_menu(&resp, &ip_info.ip, form);
                        }

                        if ip_info.is_link_local {
                            ui.label(RichText::new("link-local").weak().small());
                        } else {
                            ui.label("");
                        }
                        ui.end_row();
                    }
                });
        });

    section(ui, "System");
    egui::Grid::new("system_info_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            let status = if proxy.running { "Running" } else { "Stopped" };
            detail_row(ui, "Proxy", status, false);
            detail_row(ui, "Listening on", &proxy.active_summary, false);
            detail_row(ui, "Flows in memory", &format!("{} / {}", format_count(traffic.flows.len()), format_count(traffic.max_flows)), false);
            detail_row(ui, "Database", &db_path.to_string_lossy(), true);
        });
}

fn host_ip_menu(resp: &egui::Response, ip: &str, form: &mut ListenerForm) {
    resp.context_menu(|ui| {
        if ui.button("Copy").clicked() {
            ui.ctx().copy_text(ip.to_string());
            ui.close();
        }
        if ui.button("Use in new listener").clicked() {
            form.host = ip.to_string();
            ui.close();
        }
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
