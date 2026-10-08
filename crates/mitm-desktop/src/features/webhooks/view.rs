use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::state::WebhookDelivery;
use crate::command::{AppCommand, Commands};
use crate::format::format_bytes;
use crate::theme::{self, Tone};
use crate::widgets::confirm::confirm_button;
use crate::widgets::split;
use super::{EndpointDraft, EndpointFilter, WebhooksState};

pub fn show(ui: &mut egui::Ui, state: &mut WebhooksState, cmds: &mut Commands) {
    toolbar(ui, state, cmds);
    ui.separator();

    let mut fraction = state.prefs.list_split;
    let [mut left, mut right] = split::side_by_side(ui, "wh_list_split", &mut fraction);
    state.prefs.list_split = fraction;
    endpoints_list(&mut left, state, cmds);
    deliveries(&mut right, state, cmds);
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

fn deliveries(ui: &mut egui::Ui, state: &mut WebhooksState, cmds: &mut Commands) {
    ui.horizontal(|ui| {
        ui.heading("Deliveries");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if confirm_button(
                ui,
                ui.id().with("wh_clear"),
                RichText::new("Clear"),
                "Clear all?",
                "Delete every delivery",
            ) {
                cmds.push(AppCommand::ClearWebhookDeliveries);
            }
            if !state.search.is_empty() && ui.small_button("✕").on_hover_text("Clear search").clicked() {
                state.search.clear();
            }
            ui.add(egui::TextEdit::singleline(&mut state.search).hint_text("Search").desired_width(140.0));
        });
    });
    ui.add_space(2.0);

    let query = state.search.to_lowercase();
    let rows: Vec<usize> = state
        .deliveries
        .iter()
        .enumerate()
        .filter(|(_, d)| endpoint_matches(&state.filter, d))
        .filter(|(_, d)| {
            query.is_empty()
                || d.endpoint_path.to_lowercase().contains(&query)
                || d.payload.to_lowercase().contains(&query)
        })
        .map(|(i, _)| i)
        .collect();

    if rows.is_empty() {
        ui.weak("No deliveries");
        return;
    }

    let row_height = egui::TextStyle::Body.resolve(ui.style()).size + 6.0;
    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(75.0))
        .column(Column::remainder())
        .column(Column::exact(70.0))
        .column(Column::exact(40.0))
        .header(22.0, |mut header| {
            header.col(|ui| { ui.strong("Time"); });
            header.col(|ui| { ui.strong("Path"); });
            header.col(|ui| { ui.strong("Size"); });
            header.col(|ui| { ui.strong("Sig"); });
        })
        .body(|body| {
            body.rows(row_height, rows.len(), |mut row| {
                let delivery = &state.deliveries[rows[row.index()]];
                row.col(|ui| { ui.label(clock(delivery.timestamp)); });
                row.col(|ui| { ui.add(egui::Label::new(&delivery.endpoint_path).truncate()); });
                row.col(|ui| { ui.label(format_bytes(delivery.payload.len() as u64)); });
                row.col(|ui| { signature_chip(ui, &delivery.signature_status); });
            });
        });
}

fn endpoint_matches(filter: &EndpointFilter, delivery: &WebhookDelivery) -> bool {
    match filter {
        EndpointFilter::All => true,
        EndpointFilter::Unmatched => delivery.endpoint_id == "unmatched",
        EndpointFilter::Endpoint(id) => &delivery.endpoint_id == id,
    }
}

fn signature_chip(ui: &mut egui::Ui, status: &str) {
    let (text, tone) = match status {
        "valid" => ("✓", Tone::Success),
        "invalid" => ("✗", Tone::Danger),
        _ => ("–", Tone::Neutral),
    };
    ui.label(RichText::new(text).color(theme::color(ui, tone)));
}

fn clock(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|at| at.with_timezone(&chrono::Local).format("%H:%M:%S").to_string())
        .unwrap_or_default()
}
