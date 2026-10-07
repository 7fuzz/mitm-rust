use eframe::egui::{self, RichText};
use mitm_core::collections::RequestHistoryItem;
use mitm_core::repeater::{HeaderItem, RepeaterHistoryItem};
use crate::format::{CopyFormat, Exchange};
use crate::widgets::pills::render_status_pill;

/// A recorded send of a Repeater or collection request.
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

pub fn show<R: Run>(ui: &mut egui::Ui, id_salt: &str, runs: &[R], selected: Option<i64>) -> Option<RunAction> {
    if runs.is_empty() {
        ui.add_space(16.0);
        ui.vertical_centered(|ui| ui.label(RichText::new("No runs recorded").weak().italics()));
        return None;
    }

    let mut action = None;
    egui::ScrollArea::vertical().id_salt(id_salt).auto_shrink([false, false]).show(ui, |ui| {
        for run in runs {
            let response = ui
                .push_id(run.id(), |ui| {
                    let frame = egui::Frame::NONE.inner_margin(egui::Margin::symmetric(6, 4));
                    let frame = if selected == Some(run.id()) {
                        frame.fill(ui.visuals().selection.bg_fill.gamma_multiply(0.35))
                    } else {
                        frame
                    };
                    frame
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                render_status_pill(ui, run.status_code());
                                ui.label(RichText::new(format!("{}ms", run.duration_ms())).weak().size(10.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(RichText::new(local_time(run.executed_at_ms())).weak().size(10.0));
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(run.method()).weak().strong().size(10.0));
                                ui.add(egui::Label::new(RichText::new(url_path(run.url())).monospace().size(11.0)).truncate());
                            });
                        })
                        .response
                        .interact(egui::Sense::click())
                })
                .inner;

            if response.clicked() {
                action = Some(RunAction::Select(run.id()));
            }
            response.context_menu(|ui| {
                if ui.button("View Response").clicked() {
                    action = Some(RunAction::Select(run.id()));
                    ui.close();
                }
                if ui.button("Load into Editor").clicked() {
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
            ui.separator();
        }
    });
    action
}
