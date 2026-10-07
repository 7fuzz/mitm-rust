use eframe::egui::{self, RichText};
use mitm_core::state::HistoryEntry;
use crate::features::traffic::{flow_status_badge, FlowFlags, TrafficState};
use crate::format::{format_bytes, format_duration};
use crate::widgets::pills::{badge, render_method_pill, render_source_pill, AMBER, ROSE, SKY};
use crate::widgets::split;
use crate::widgets::message_pane::{self as pane, PaneData};
use super::{query_params, request_cookies, response_cookies, InspectorState};

pub fn show(ui: &mut egui::Ui, state: &mut InspectorState, traffic: &TrafficState) {
    let Some(flow) = traffic.selected_flow() else {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(RichText::new("No request selected").weak());
        });
        return;
    };

    let detail = traffic.selected_detail.as_ref().filter(|d| d.id == flow.id);
    let flags = FlowFlags::of(flow);

    summary(ui, flow, &flags);
    url_bar(ui, &flow.url);
    ui.separator();

    let request_headers = detail.map_or(&flow.request_headers, |d| &d.request_headers);
    let response_headers = detail.map_or(&flow.response_headers, |d| &d.response_headers);
    let request = PaneData {
        cache_key: flow.id as u64,
        title: "Request",
        headers: request_headers,
        body: detail.map_or(&flow.request_body, |d| &d.request_body),
        content_type: "",
        params: Some(query_params(&flow.url)),
        cookies: Some(request_cookies(request_headers)),
    };
    let response = PaneData {
        cache_key: flow.id as u64,
        title: "Response",
        headers: response_headers,
        body: detail.map_or(&flow.response_body, |d| &d.response_body),
        content_type: &flow.content_type,
        params: None,
        cookies: Some(response_cookies(response_headers)),
    };

    split::horizontal(
        ui,
        "inspector_split",
        &mut state.split,
        |ui| pane::show(ui, "inspector_request", &mut state.request, &request),
        |ui| pane::show(ui, "inspector_response", &mut state.response, &response),
    );
}

fn summary(ui: &mut egui::Ui, flow: &HistoryEntry, flags: &FlowFlags) {
    ui.horizontal(|ui| {
        render_method_pill(ui, &flow.method);
        flow_status_badge(ui, flow.status_code, flags);
        if !flow.listener_label.is_empty() && flow.listener_label != "Default" {
            render_source_pill(ui, &flow.listener_label);
        }

        if flags.pending_intercept {
            badge(ui, "PAUSED IN INTERCEPTOR", AMBER);
        }
        if flags.pending_response {
            badge(ui, "WAITING FOR RESPONSE...", ROSE);
        }
        if flags.intercepted {
            badge(ui, "INTERCEPTED", AMBER);
        }
        if flags.rewritten {
            badge(ui, "REWRITTEN", SKY);
        }
        if flags.failed {
            badge(ui, "FAILED (NO RESPONSE)", ROSE);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (size, duration) = if flags.is_pending() {
                ("-".to_string(), "...".to_string())
            } else {
                (format_bytes(flow.response_size), format_duration(flow.duration_ms))
            };
            ui.label(RichText::new(size).weak().size(11.0));
            ui.label(RichText::new(duration).weak().size(11.0));
        });
    });
}

fn url_bar(ui: &mut egui::Ui, url: &str) {
    let mut display = url;
    ui.add(
        egui::TextEdit::singleline(&mut display)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Monospace),
    );
}
