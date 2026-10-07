use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::fuzzer::FuzzTemplate;
use mitm_core::repeater::HeaderItem;
use crate::state::{FuzzerActionCmd, UiState};
use crate::views::pills::{format_bytes, format_duration, render_method_pill, render_status_pill};

pub fn render_fuzzer_view(ui: &mut egui::Ui, state: &mut UiState) {
    let fuzzer = &mut state.fuzzer_state;

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

        ui.add(
            egui::TextEdit::singleline(&mut fuzzer.url)
                .hint_text("Target URL with placeholder e.g. https://target.com/api/{{id}}")
                .desired_width(ui.available_width() - 260.0),
        );

        if fuzzer.is_running {
            if ui.button(RichText::new("⏹ Stop").color(Color32::from_rgb(239, 68, 68)).strong()).clicked() {
                state.fuzzer_action_queue.push(FuzzerActionCmd::Stop);
                fuzzer.is_running = false;
            }
            ui.spinner();
            ui.label(format!("{}/{}", fuzzer.progress_completed, fuzzer.progress_total));
        } else {
            let start_btn = RichText::new("▶ Start Fuzzing")
                .color(Color32::from_rgb(34, 197, 94))
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

                    let headers: Vec<HeaderItem> = fuzzer
                        .headers
                        .lines()
                        .filter_map(|line| {
                            let mut parts = line.splitn(2, ':');
                            let key = parts.next()?.trim().to_string();
                            let value = parts.next()?.trim().to_string();
                            if !key.is_empty() {
                                Some(HeaderItem {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    key,
                                    value,
                                    enabled: true,
                                })
                            } else {
                                None
                            }
                        })
                        .collect();

                    let template = FuzzTemplate {
                        method: fuzzer.method.clone(),
                        url: fuzzer.url.clone(),
                        headers,
                        params: Vec::new(),
                        body_type: "raw".to_string(),
                        body: if fuzzer.body.is_empty() { None } else { Some(fuzzer.body.clone()) },
                    };

                    state.fuzzer_action_queue.push(FuzzerActionCmd::Start {
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
            ui.add(
                egui::TextEdit::multiline(&mut fuzzer.headers)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(6.0);
            ui.strong("Body Template:");
            ui.add(
                egui::TextEdit::multiline(&mut fuzzer.body)
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

                        row.col(|ui| {
                            render_status_pill(ui, r.status_code);
                        });

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
