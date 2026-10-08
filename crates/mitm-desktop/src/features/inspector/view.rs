use eframe::egui;
use mitm_core::state::HistoryEntry;
use crate::command::{AppCommand, Commands};
use crate::features::traffic::{FlowFlags, TrafficState};
use crate::format::{format_bytes, format_duration};
use crate::theme::{self, Tone};
use crate::widgets::pills::{method_text, status_text};
use crate::widgets::split;
use crate::widgets::message_pane::{self as pane, PaneData};
use crate::features::request_editor::body::captured_form;
use super::{header_values, query_params, request_cookies, response_cookies, InspectorState};

pub fn show(ui: &mut egui::Ui, state: &mut InspectorState, traffic: &TrafficState, cmds: &mut Commands) {
    let Some(flow) = traffic.selected_flow() else { return };

    let detail = traffic.selected_detail.as_ref().filter(|d| d.id == flow.id);
    let flags = FlowFlags::of(flow);

    summary(ui, flow, &flags, &mut state.prefs.open);
    url_bar(ui, &flow.url);
    ui.separator();

    let request_headers = detail.map_or(&flow.request_headers, |d| &d.request_headers);
    let response_headers = detail.map_or(&flow.response_headers, |d| &d.response_headers);
    let request = PaneData {
        cache_key: flow.id as u64,
        title: "Request",
        headers: request_headers,
        body: detail.map_or(&flow.request_body, |d| &d.request_body),
        content_type: header_value(request_headers, "content-type"),
        params: Some(query_params(&flow.url)),
        cookies: Some(request_cookies(request_headers)),
        form_parser: Some(captured_form),
    };
    let response = PaneData {
        cache_key: flow.id as u64,
        title: "Response",
        headers: response_headers,
        body: detail.map_or(&flow.response_body, |d| &d.response_body),
        content_type: &flow.content_type,
        params: None,
        cookies: Some(response_cookies(response_headers)),
        form_parser: Some(captured_form),
    };

    let [mut request_ui, mut response_ui] = split::side_by_side(ui, "inspector_split", &mut state.prefs.split);
    let saves = [
        pane::show(&mut request_ui, "inspector_request", &mut state.request, &request),
        pane::show(&mut response_ui, "inspector_response", &mut state.response, &response),
    ];
    for save in saves.into_iter().flatten() {
        cmds.push(AppCommand::SaveBody(save));
    }
}

fn summary(ui: &mut egui::Ui, flow: &HistoryEntry, flags: &FlowFlags, open: &mut bool) {
    egui::Sides::new().shrink_left().truncate().show(
        ui,
        |ui| {
            ui.label(method_text(ui.visuals(), &flow.method));
            match (flags.pending_intercept, flags.pending_response) {
                (true, _) => ui.colored_label(theme::color(ui, Tone::Warn), "Paused in interceptor"),
                (_, true) => ui.colored_label(theme::color(ui, Tone::Danger), "Waiting for response…"),
                _ => ui.label(status_text(ui.visuals(), flow.status_code)),
            };
            if flags.failed {
                ui.colored_label(theme::color(ui, Tone::Danger), "Failed");
            }
            if flags.intercepted {
                ui.colored_label(theme::color(ui, Tone::Warn), "Intercepted");
            }
            if flags.rewritten {
                ui.colored_label(theme::color(ui, Tone::Info), "Rewritten");
            }
            if !flow.listener_label.is_empty() && flow.listener_label != "Default" {
                ui.weak(format!("via {}", flow.listener_label));
            }
        },
        |ui| {
            if ui.small_button("✖").on_hover_text("Hide inspector").clicked() {
                *open = false;
            }
            if !flags.is_pending() {
                ui.weak(format!("{} · {}", format_duration(flow.duration_ms), format_bytes(flow.response_size)));
            }
        },
    );
}

fn url_bar(ui: &mut egui::Ui, url: &str) {
    let mut display = url;
    ui.add(
        egui::TextEdit::singleline(&mut display)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Monospace),
    );
}

fn header_value<'a>(headers: &'a [(String, String)], name: &'a str) -> &'a str {
    header_values(headers, name).next().unwrap_or_default()
}
