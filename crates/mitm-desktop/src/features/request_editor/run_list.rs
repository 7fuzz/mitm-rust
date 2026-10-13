use eframe::egui::{self, RichText};
use mitm_core::collections::RequestHistoryItem;
use mitm_core::repeater::{HeaderItem, RepeaterHistoryItem};
use crate::util::copy::Exchange;
use crate::widgets::copy_menu;
use crate::widgets::message_pane::{self, PaneData, PaneState};
use crate::widgets::pills::status_text;
use crate::widgets::split;
use super::body::captured_form;

pub trait Run {
    fn id(&self) -> i64;
    fn method(&self) -> &str;
    fn url(&self) -> &str;
    fn status_code(&self) -> u16;
    fn duration_ms(&self) -> u64;
    fn executed_at_ms(&self) -> i64;
    fn request_headers(&self) -> &[HeaderItem];
    fn request_body(&self) -> &str;
    fn response_headers(&self) -> &[HeaderItem];
    fn response_body(&self) -> &str;

    fn exchange(&self) -> Exchange {
        let content_type = self
            .response_headers()
            .iter()
            .find(|h| h.key.eq_ignore_ascii_case("content-type"))
            .map_or(String::new(), |h| h.value.clone());
        Exchange {
            method: self.method().to_string(),
            url: self.url().to_string(),
            request_headers: self.request_headers().iter().map(|h| (h.key.clone(), h.value.clone())).collect(),
            request_body: self.request_body().to_string(),
            status_code: self.status_code(),
            response_headers: self.response_headers().iter().map(|h| (h.key.clone(), h.value.clone())).collect(),
            response_body: self.response_body().to_string(),
            content_type,
        }
    }
}

macro_rules! run_impl {
    ($ty:ty) => {
        impl Run for $ty {
            fn id(&self) -> i64 { self.id }
            fn method(&self) -> &str { &self.method }
            fn url(&self) -> &str { &self.url }
            fn status_code(&self) -> u16 { self.status_code }
            fn duration_ms(&self) -> u64 { self.duration_ms }
            fn executed_at_ms(&self) -> i64 { self.executed_at_ms }
            fn request_headers(&self) -> &[HeaderItem] { &self.request_headers }
            fn request_body(&self) -> &str { self.request_body.as_deref().unwrap_or_default() }
            fn response_headers(&self) -> &[HeaderItem] { &self.response_headers }
            fn response_body(&self) -> &str { self.response_body.as_deref().unwrap_or_default() }
        }
    };
}

run_impl!(RepeaterHistoryItem);
run_impl!(RequestHistoryItem);

pub enum RunAction {
    Select(i64),
    Load(i64),
}

pub fn local_time(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|at| at.with_timezone(&chrono::Local).format("%H:%M:%S").to_string())
        .unwrap_or_default()
}

pub fn url_path(url: &str) -> &str {
    let without_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    without_scheme.find('/').map_or("/", |i| &without_scheme[i..])
}

/// The run open in the details window, with the panes' view state.
pub struct RunDetails {
    pub open: Option<i64>,
    request: PaneState,
    response: PaneState,
    split: f32,
}

impl Default for RunDetails {
    fn default() -> Self {
        Self { open: None, request: PaneState::default(), response: PaneState::default(), split: 0.5 }
    }
}

pub fn show<R: Run>(
    ui: &mut egui::Ui,
    id_salt: &str,
    runs: &[R],
    selected: Option<i64>,
    can_load: bool,
    details: &mut RunDetails,
) -> Option<RunAction> {
    if runs.is_empty() {
        ui.weak("No runs yet");
        return None;
    }

    let mut action = details_window(ui.ctx(), id_salt, runs, can_load, details);
    egui::ScrollArea::vertical().id_salt(id_salt).auto_shrink([false, false]).show(ui, |ui| {
        for run in runs {
            let style = ui.style().clone();
            let mut text = egui::text::LayoutJob::default();
            status_text(ui.visuals(), run.status_code()).append_to(&mut text, &style, egui::FontSelection::Default, egui::Align::Center);
            RichText::new(format!("  {} {}", run.method(), url_path(run.url())))
                .monospace()
                .append_to(&mut text, &style, egui::FontSelection::Default, egui::Align::Center);

            let response = ui
                .add(
                    egui::Button::selectable(selected == Some(run.id()), text)
                        .right_text(RichText::new(local_time(run.executed_at_ms())).weak().small())
                        .truncate()
                        .min_size(egui::vec2(ui.available_width(), 22.0)),
                )
                .on_hover_text(format!("{}\n{}ms", run.url(), run.duration_ms()));

            if response.clicked() {
                action = Some(RunAction::Select(run.id()));
            }
            if response.double_clicked() {
                details.open = Some(run.id());
            }
            response.context_menu(|ui| {
                if ui.button("View Response").clicked() {
                    action = Some(RunAction::Select(run.id()));
                    ui.close();
                }
                if ui.button("Open details").clicked() {
                    details.open = Some(run.id());
                    ui.close();
                }
                if can_load && ui.button("Load into Editor").clicked() {
                    action = Some(RunAction::Load(run.id()));
                    ui.close();
                }
                ui.menu_button("Copy", |ui| {
                    if let Some(action) = copy_menu::items(ui, true) {
                        ui.ctx().copy_text(run.exchange().copy_text(&action));
                    }
                });
            });
        }
    });
    action
}

fn details_window<R: Run>(ctx: &egui::Context, id_salt: &str, runs: &[R], can_load: bool, details: &mut RunDetails) -> Option<RunAction> {
    let index = runs.iter().position(|r| Some(r.id()) == details.open)?;
    let run = &runs[index];
    let mut open = true;
    let mut action = None;
    // Runs are newest first, so older is further down the list.
    let mut step = None;
    egui::Window::new(format!("{} {}", run.method(), url_path(run.url())))
        .id(egui::Id::new((id_salt, "run_details")))
        .open(&mut open)
        .default_size([960.0, 620.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.add_enabled(index + 1 < runs.len(), egui::Button::new("Older")).clicked() {
                    step = Some(index + 1);
                }
                if ui.add_enabled(index > 0, egui::Button::new("Newer")).clicked() {
                    step = index.checked_sub(1);
                }
                ui.label(status_text(ui.visuals(), run.status_code()));
                ui.label(RichText::new(format!("{} · {}ms", local_time(run.executed_at_ms()), run.duration_ms())).weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if can_load && ui.button("Load into editor").clicked() {
                        action = Some(RunAction::Load(run.id()));
                    }
                });
            });
            ui.add_space(4.0);
            let request_headers = pairs(run.request_headers());
            let response_headers = pairs(run.response_headers());
            let request = PaneData {
                cache_key: run.id() as u64,
                title: "Request",
                headers: &request_headers,
                body: run.request_body(),
                content_type: content_type(&request_headers),
                params: None,
                cookies: None,
                form_parser: Some(captured_form),
            };
            let response = PaneData {
                cache_key: run.id() as u64,
                title: "Response",
                headers: &response_headers,
                body: run.response_body(),
                content_type: content_type(&response_headers),
                params: None,
                cookies: None,
                form_parser: Some(captured_form),
            };
            let [mut request_ui, mut response_ui] = split::side_by_side(ui, (id_salt, "run_details_split"), &mut details.split);
            message_pane::show(&mut request_ui, &format!("{id_salt}_details_request"), &mut details.request, &request);
            message_pane::show(&mut response_ui, &format!("{id_salt}_details_response"), &mut details.response, &response);
        });
    let typing = ctx.egui_wants_keyboard_input();
    ctx.input(|i| {
        if !typing && i.key_pressed(egui::Key::ArrowLeft) && index + 1 < runs.len() {
            step = Some(index + 1);
        }
        if !typing && i.key_pressed(egui::Key::ArrowRight) && index > 0 {
            step = Some(index - 1);
        }
        if i.key_pressed(egui::Key::Escape) {
            open = false;
        }
    });
    if let Some(next) = step {
        details.open = Some(runs[next].id());
    }
    if !open {
        details.open = None;
    }
    action
}

fn pairs(headers: &[HeaderItem]) -> Vec<(String, String)> {
    headers.iter().map(|h| (h.key.clone(), h.value.clone())).collect()
}

fn content_type(headers: &[(String, String)]) -> &str {
    headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("content-type")).map_or("", |(_, v)| v.as_str())
}
