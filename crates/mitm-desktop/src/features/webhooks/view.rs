use eframe::egui::{self, RichText};
use crate::command::{AppCommand, Commands};
use crate::theme::{self, Tone};
use super::{EndpointDraft, EndpointFilter, WebhooksState};

pub fn show(ui: &mut egui::Ui, state: &mut WebhooksState, cmds: &mut Commands) {
    toolbar(ui, state, cmds);
    ui.separator();
    endpoints_list(ui, state, cmds);
}

fn toolbar(ui: &mut egui::Ui, state: &mut WebhooksState, cmds: &mut Commands) {
    ui.horizontal(|ui| {
        let toggle = ui
            .toggle_value(&mut state.running, "Receiver")
            .on_hover_text("Accept incoming webhook deliveries");
        if toggle.changed() {
            cmds.push(if state.running {
                AppCommand::StartWebhookReceiver
            } else {
                AppCommand::StopWebhookReceiver
            });
        }

        ui.add_enabled_ui(!state.running, |ui| {
            let port = ui
                .add(egui::DragValue::new(&mut state.port).range(1..=65535))
                .on_hover_text("Stop the receiver to change the port");
            if port.changed() {
                cmds.push(AppCommand::SetWebhookPort(state.port));
            }
        });

        ui.separator();
        let url = state.base_url();
        ui.monospace(&url);
        if ui.small_button("📋").on_hover_text("Copy base URL").clicked() {
            ui.ctx().copy_text(url);
        }
    });
}

fn endpoints_list(ui: &mut egui::Ui, state: &mut WebhooksState, cmds: &mut Commands) {
    ui.horizontal(|ui| {
        ui.heading("Endpoints");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("➕").on_hover_text("Add endpoint").clicked() && state.draft.is_none() {
                state.draft = Some(EndpointDraft { path: "/".to_string(), ..Default::default() });
            }
        });
    });

    let WebhooksState { endpoints, filter, draft, port, .. } = state;
    let base = format!("http://localhost:{}", port);

    ui.selectable_value(filter, EndpointFilter::All, "All");
    ui.selectable_value(filter, EndpointFilter::Unmatched, "Unmatched");
    ui.separator();

    let mut action = None;
    for endpoint in endpoints.iter() {
        let url = format!("{}{}", base, endpoint.path);
        let hover = format!("{}\nHits: {}", url, endpoint.hit_count);
        let response = ui
            .selectable_value(filter, EndpointFilter::Endpoint(endpoint.id.clone()), &endpoint.name)
            .on_hover_text(hover);
        response.context_menu(|ui| {
            if ui.button("Copy URL").clicked() {
                ui.ctx().copy_text(url.clone());
                ui.close();
            }
            if ui.button("Copy secret").clicked() {
                ui.ctx().copy_text(endpoint.secret_key.clone());
                ui.close();
            }
            if ui.button("Copy test cURL").clicked() {
                action = Some(AppCommand::CopyWebhookTestCurl(endpoint.id.clone()));
                ui.close();
            }
            ui.separator();
            if ui.button(RichText::new("🗑 Delete").color(theme::color(ui, Tone::Danger))).clicked() {
                action = Some(AppCommand::DeleteWebhookEndpoint(endpoint.id.clone()));
                ui.close();
            }
        });
    }

    if let Some(action) = action {
        cmds.push(action);
    }

    let mut cancel = false;
    if let Some(draft) = draft.as_mut() {
        ui.add_space(4.0);
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut draft.name).hint_text("Name").desired_width(110.0));
                ui.add(egui::TextEdit::singleline(&mut draft.path).hint_text("/path").desired_width(110.0));
                ui.add(egui::TextEdit::singleline(&mut draft.secret).hint_text("Secret").desired_width(140.0));
                let valid = !draft.name.trim().is_empty() && draft.path.starts_with('/') && draft.path.len() > 1;
                if ui
                    .add_enabled(valid, egui::Button::new("Add"))
                    .on_disabled_hover_text("Name and a /path are required")
                    .clicked()
                {
                    cmds.push(AppCommand::CreateWebhookEndpoint {
                        name: draft.name.trim().to_string(),
                        path: draft.path.trim().to_string(),
                        secret: draft.secret.clone(),
                    });
                }
                if ui.button("✕").on_hover_text("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
    }
    if cancel {
        *draft = None;
    }
}
