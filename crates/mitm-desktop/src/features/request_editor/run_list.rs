use eframe::egui::{self, RichText};
use mitm_core::collections::RequestHistoryItem;
use mitm_core::repeater::{HeaderItem, RepeaterHistoryItem};
use crate::format::{CopyFormat, Exchange};
use crate::widgets::pills::status_text;

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

pub fn show<R: Run>(ui: &mut egui::Ui, id_salt: &str, runs: &[R], selected: Option<i64>, can_load: bool) -> Option<RunAction> {
    if runs.is_empty() {
        ui.weak("No runs yet");
        return None;
    }

    let mut action = None;
    egui::ScrollArea::vertical().id_salt(id_salt).auto_shrink([false, false]).show(ui, |ui| {
        for run in runs {
            let style = ui.style().clone();
            let mut text = egui::text::LayoutJob::default();
            status_text(run.status_code()).append_to(&mut text, &style, egui::FontSelection::Default, egui::Align::Center);
            RichText::new(format!("  {} {}", run.method(), url_path(run.url())))
                .monospace()
                .append_to(&mut text, &style, egui::FontSelection::Default, egui::Align::Center);

            let response = ui
                .add(
                    egui::Button::selectable(selected == Some(run.id()), text)
                        .right_text(RichText::new(local_time(run.executed_at_ms())).weak().size(11.0))
                        .truncate()
                        .min_size(egui::vec2(ui.available_width(), 22.0)),
                )
                .on_hover_text(format!("{}\n{}ms", run.url(), run.duration_ms()));

            if response.clicked() {
                action = Some(RunAction::Select(run.id()));
            }
            response.context_menu(|ui| {
                if ui.button("View Response").clicked() {
                    action = Some(RunAction::Select(run.id()));
                    ui.close();
                }
                if can_load && ui.button("Load into Editor").clicked() {
                    action = Some(RunAction::Load(run.id()));
                    ui.close();
                }
                ui.menu_button("Copy", |ui| {
                    for (label, format) in [
                        ("Copy cURL", CopyFormat::Curl),
                        ("Copy cURL and Response", CopyFormat::CurlAndResponse),
                        ("Copy URL, Body, and Response", CopyFormat::UrlBodyAndResponse),
                    ] {
                        if ui.button(label).clicked() {
                            ui.ctx().copy_text(run.exchange().copy_text(format));
                            ui.close();
                        }
                    }
                });
            });
        }
    });
    action
}
