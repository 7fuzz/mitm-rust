use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::state::WebhookDelivery;
use crate::app::command::{AppCommand, Commands};
use super::Cmd;
use crate::util::format::format_bytes;
use crate::ui::theme::{self, Tone};
use crate::widgets::confirm::confirm_button;
use crate::widgets::message_pane::{self, PaneData};
use crate::widgets::pills::badge;
use crate::widgets::split;
use super::{EndpointDraft, EndpointFilter, WebhooksState};
use crate::ui::icons;
use crate::widgets::button::{icon_button, primary};
use crate::widgets::card::card;
use crate::widgets::search::search_field_sized;
use crate::widgets::section::section_with;

pub fn show(ui: &mut egui::Ui, state: &mut WebhooksState, cmds: &mut Commands) {
    toolbar(ui, state, cmds);
    ui.add_space(4.0);

    let mut fraction = state.prefs.list_split;
    let [mut left, mut right] = split::side_by_side(ui, "wh_list_split", &mut fraction);
    state.prefs.list_split = fraction;
    endpoints_list(&mut left, state, cmds);
    deliveries(&mut right, state, cmds);
}

fn toolbar(ui: &mut egui::Ui, state: &mut WebhooksState, cmds: &mut Commands) {
    ui.horizontal(|ui| {
        let (tone, status) = if state.running { (Tone::Success, "Listening") } else { (Tone::Neutral, "Stopped") };
        ui.label(RichText::new("●").color(theme::color(ui, tone)));
        ui.label(status);
        ui.add_enabled_ui(!state.switching, |ui| {
            if state.running {
                if ui.button("Stop").clicked() {
                    cmds.push(Cmd::StopReceiver);
                }
            } else if ui.add(primary(ui, "Start")).on_hover_text("Accept incoming webhook deliveries").clicked() {
                cmds.push(Cmd::StartReceiver);
            }
        });
        ui.add_space(8.0);
        ui.weak("Port");

        ui.add_enabled_ui(!state.running, |ui| {
            let port = ui
                .add(egui::DragValue::new(&mut state.port).range(1..=65535))
                .on_hover_text("Stop the receiver to change the port");
            if port.changed() {
                cmds.push(Cmd::SetPort(state.port));
            }
        });

        ui.add_space(8.0);
        let url = state.base_url();
        ui.label(RichText::new(&url).monospace().weak());
        if icon_button(ui, icons::COPY, "Copy base URL").clicked() {
            ui.ctx().copy_text(url);
        }
    });
}

fn endpoints_list(ui: &mut egui::Ui, state: &mut WebhooksState, cmds: &mut Commands) {
    section_with(ui, "Endpoints", |ui| {
        if icon_button(ui, icons::PLUS, "Add endpoint").clicked() && state.draft.is_none() {
            state.draft = Some(EndpointDraft { path: "/".to_string(), ..Default::default() });
        }
    });

    let WebhooksState { endpoints, filter, draft, port, .. } = state;
    let base = format!("http://localhost:{}", port);

    ui.selectable_value(filter, EndpointFilter::All, "All");
    ui.selectable_value(filter, EndpointFilter::Unmatched, "Unmatched");
    ui.add_space(4.0);

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
                action = Some(Cmd::CopyTestCurl(endpoint.id.clone()));
                ui.close();
            }
            ui.separator();
            if ui.button(RichText::new("Delete").color(theme::color(ui, Tone::Danger))).clicked() {
                action = Some(Cmd::DeleteEndpoint(endpoint.id.clone()));
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
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut draft.name).hint_text("Name").desired_width(110.0));
                ui.add(egui::TextEdit::singleline(&mut draft.path).hint_text("/path").desired_width(110.0));
                ui.add(egui::TextEdit::singleline(&mut draft.secret).hint_text("Secret").desired_width(140.0));
                let valid = !draft.name.trim().is_empty() && draft.path.starts_with('/') && draft.path.len() > 1;
                if ui
                    .add_enabled(valid, primary(ui, "Add"))
                    .on_disabled_hover_text("Name and a /path are required")
                    .clicked()
                {
                    cmds.push(Cmd::CreateEndpoint {
                        name: draft.name.trim().to_string(),
                        path: draft.path.trim().to_string(),
                        secret: draft.secret.clone(),
                    });
                }
                if icon_button(ui, icons::X, "Cancel").clicked() {
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
    section_with(ui, "Deliveries", |ui| {
        if confirm_button(ui, ui.id().with("wh_clear"), RichText::new(icons::TRASH), "Clear all?", "Delete every delivery") {
            cmds.push(Cmd::ClearDeliveries);
        }
        search_field_sized(ui, &mut state.search, "Search path or payload", 200.0);
    });

    let mut fraction = state.prefs.detail_split;
    let [mut top, mut bottom] = split::stacked(ui, "wh_detail_split", &mut fraction);
    state.prefs.detail_split = fraction;
    deliveries_table(&mut top, state);
    delivery_detail(&mut bottom, state, cmds);
}

fn deliveries_table(ui: &mut egui::Ui, state: &mut WebhooksState) {
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
        ui.weak("No deliveries yet");
        return;
    }

    let row_height = egui::TextStyle::Body.resolve(ui.style()).size + 6.0;
    let mut new_selection = None;
    TableBuilder::new(ui)
        .striped(false)
        .resizable(true)
        .sense(egui::Sense::click())
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(75.0))
        .column(Column::remainder())
        .column(Column::exact(70.0))
        .column(Column::exact(40.0))
        .header(22.0, |mut header| {
            for title in ["Time", "Path", "Size", "Sig"] {
                header.col(|ui| {
                    ui.label(RichText::new(title).small().weak());
                });
            }
        })
        .body(|body| {
            body.rows(row_height, rows.len(), |mut row| {
                let delivery = &state.deliveries[rows[row.index()]];
                row.set_selected(state.selected_delivery == Some(delivery.id));
                row.col(|ui| { ui.label(RichText::new(clock(delivery.timestamp)).monospace().weak()); });
                row.col(|ui| { ui.add(egui::Label::new(RichText::new(&delivery.endpoint_path).monospace()).truncate()); });
                row.col(|ui| { ui.label(RichText::new(format_bytes(delivery.payload.len() as u64)).monospace().weak()); });
                row.col(|ui| { signature_chip(ui, &delivery.signature_status); });
                if row.response().clicked() {
                    new_selection = Some(delivery.id);
                }
            });
        });

    if let Some(id) = new_selection {
        state.selected_delivery = Some(id);
    }
}

fn delivery_detail(ui: &mut egui::Ui, state: &mut WebhooksState, cmds: &mut Commands) {
    let WebhooksState { deliveries, endpoints, selected_delivery, detail_pane, replay_result, prefs, .. } = state;
    let Some(id) = *selected_delivery else {
        ui.weak("Select a delivery");
        return;
    };
    let Some(delivery) = deliveries.iter().find(|d| d.id == id) else {
        ui.weak("Select a delivery");
        return;
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new(&delivery.endpoint_path).monospace().strong());
        ui.label(RichText::new(format!("{} · {}", clock(delivery.timestamp), format_bytes(delivery.payload.len() as u64))).monospace().weak());
        if delivery.endpoint_id == "unmatched" {
            badge(ui, "UNMATCHED", Tone::Warn);
        }
    });
    signature_view(ui, delivery);

    if ui.button("Recompute with…").on_hover_text("Open the HMAC signature tool prefilled").clicked() {
        let secret = endpoints
            .iter()
            .find(|e| e.id == delivery.endpoint_id)
            .map(|e| e.secret_key.clone())
            .unwrap_or_default();
        cmds.push(AppCommand::RecomputeHmac { secret, payload: delivery.payload.clone() });
    }

    ui.horizontal(|ui| {
        let url = prefs.replay_urls.entry(delivery.endpoint_id.clone()).or_default();
        ui.add(egui::TextEdit::singleline(url).hint_text("Replay to URL").desired_width(240.0));
        let can_replay = url.trim_start().starts_with("http");
        if ui.add_enabled(can_replay, egui::Button::new("Replay")).clicked() {
            cmds.push(Cmd::Replay { id, url: url.trim().to_string() });
        }
        if let Some((_, result)) = replay_result.as_ref().filter(|(rid, _)| *rid == id) {
            match result {
                Ok(res) => {
                    let tone = if res.success { Tone::Success } else { Tone::Danger };
                    ui.label(RichText::new(format!("{} · {} ms", res.status_code, res.duration_ms)).monospace().color(theme::color(ui, tone)));
                }
                Err(e) => {
                    ui.label(RichText::new(e).color(theme::color(ui, Tone::Danger)));
                }
            }
        }
    });
    ui.add_space(4.0);

    let data = PaneData {
        cache_key: delivery.id as u64,
        title: "",
        headers: &delivery.headers,
        body: &delivery.payload,
        content_type: content_type(&delivery.headers),
        params: None,
        cookies: None,
        form_parser: None,
    };
    if let Some(save) = message_pane::show(ui, "wh_delivery_pane", detail_pane, &data) {
        cmds.push(AppCommand::SaveBody(save));
    }
}

fn signature_view(ui: &mut egui::Ui, delivery: &WebhookDelivery) {
    ui.horizontal(|ui| {
        ui.weak("Signature");
        match delivery.signature_status.as_str() {
            "valid" => badge(ui, "VALID", Tone::Success),
            "invalid" => badge(ui, "INVALID", Tone::Danger),
            _ => badge(ui, "NONE", Tone::Neutral),
        };
    });
    if let Some(provided) = &delivery.provided_hmac {
        ui.horizontal(|ui| {
            ui.weak("received");
            ui.add(egui::Label::new(RichText::new(provided).monospace()).truncate());
        });
    }
    if let Some(expected) = &delivery.computed_hmac {
        ui.horizontal(|ui| {
            ui.weak("expected");
            ui.add(egui::Label::new(RichText::new(expected).monospace()).truncate());
        });
    }
}

fn content_type(headers: &[(String, String)]) -> &'static str {
    let raw = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");
    if raw.contains("json") {
        "application/json"
    } else if raw.contains("xml") {
        "application/xml"
    } else {
        "text/plain"
    }
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
        "valid" => (icons::CHECK, Tone::Success),
        "invalid" => (icons::X, Tone::Danger),
        _ => ("–", Tone::Neutral),
    };
    ui.label(RichText::new(text).color(theme::color(ui, tone)));
}

fn clock(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|at| at.with_timezone(&chrono::Local).format("%H:%M:%S").to_string())
        .unwrap_or_default()
}
