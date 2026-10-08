use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use crate::command::{AppCommand, Commands};
use crate::features::request_editor::method::method_combo;
use crate::format::{format_bytes, format_duration};
use crate::theme::{self, Tone};
use crate::widgets::pills::render_status_pill;
use crate::widgets::{split, template};
use super::{ConfigTab, FuzzerState, RunStatus};

pub fn show(ui: &mut egui::Ui, fuzzer: &mut FuzzerState, cmds: &mut Commands) {
    toolbar(ui, fuzzer, cmds);
    ui.separator();
    let [mut top, mut bottom] = split::stacked(ui, "fuzzer_split", &mut fuzzer.prefs.split);
    config(&mut top, fuzzer);
    results(&mut bottom, fuzzer);
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
    egui::ScrollArea::vertical().id_salt("fuzzer_config").auto_shrink([false, false]).show(ui, |ui| match fuzzer.config_tab {
        ConfigTab::Request => request_tab(ui, fuzzer),
        ConfigTab::Payloads => {
            ui.add(
                egui::TextEdit::multiline(&mut fuzzer.wordlist_input)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(10)
                    .desired_width(f32::INFINITY),
            );
        }
        ConfigTab::Options => {
            ui.horizontal(|ui| {
                ui.label("Concurrency");
                ui.add(egui::DragValue::new(&mut fuzzer.concurrency).range(1..=50));
            });
        }
    });
}

fn request_tab(ui: &mut egui::Ui, fuzzer: &mut FuzzerState) {
    ui.horizontal(|ui| {
        method_combo(ui, "fuzzer_method", &mut fuzzer.method);
        let mut layouter = template::layouter(egui::TextStyle::Monospace, None);
        template::show(ui, &mut fuzzer.url, |text| {
            egui::TextEdit::singleline(text)
                .hint_text("https://target.com/api/{{id}}")
                .layouter(&mut layouter)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
        });
    });
    ui.add_space(6.0);
    ui.strong("Headers");
    let mut layouter = template::layouter(egui::TextStyle::Monospace, None);
    template::show(ui, &mut fuzzer.headers, |text| {
        egui::TextEdit::multiline(text)
            .layouter(&mut layouter)
            .font(egui::TextStyle::Monospace)
            .desired_rows(4)
            .desired_width(f32::INFINITY)
    });
    ui.add_space(6.0);
    ui.strong("Body");
    let mut layouter = template::layouter(egui::TextStyle::Monospace, None);
    template::show(ui, &mut fuzzer.body, |text| {
        egui::TextEdit::multiline(text)
            .layouter(&mut layouter)
            .font(egui::TextStyle::Monospace)
            .desired_rows(6)
            .desired_width(f32::INFINITY)
    });
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
