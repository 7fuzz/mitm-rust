use eframe::egui::{self, RichText};
use crate::theme::{self, Tone};
use egui_extras::{Column, TableBuilder};
use mitm_core::fuzzer::FuzzTemplate;
use crate::command::{AppCommand, Commands};
use crate::format::{format_bytes, format_duration, parse_header_items};
use crate::widgets::pills::{render_method_pill, render_status_pill};
use crate::widgets::template;
use super::FuzzerState;

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
        ui.add(
            egui::TextEdit::singleline(&mut fuzzer.url)
                .hint_text("Target URL with placeholder e.g. https://target.com/api/{{id}}")
                .layouter(&mut layouter)
                .desired_width(ui.available_width() - 260.0),
        );

        if fuzzer.is_running {
            if ui.button(RichText::new("⏹ Stop").color(theme::color(ui, Tone::Danger)).strong()).clicked() {
                fuzzer.is_running = false;
            }
            ui.spinner();
            ui.label(format!("{}/{}", fuzzer.progress_completed, fuzzer.progress_total));
        } else {
            let start_btn = RichText::new("▶ Start Fuzzing")
                .color(theme::color(ui, Tone::Success))
                .strong();
            if ui.button(start_btn).clicked() {
                let payloads: Vec<String> = fuzzer
                    .wordlist_input
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();

                if !payloads.is_empty() {
                    fuzzer.is_running = true;
                    fuzzer.progress_completed = 0;
                    fuzzer.progress_total = payloads.len();
                    fuzzer.results.clear();

                    let template = FuzzTemplate {
                        method: fuzzer.method.clone(),
                        url: fuzzer.url.clone(),
                        headers: parse_header_items(&fuzzer.headers),
                        params: Vec::new(),
                        body_type: "raw".to_string(),
                        body: if fuzzer.body.is_empty() { None } else { Some(fuzzer.body.clone()) },
                    };

                    cmds.push(AppCommand::StartFuzzer {
                        template,
                        payloads,
                        concurrency: fuzzer.concurrency,
                    });
                }
            }
        }

        if !fuzzer.results.is_empty() && ui.button("🗑 Clear").clicked() {
            fuzzer.results.clear();
            fuzzer.progress_completed = 0;
            fuzzer.progress_total = 0;
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
            ui.add(
                egui::TextEdit::multiline(&mut fuzzer.headers)
                    .layouter(&mut layouter)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(6.0);
            ui.strong("Body Template:");
            let mut layouter = template::layouter(egui::TextStyle::Monospace, None);
            ui.add(
                egui::TextEdit::multiline(&mut fuzzer.body)
                    .layouter(&mut layouter)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(6)
                    .desired_width(f32::INFINITY),
            );
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
                            ui.label(r.index.to_string());
                        });

                        row.col(|ui| {
                            ui.label(RichText::new(&r.payload).strong());
                        });

                        let (_, status_cell) = row.col(|ui| {
                            render_status_pill(ui, r.status_code);
                        });
                        if let Some(err) = &r.error {
                            status_cell.on_hover_text(err);
                        }

                        row.col(|ui| {
                            ui.label(format_bytes(r.size));
                        });

                        row.col(|ui| {
                            ui.label(format_duration(Some(r.duration_ms)));
                        });
                    });
                });
        });
    });
}
