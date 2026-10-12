use eframe::egui::{self, RichText};
use mitm_core::state::HistoryEntry;
use crate::app::command::{AppCommand, Commands};
use crate::features::traffic::{FlowFlags, TrafficState};
use crate::util::format::{format_bytes, format_duration};
use crate::ui::theme::Tone;
use crate::widgets::button::icon_button;
use crate::widgets::pills::{badge, render_method_pill, render_status_pill};
use crate::widgets::split;
use crate::widgets::message_pane::{self as pane, PaneData};
use crate::features::request_editor::body::captured_form;
use super::{header_values, query_params, request_cookies, response_cookies, InspectorPrefs, InspectorState};
use crate::ui::icons;

pub fn show(ui: &mut egui::Ui, state: &mut InspectorState, traffic: &TrafficState, cmds: &mut Commands) {
    let Some(flow) = traffic.selected_flow() else { return };

    let detail = traffic.selected_detail.as_ref().filter(|d| d.id == flow.id);
    let flags = FlowFlags::of(flow);

    summary(ui, flow, &flags, &mut state.prefs, cmds);
    url_bar(ui, &flow.url);
    ui.add_space(4.0);

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

/// The inspector in its own OS window, e.g. on a second monitor; closing it docks the inspector back.
pub fn show_window(ctx: &egui::Context, state: &mut InspectorState, traffic: &TrafficState, cmds: &mut Commands) {
    if !state.prefs.popped_out {
        return;
    }
    let builder = egui::ViewportBuilder::default().with_title("Inspector").with_inner_size([960.0, 720.0]);
    ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("inspector_window"), builder, |ui, _| {
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            show(ui, state, traffic, cmds);
        });
        if ui.ctx().input(|i| i.viewport().close_requested()) {
            state.prefs.popped_out = false;
        }
    });
}

fn summary(ui: &mut egui::Ui, flow: &HistoryEntry, flags: &FlowFlags, prefs: &mut InspectorPrefs, cmds: &mut Commands) {
    egui::Sides::new().shrink_left().truncate().show(
        ui,
        |ui| {
            render_method_pill(ui, &flow.method);
            match (flags.pending_intercept, flags.pending_response) {
                (true, _) => {
                    badge(ui, "Paused in interceptor", Tone::Warn);
                }
                (_, true) => {
                    badge(ui, "Waiting for response", Tone::Danger);
                }
                _ => render_status_pill(ui, flow.status_code),
            }
            if flags.failed {
                badge(ui, "Failed", Tone::Danger);
            }
            if flags.intercepted {
                badge(ui, "Intercepted", Tone::Warn);
            }
            if flags.rewritten {
                badge(ui, "Rewritten", Tone::Info);
            }
            if !flags.is_pending() {
                let stats = format!("{} · {}", format_duration(flow.duration_ms), format_bytes(flow.response_size));
                ui.label(RichText::new(stats).small().family(egui::FontFamily::Monospace).weak());
            }
            if !flow.listener_label.is_empty() && flow.listener_label != "Default" {
                ui.weak(format!("via {}", flow.listener_label));
            }
        },
        |ui| {
            if !prefs.popped_out && icon_button(ui, icons::X, "Hide inspector").clicked() {
                prefs.open = false;
            }
            let (icon, hint) = if prefs.popped_out {
                (icons::ARROW_SQUARE_IN, "Dock back into the main window")
            } else {
                (icons::ARROW_SQUARE_OUT, "Open in its own window")
            };
            if icon_button(ui, icon, hint).clicked() {
                prefs.popped_out = !prefs.popped_out;
            }
            if ui.button("Fuzzer").on_hover_text("Send to Fuzzer").clicked() {
                cmds.push(AppCommand::SendFlowToFuzzer(flow.id));
            }
            if ui.button("Repeater").on_hover_text("Send to Repeater").clicked() {
                cmds.push(AppCommand::SendFlowToRepeater(flow.id));
            }
        },
    );
}

/// The URL in a read-only `base` well, with a copy button inside it.
fn url_bar(ui: &mut egui::Ui, url: &str) {
    let visuals = ui.visuals().clone();
    egui::Frame::new()
        .fill(visuals.extreme_bg_color)
        .stroke(visuals.widgets.inactive.bg_stroke)
        .corner_radius(visuals.widgets.inactive.corner_radius)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let side = ui.spacing().interact_size.y;
                let mut display = url;
                ui.add(
                    egui::TextEdit::singleline(&mut display)
                        .frame(egui::Frame::NONE)
                        .desired_width(ui.available_width() - side - 4.0)
                        .font(egui::TextStyle::Monospace),
                );
                if icon_button(ui, icons::COPY, "Copy URL").clicked() {
                    ui.ctx().copy_text(url.to_string());
                }
            });
        });
}

fn header_value<'a>(headers: &'a [(String, String)], name: &'a str) -> &'a str {
    header_values(headers, name).next().unwrap_or_default()
}
