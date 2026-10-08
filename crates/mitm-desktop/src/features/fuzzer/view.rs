use eframe::egui::{self, RichText};
use crate::theme::{self, Tone};
use egui_extras::{Column, TableBuilder};
use crate::command::{AppCommand, Commands};
use crate::format::{format_bytes, format_duration};
use crate::widgets::pills::{render_method_pill, render_status_pill};
use crate::widgets::template;
use super::{FuzzerState, RunStatus};

pub fn show(ui: &mut egui::Ui, fuzzer: &mut FuzzerState, cmds: &mut Commands) {

    ui.horizontal(|ui| {
        ui.heading("HTTP Fuzzer");
        ui.separator();

        render_method_pill(ui, &fuzzer.method);

        egui::ComboBox::from_id_salt("fuzzer_method_combo")
            .selected_text(&fuzzer.method)
            .show_ui(ui, |ui| {
                for m in &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
                    ui.selectable_value(&mut fuzzer.method, m.to_string(), *m);
                }
            });

        let mut layouter = template::layouter(egui::TextStyle::Body, None);
        let width = ui.available_width() - 260.0;
        template::show(ui, &mut fuzzer.url, |text| {
            egui::TextEdit::singleline(text)
                .hint_text("Target URL with placeholder e.g. https://target.com/api/{{id}}")
                .layouter(&mut layouter)
                .desired_width(width)
        });

        if fuzzer.status == RunStatus::Running {
            if ui.button(RichText::new("⏹ Stop").color(theme::color(ui, Tone::Danger)).strong()).clicked() {
                cmds.push(AppCommand::StopFuzzer);
            }
            ui.spinner();
        } else {
            let start_btn = RichText::new("▶ Start Fuzzing")
                .color(theme::color(ui, Tone::Success))
                .strong();
            if ui.button(start_btn).clicked() {
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

    ui.separator();

    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            ui.heading("Wordlist & Markers");
            ui.label(RichText::new("Use {{variable}} anywhere in the URL, headers, or body.").weak());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.strong("Concurrency:");
                ui.add(egui::DragValue::new(&mut fuzzer.concurrency).range(1..=50));
            });

            ui.add_space(6.0);
            ui.strong("Payload Wordlist (one per line):");
            ui.add(
                egui::TextEdit::multiline(&mut fuzzer.wordlist_input)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(8)
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(6.0);
            ui.strong("Headers Template:");
            let mut layouter = template::layouter(egui::TextStyle::Monospace, None);
            template::show(ui, &mut fuzzer.headers, |text| {
                egui::TextEdit::multiline(text)
                    .layouter(&mut layouter)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY)
            });

            ui.add_space(6.0);
            ui.strong("Body Template:");
            let mut layouter = template::layouter(egui::TextStyle::Monospace, None);
            template::show(ui, &mut fuzzer.body, |text| {
                egui::TextEdit::multiline(text)
                    .layouter(&mut layouter)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(6)
                    .desired_width(f32::INFINITY)
            });
        });

        columns[1].vertical(|ui| {
            ui.heading(format!("Results ({})", fuzzer.results.len()));
            ui.add_space(4.0);

            let num_rows = fuzzer.results.len();
            let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
            let row_height = text_height + 6.0;

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(45.0))
                .column(Column::remainder())
                .column(Column::exact(55.0))
                .column(Column::exact(65.0))
                .column(Column::exact(60.0))
                .header(22.0, |mut header| {
                    header.col(|ui| { ui.strong("#"); });
                    header.col(|ui| { ui.strong("Payload"); });
                    header.col(|ui| { ui.strong("Status"); });
                    header.col(|ui| { ui.strong("Size"); });
                    header.col(|ui| { ui.strong("Time"); });
                })
                .body(|body| {
                    body.rows(row_height, num_rows, |mut row| {
                        let idx = row.index();
                        let r = &fuzzer.results[idx];

                        row.col(|ui| {
                            ui.label((r.idx + 1).to_string());
                        });

                        row.col(|ui| {
                            ui.label(RichText::new(r.payloads.join(", ")).strong());
                        });

                        let (_, status_cell) = row.col(|ui| {
                            render_status_pill(ui, r.status_code);
                        });
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
        });
    });
}
