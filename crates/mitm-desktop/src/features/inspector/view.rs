use eframe::egui::{self, RichText};
use mitm_core::proxy::history::HistoryEntry;
use crate::app::command::{AppCommand, Commands};
use eframe::egui::containers::menu::MenuButton;
use mitm_core::collections::CollectionTreeItem;
use crate::features::traffic::{flow_menu, FlowFlags, TrafficState};
use crate::util::format::{format_bytes, format_duration};
use crate::ui::theme::Tone;
use crate::widgets::button::icon_button;
use crate::widgets::pills::{badge, render_method_pill, render_status_pill};
use crate::widgets::split;
use crate::widgets::message_pane::{self as pane, PaneData};
use crate::features::request_editor::body::captured_form;
use super::{header_values, query_params, request_cookies, response_cookies, InspectorPrefs, InspectorState};
use crate::ui::icons;

pub fn show(ui: &mut egui::Ui, state: &mut InspectorState, traffic: &TrafficState, folders: &[CollectionTreeItem], cmds: &mut Commands) {
    let Some(flow) = traffic.selected_flow() else { return };

    let detail = traffic.selected_detail.as_ref().filter(|d| d.id == flow.id);
    let flags = FlowFlags::of(flow);

    summary(ui, flow, &flags, &mut state.prefs, folders, cmds);
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

/// Method, status and stats, then the URL filling the space up to the controls.
fn summary(ui: &mut egui::Ui, flow: &HistoryEntry, flags: &FlowFlags, prefs: &mut InspectorPrefs, folders: &[CollectionTreeItem], cmds: &mut Commands) {
    ui.horizontal(|ui| {
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
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !prefs.popped_out && icon_button(ui, icons::X, "Hide inspector").clicked() {
                prefs.open = false;
            }
            let (icon, hint) = if prefs.popped_out {
                (icons::ARROW_SQUARE_IN, "Put back into Traffic")
            } else {
                (icons::ARROW_SQUARE_OUT, "Detach into its own panel")
            };
            if icon_button(ui, icon, hint).clicked() {
                cmds.push(AppCommand::DetachInspector(!prefs.popped_out));
            }
            let side = ui.spacing().interact_size.y;
            let dots = egui::Button::new(icons::DOTS_THREE).frame_when_inactive(false).min_size(egui::vec2(side, side));
            let (response, _) = MenuButton::from_button(dots).ui(ui, |ui| flow_menu(ui, flow, folders, cmds));
            response.on_hover_text("Actions");
            url_bar(ui, &flow.url);
        });
    });
}

/// The URL in a read-only `base` well, with a copy button inside it.
fn url_bar(ui: &mut egui::Ui, url: &str) {
    let visuals = ui.visuals().clone();
    egui::Frame::new()
        .fill(visuals.extreme_bg_color)
        .stroke(visuals.widgets.inactive.bg_stroke)
        .corner_radius(visuals.widgets.inactive.corner_radius)
        .inner_margin(egui::Margin::symmetric(8, 0))
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
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
