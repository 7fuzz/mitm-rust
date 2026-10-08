use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::repeater::HeaderItem;
use crate::command::{AppCommand, Commands};
use crate::features::request_editor::body::{content_type_for, convert, REPEATER_BODY_TYPES};
use crate::features::request_editor::body_editor;
use crate::features::request_editor::kv_table::{self, standard_headers, KvLabels};
use crate::features::request_editor::method::method_combo;
use crate::features::request_editor::url_params::{build_url, new_id, parse_query};
use crate::format::{format_bytes, format_duration};
use crate::theme::{self, Tone};
use crate::widgets::pills::render_status_pill;
use crate::widgets::{split, template};
use super::{ConfigTab, FuzzerState, RequestTab, RunStatus};

pub fn show(ui: &mut egui::Ui, fuzzer: &mut FuzzerState, cmds: &mut Commands) {
    toolbar(ui, fuzzer, cmds);
    ui.separator();
    let [mut top, mut bottom] = split::stacked(ui, "fuzzer_split", &mut fuzzer.prefs.split);
    config(&mut top, fuzzer);
    results(&mut bottom, fuzzer);
    // Any request not taken by a focused field this frame is dropped so it never fires later.
    template::clear_mark_request();
}

fn toolbar(ui: &mut egui::Ui, fuzzer: &mut FuzzerState, cmds: &mut Commands) {
    ui.horizontal(|ui| {
        ui.set_min_height(28.0);
        if fuzzer.status == RunStatus::Running {
            if ui.button(RichText::new("⏹ Stop").color(theme::color(ui, Tone::Danger)).strong()).clicked() {
                cmds.push(AppCommand::StopFuzzer);
            }
            ui.spinner();
        } else {
            let start = RichText::new("▶ Start").color(theme::color(ui, Tone::Success)).strong();
            if ui.button(start).clicked() {
                cmds.push(AppCommand::StartFuzzer(fuzzer.config()));
            }
        }
        if fuzzer.status != RunStatus::Idle {
            ui.label(format!("{}/{}", fuzzer.results.len(), fuzzer.total));
        }
        if fuzzer.status == RunStatus::Stopped {
            ui.label(RichText::new("Stopped").color(theme::color(ui, Tone::Warn)));
        }
        if let Some(error) = &fuzzer.error {
            ui.label(RichText::new(error).color(theme::color(ui, Tone::Danger)));
        }

        let finished = matches!(fuzzer.status, RunStatus::Stopped | RunStatus::Done);
        if finished && ui.button("🗑 Clear").clicked() {
            fuzzer.clear();
        }
    });
}

fn config(ui: &mut egui::Ui, fuzzer: &mut FuzzerState) {
    ui.horizontal(|ui| {
        for (value, label) in [(ConfigTab::Request, "Request"), (ConfigTab::Payloads, "Payloads"), (ConfigTab::Options, "Options")] {
            ui.selectable_value(&mut fuzzer.config_tab, value, label);
        }
    });
    ui.separator();
    match fuzzer.config_tab {
        ConfigTab::Request => request_tab(ui, fuzzer),
        ConfigTab::Payloads => {
            egui::ScrollArea::vertical().id_salt("fuzzer_payloads").auto_shrink([false, false]).show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut fuzzer.wordlist_input)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(10)
                        .desired_width(f32::INFINITY),
                );
            });
        }
        ConfigTab::Options => {
            ui.horizontal(|ui| {
                ui.label("Concurrency");
                ui.add(egui::DragValue::new(&mut fuzzer.concurrency).range(1..=50));
            });
        }
    }
}

fn request_tab(ui: &mut egui::Ui, fuzzer: &mut FuzzerState) {
    ui.horizontal(|ui| {
        method_combo(ui, "fuzzer_method", &mut fuzzer.method);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Clear marks").on_hover_text("Strip {{ }} placeholders").clicked() {
                fuzzer.clear_marks();
            }
            if ui.button("Mark").on_hover_text("Wrap selection in {{ }}  (Ctrl+M)").clicked() {
                template::request_mark();
            }
        });
    });
    ui.horizontal(|ui| {
        let mut layouter = template::layouter(egui::TextStyle::Monospace, None);
        let url = template::show(ui, &mut fuzzer.url, |text| {
            egui::TextEdit::singleline(text)
                .hint_text("https://target.com/api/{{id}}")
                .layouter(&mut layouter)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
        });
        if url.changed() {
            fuzzer.params = parse_query(&fuzzer.url, &fuzzer.params);
        }
    });
    ui.separator();

    ui.horizontal(|ui| {
        let params = fuzzer.params.iter().filter(|p| p.enabled && !p.key.trim().is_empty()).count();
        let headers = fuzzer.headers.iter().filter(|h| h.enabled && !h.key.trim().is_empty()).count();
        ui.selectable_value(&mut fuzzer.request_tab, RequestTab::Params, format!("Params ({})", params));
        ui.selectable_value(&mut fuzzer.request_tab, RequestTab::Headers, format!("Headers ({})", headers));
        ui.selectable_value(&mut fuzzer.request_tab, RequestTab::Body, "Body");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if fuzzer.request_tab == RequestTab::Body {
                body_type_combo(ui, fuzzer);
            }
        });
    });
    ui.separator();

    match fuzzer.request_tab {
        RequestTab::Body => {
            body_editor::show(ui, "fuzzer_body", &fuzzer.body_type, &mut fuzzer.body_content);
        }
        other => {
            egui::ScrollArea::vertical().id_salt("fuzzer_editor").auto_shrink([false, false]).show(ui, |ui| match other {
                RequestTab::Params => {
                    let labels = KvLabels { key: "Parameter Key", value: "Parameter Value" };
                    if kv_table::show(ui, "fuzzer_params", &mut fuzzer.params, labels, &[]) {
                        fuzzer.url = build_url(&fuzzer.url, &fuzzer.params);
                    }
                }
                RequestTab::Headers => {
                    let labels = KvLabels { key: "Header Name", value: "Header Value" };
                    let suggestions = standard_headers(content_type_for(&fuzzer.body_type).unwrap_or("application/json"));
                    kv_table::show(ui, "fuzzer_headers", &mut fuzzer.headers, labels, &suggestions);
                }
                RequestTab::Body => {}
            });
        }
    }
}

fn body_type_combo(ui: &mut egui::Ui, fuzzer: &mut FuzzerState) {
    let current = REPEATER_BODY_TYPES
        .iter()
        .find(|(id, _)| *id == fuzzer.body_type)
        .map_or("Raw", |(_, label)| *label);
    let mut picked = None;
    egui::ComboBox::from_id_salt("fuzzer_body_type").selected_text(current).show_ui(ui, |ui| {
        for (id, label) in REPEATER_BODY_TYPES {
            if ui.selectable_label(fuzzer.body_type == id, label).clicked() && fuzzer.body_type != id {
                picked = Some(id);
            }
        }
    });
    let Some(next) = picked else { return };
    set_content_type_header(&mut fuzzer.headers, content_type_for(next));
    fuzzer.body_content = convert(&fuzzer.body_type, next, &fuzzer.body_content);
    fuzzer.body_type = next.to_string();
}

fn set_content_type_header(headers: &mut Vec<HeaderItem>, content_type: Option<&str>) {
    let existing = headers.iter_mut().find(|h| h.key.trim().eq_ignore_ascii_case("content-type"));
    match (existing, content_type) {
        (Some(h), None) => h.enabled = false,
        (Some(h), Some(ct)) => {
            h.value = ct.to_string();
            h.enabled = true;
        }
        (None, Some(ct)) => headers.push(HeaderItem {
            id: new_id(),
            key: "Content-Type".to_string(),
            value: ct.to_string(),
            enabled: true,
        }),
        (None, None) => {}
    }
}

fn results(ui: &mut egui::Ui, fuzzer: &FuzzerState) {
    let row_height = egui::TextStyle::Body.resolve(ui.style()).size + 6.0;
    TableBuilder::new(ui)
        .id_salt("fuzzer_results")
        .striped(true)
        .resizable(true)
        .auto_shrink([false, false])
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(55.0))
        .column(Column::remainder())
        .column(Column::exact(55.0))
        .column(Column::exact(70.0))
        .column(Column::exact(65.0))
        .header(22.0, |mut header| {
            for title in ["#", "Payload", "Status", "Size", "Time"] {
                header.col(|ui| {
                    ui.strong(title);
                });
            }
        })
        .body(|body| {
            body.rows(row_height, fuzzer.results.len(), |mut row| {
                let r = &fuzzer.results[row.index()];
                row.col(|ui| {
                    ui.label((r.idx + 1).to_string());
                });
                row.col(|ui| {
                    ui.add(egui::Label::new(RichText::new(r.payloads.join(", ")).monospace()).truncate());
                });
                let (_, status_cell) = row.col(|ui| render_status_pill(ui, r.status_code));
                if let Some(err) = &r.error {
                    status_cell.on_hover_text(err);
                }
                row.col(|ui| {
                    ui.label(format_bytes(r.response_size));
                });
                row.col(|ui| {
                    ui.label(format_duration(Some(r.duration_ms)));
                });
            });
        });
}
