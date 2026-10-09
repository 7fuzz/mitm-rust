use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::repeater::HeaderItem;
use crate::app::command::{AppCommand, Commands};
use crate::features::request_editor::body::{content_type_for, convert, REPEATER_BODY_TYPES};
use crate::features::request_editor::body_editor;
use crate::features::request_editor::kv_table::{self, standard_headers, KvLabels};
use crate::features::request_editor::method::method_combo;
use crate::features::request_editor::url_params::{build_url, new_id, parse_query};
use crate::util::format::{format_bytes, format_duration};
use crate::ui::theme::{self, Tone};
use crate::widgets::pills::render_status_pill;
use crate::widgets::tri_state::tri_state_button;
use crate::widgets::{split, template};
use super::{AttackType, ConfigTab, FuzzSearchScope, FuzzSortColumn, FuzzerState, MatchRule, PayloadConfig, PayloadKind, RequestTab, RunStatus};

pub fn show(ui: &mut egui::Ui, fuzzer: &mut FuzzerState, cmds: &mut Commands) {
    toolbar(ui, fuzzer, cmds);
    ui.separator();
    let [mut top, mut bottom] = split::stacked(ui, "fuzzer_split", &mut fuzzer.prefs.split);
    config(&mut top, fuzzer);
    results(&mut bottom, fuzzer, cmds);
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
        runs_popover(ui, fuzzer, cmds);
        if fuzzer.status != RunStatus::Idle {
            ui.label(format!("{}/{}", fuzzer.results.len(), fuzzer.total));
        } else if !fuzzer.variable_names().is_empty() {
            let estimate = fuzzer.request_estimate();
            let label = if estimate == 1 {
                "1 request".to_string()
            } else {
                format!("{} requests", estimate)
            };
            if estimate > mitm_core::fuzzer::MAX_REQUESTS {
                ui.label(RichText::new(label).color(theme::color(ui, Tone::Danger)))
                    .on_hover_text(format!("Limit is {} requests", mitm_core::fuzzer::MAX_REQUESTS));
            } else {
                ui.weak(label);
            }
        }

        if let Some(run_id) = fuzzer.run_id {
            if !fuzzer.is_saved {
                ui.label(RichText::new("●").color(theme::color(ui, Tone::Warn)))
                    .on_hover_text("Unsaved");
                if fuzzer.saving_run {
                    let text_edit = ui.add(
                        egui::TextEdit::singleline(&mut fuzzer.save_name_input)
                            .hint_text("Run name")
                            .desired_width(120.0),
                    );
                    if fuzzer.focus_save_input {
                        text_edit.request_focus();
                        fuzzer.focus_save_input = false;
                    }
                    let enter = text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if enter || ui.button("Save").clicked() {
                        let name = if fuzzer.save_name_input.trim().is_empty() {
                            format!("Run #{}", run_id)
                        } else {
                            fuzzer.save_name_input.trim().to_string()
                        };
                        cmds.push(AppCommand::SaveFuzzRun { run_id, name });
                        fuzzer.saving_run = false;
                    }
                    if ui.button("✕").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        fuzzer.saving_run = false;
                    }
                } else if ui.button("Save run…").clicked() {
                    fuzzer.saving_run = true;
                    fuzzer.focus_save_input = true;
                    fuzzer.save_name_input = fuzzer
                        .run_name
                        .clone()
                        .unwrap_or_else(|| format!("Run #{}", run_id));
                }
            } else if let Some(ref name) = fuzzer.run_name {
                ui.label(RichText::new(name).strong());
            }
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

fn runs_popover(ui: &mut egui::Ui, fuzzer: &mut FuzzerState, cmds: &mut Commands) {
    let menu = ui.menu_button("🕘", |ui| {
        ui.set_min_width(300.0);
        if fuzzer.saved_runs.is_empty() {
            ui.weak("No saved runs");
            return;
        }
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            let mut to_load = None;
            let mut to_delete = None;
            for run in &fuzzer.saved_runs {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.strong(&run.name);
                        let date = chrono::DateTime::from_timestamp_millis(run.created_at_ms)
                            .map(|at| at.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_default();
                        ui.weak(format!("{} · {} reqs · {}", run.attack_type, run.total, date));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("🗑").on_hover_text("Delete run").clicked() {
                            to_delete = Some(run.id);
                        }
                        if ui.button("Open").clicked() {
                            to_load = Some(run.id);
                            ui.close();
                        }
                    });
                });
                ui.separator();
            }
            if let Some(id) = to_load {
                cmds.push(AppCommand::LoadFuzzRun(id));
            }
            if let Some(id) = to_delete {
                cmds.push(AppCommand::DeleteFuzzRun(id));
            }
        });
    });
    if menu.response.clicked() {
        cmds.push(AppCommand::ReloadSavedFuzzRuns);
    }
    menu.response.on_hover_text("Saved runs");
}

fn config(ui: &mut egui::Ui, fuzzer: &mut FuzzerState) {
    ui.horizontal(|ui| {
        for (value, label) in [
            (ConfigTab::Request, "Request"),
            (ConfigTab::Payloads, "Payloads"),
            (ConfigTab::Match, "Match"),
            (ConfigTab::Options, "Options"),
        ] {
            ui.selectable_value(&mut fuzzer.config_tab, value, label);
        }
    });
    ui.separator();
    match fuzzer.config_tab {
        ConfigTab::Request => request_tab(ui, fuzzer),
        ConfigTab::Payloads => payloads_tab(ui, fuzzer),
        ConfigTab::Match => match_tab(ui, fuzzer),
        ConfigTab::Options => options_tab(ui, fuzzer),
    }
}

fn match_tab(ui: &mut egui::Ui, fuzzer: &mut FuzzerState) {
    ui.horizontal(|ui| {
        if ui.button("➕ Add rule").clicked() {
            let next_idx = fuzzer.match_rules.len() + 1;
            fuzzer.match_rules.push(MatchRule {
                name: format!("match{}", next_idx),
                kind: "contains".to_string(),
                pattern: String::new(),
            });
        }
    });
    ui.separator();

    if fuzzer.match_rules.is_empty() {
        ui.weak("No match rules");
        return;
    }

    let mut remove_idx = None;
    egui::ScrollArea::vertical().id_salt("fuzzer_match_scroll").auto_shrink([false, false]).show(ui, |ui| {
        TableBuilder::new(ui)
            .id_salt("fuzzer_match_table")
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(130.0).at_least(80.0))
            .column(Column::exact(130.0))
            .column(Column::remainder())
            .column(Column::exact(28.0))
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Name");
                });
                header.col(|ui| {
                    ui.strong("Type");
                });
                header.col(|ui| {
                    ui.strong("Pattern");
                });
                header.col(|_ui| {});
            })
            .body(|body| {
                let row_height = 24.0;
                body.rows(row_height, fuzzer.match_rules.len(), |mut row| {
                    let idx = row.index();
                    let rule = &mut fuzzer.match_rules[idx];
                    row.col(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut rule.name).desired_width(f32::INFINITY));
                    });
                    row.col(|ui| {
                        ui.selectable_value(&mut rule.kind, "contains".to_string(), "Count");
                        ui.selectable_value(&mut rule.kind, "regex".to_string(), "Regex");
                    });
                    row.col(|ui| {
                        let hint = if rule.kind == "regex" { "\"token\":\"(\\w+)\"" } else { "string" };
                        ui.add(egui::TextEdit::singleline(&mut rule.pattern).hint_text(hint).desired_width(f32::INFINITY));
                    });
                    row.col(|ui| {
                        if ui.small_button("🗑").on_hover_text("Remove rule").clicked() {
                            remove_idx = Some(idx);
                        }
                    });
                });
            });
    });

    if let Some(idx) = remove_idx {
        fuzzer.match_rules.remove(idx);
    }
}

fn options_tab(ui: &mut egui::Ui, fuzzer: &mut FuzzerState) {
    ui.horizontal(|ui| {
        ui.label("Concurrency");
        ui.add(egui::DragValue::new(&mut fuzzer.concurrency).range(1..=200));
        ui.add_space(16.0);
        ui.label("Delay");
        ui.add(egui::DragValue::new(&mut fuzzer.delay_ms).range(0..=60_000).suffix(" ms"));
    });
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
        let mut layouter = template::singleline_layouter(egui::TextStyle::Monospace, None);
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

fn payloads_tab(ui: &mut egui::Ui, fuzzer: &mut FuzzerState) {
    ui.horizontal(|ui| {
        for attack in [AttackType::Sniper, AttackType::Pitchfork, AttackType::ClusterBomb] {
            ui.selectable_value(&mut fuzzer.attack_type, attack, attack.label())
                .on_hover_text(attack.tooltip());
        }
    });
    ui.separator();

    let names = fuzzer.variable_names();
    if names.is_empty() {
        ui.weak("No variables");
        return;
    }
    egui::ScrollArea::vertical().id_salt("fuzzer_payloads").auto_shrink([false, false]).show(ui, |ui| {
        for name in &names {
            let cfg = fuzzer.payloads.entry(name.clone()).or_default();
            ui.push_id(name, |ui| payload_set(ui, name, cfg));
            ui.add_space(4.0);
        }
    });
}

fn payload_set(ui: &mut egui::Ui, name: &str, cfg: &mut PayloadConfig) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.strong(RichText::new(format!("{{{{{name}}}}}")).monospace());
            ui.selectable_value(&mut cfg.kind, PayloadKind::List, "List");
            ui.selectable_value(&mut cfg.kind, PayloadKind::Numbers, "Numbers");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(format!("{} payloads", cfg.count()));
            });
        });
        match cfg.kind {
            PayloadKind::List => {
                ui.add(
                    egui::TextEdit::multiline(&mut cfg.list_text)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(5)
                        .desired_width(f32::INFINITY),
                );
            }
            PayloadKind::Numbers => {
                ui.horizontal(|ui| {
                    ui.label("From");
                    ui.add(egui::DragValue::new(&mut cfg.from));
                    ui.label("To");
                    ui.add(egui::DragValue::new(&mut cfg.to));
                    ui.label("Step");
                    ui.add(egui::DragValue::new(&mut cfg.step));
                    ui.label("Pad").on_hover_text("Zero-pad to this width");
                    ui.add(egui::DragValue::new(&mut cfg.pad).range(0..=12));
                });
            }
        }
        ui.horizontal(|ui| {
            ui.label("Prefix");
            ui.add(egui::TextEdit::singleline(&mut cfg.prefix).desired_width(100.0));
            ui.label("Suffix");
            ui.add(egui::TextEdit::singleline(&mut cfg.suffix).desired_width(100.0));
            ui.checkbox(&mut cfg.url_encode, "URL-encode");
        });
    });
}

fn results(ui: &mut egui::Ui, fuzzer: &mut FuzzerState, cmds: &mut Commands) {
    if fuzzer.status == RunStatus::Idle && fuzzer.results.is_empty() {
        return;
    }
    if fuzzer.selected_idx.is_some() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        fuzzer.selected_idx = None;
        fuzzer.selected_result = None;
    }
    results_toolbar(ui, fuzzer);
    ui.separator();
    if fuzzer.selected_idx.is_some() {
        let [mut left, mut right] = split::side_by_side(ui, "fuzzer_results_split", &mut fuzzer.prefs.detail_split);
        results_table(&mut left, fuzzer, cmds);
        response_pane(&mut right, fuzzer, cmds);
    } else {
        results_table(ui, fuzzer, cmds);
    }
}

fn results_toolbar(ui: &mut egui::Ui, fuzzer: &mut FuzzerState) {
    ui.horizontal(|ui| {
        for group in ["2xx", "3xx", "4xx", "5xx", "ERR"] {
            let state = fuzzer.status_filters.entry(group.to_string()).or_default();
            tri_state_button(ui, group, state);
        }
        ui.separator();
        if ui
            .selectable_label(fuzzer.hide_common_size, "Hide most frequent length")
            .on_hover_text("Hide responses with the most frequent response size")
            .clicked()
        {
            fuzzer.hide_common_size = !fuzzer.hide_common_size;
        }
        ui.separator();

        crate::widgets::search::claim_focus(&ui.add(egui::TextEdit::singleline(&mut fuzzer.search_query).hint_text("Search…").desired_width(110.0)));
        ui.selectable_value(&mut fuzzer.search_scope, FuzzSearchScope::Payload, "Payload");
        ui.selectable_value(&mut fuzzer.search_scope, FuzzSearchScope::Response, "Response");
        if !fuzzer.search_query.is_empty() && ui.small_button("✕").on_hover_text("Clear search").clicked() {
            fuzzer.search_query.clear();
        }

        if fuzzer.has_active_filters() && ui.small_button("Reset").on_hover_text("Reset filters").clicked() {
            fuzzer.reset_filters();
        }

        let filtered_count = fuzzer.visible_indices().len();
        let total_count = fuzzer.results.len();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if fuzzer.has_active_filters() {
                ui.weak(format!("{filtered_count} of {total_count}"));
            } else if total_count > 0 {
                ui.weak(format!("{total_count}"));
            }
        });
    });
}

fn results_table(ui: &mut egui::Ui, fuzzer: &mut FuzzerState, cmds: &mut Commands) {
    let row_height = egui::TextStyle::Body.resolve(ui.style()).size + 6.0;
    let visible_indices = fuzzer.visible_indices().to_vec();

    let mut builder = TableBuilder::new(ui)
        .id_salt("fuzzer_results")
        .striped(true)
        .resizable(true)
        .sense(egui::Sense::click())
        .auto_shrink([false, false])
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(60.0))
        .column(Column::remainder().at_least(100.0))
        .column(Column::exact(65.0))
        .column(Column::exact(75.0))
        .column(Column::exact(70.0));

    for _ in &fuzzer.match_rules {
        builder = builder.column(Column::initial(80.0).at_least(50.0));
    }

    let mut next_sort = None;

    builder
        .header(22.0, |mut header| {
            header.col(|ui| {
                sort_header(ui, "#", FuzzSortColumn::Index, fuzzer.sort_column.as_ref(), fuzzer.sort_desc, &mut next_sort);
            });
            header.col(|ui| {
                sort_header(ui, "Payload", FuzzSortColumn::Payload, fuzzer.sort_column.as_ref(), fuzzer.sort_desc, &mut next_sort);
            });
            header.col(|ui| {
                sort_header(ui, "Status", FuzzSortColumn::Status, fuzzer.sort_column.as_ref(), fuzzer.sort_desc, &mut next_sort);
            });
            header.col(|ui| {
                sort_header(ui, "Size", FuzzSortColumn::Size, fuzzer.sort_column.as_ref(), fuzzer.sort_desc, &mut next_sort);
            });
            header.col(|ui| {
                sort_header(ui, "Time", FuzzSortColumn::Time, fuzzer.sort_column.as_ref(), fuzzer.sort_desc, &mut next_sort);
            });
            for rule in &fuzzer.match_rules {
                header.col(|ui| {
                    sort_header(
                        ui,
                        &rule.name,
                        FuzzSortColumn::Match(rule.name.clone()),
                        fuzzer.sort_column.as_ref(),
                        fuzzer.sort_desc,
                        &mut next_sort,
                    );
                });
            }
        })
        .body(|body| {
            body.rows(row_height, visible_indices.len(), |mut row| {
                let r = &fuzzer.results[visible_indices[row.index()]];
                let selected = fuzzer.selected_idx == Some(r.idx);
                row.set_selected(selected);
                let mut text_cells: Vec<egui::Response> = Vec::new();

                row.col(|ui| {
                    text_cells.push(ui.label((r.idx + 1).to_string()));
                });
                row.col(|ui| {
                    text_cells.push(ui.add(egui::Label::new(RichText::new(r.payloads.join(", ")).monospace()).truncate()));
                });
                let (_, mut status_cell) = row.col(|ui| render_status_pill(ui, r.status_code));
                if let Some(err) = &r.error {
                    status_cell = status_cell.on_hover_text(err);
                }
                text_cells.push(status_cell);
                row.col(|ui| {
                    text_cells.push(ui.label(format_bytes(r.response_size)));
                });
                row.col(|ui| {
                    text_cells.push(ui.label(format_duration(Some(r.duration_ms))));
                });
                for (i, _) in fuzzer.match_rules.iter().enumerate() {
                    row.col(|ui| {
                        let text = r.matches.get(i).map(String::as_str).unwrap_or("");
                        text_cells.push(ui.add(egui::Label::new(text).truncate()));
                    });
                }

                let response = text_cells.into_iter().fold(row.response(), |acc, resp| acc | resp);
                if (response.clicked() || response.secondary_clicked()) && fuzzer.selected_idx != Some(r.idx) {
                    if let Some(run_id) = fuzzer.run_id {
                        cmds.push(AppCommand::SelectFuzzResult { run_id, idx: r.idx });
                    }
                }
                response.context_menu(|ui| {
                    if let Some(run_id) = fuzzer.run_id {
                        if ui.button("🔁 Send to Repeater").clicked() {
                            cmds.push(AppCommand::SendFuzzResultToRepeater { run_id, idx: r.idx });
                            ui.close();
                        }
                    }
                    if ui.button("📋 Copy payload").clicked() {
                        let text = if r.payloads.len() == 1 {
                            r.payloads[0].clone()
                        } else {
                            r.payloads.join(", ")
                        };
                        ui.ctx().copy_text(text);
                        ui.close();
                    }
                    if let Some(run_id) = fuzzer.run_id {
                        if ui.button("📋 Copy cURL").clicked() {
                            cmds.push(AppCommand::CopyFuzzCurl { run_id, idx: r.idx });
                            ui.close();
                        }
                    }
                });
            });
        });

    if let Some(col) = next_sort {
        fuzzer.toggle_sort(col);
    }
}

fn response_pane(ui: &mut egui::Ui, fuzzer: &mut FuzzerState, cmds: &mut Commands) {
    let Some(result) = &fuzzer.selected_result else {
        ui.centered_and_justified(|ui| {
            ui.weak("Loading response…");
        });
        return;
    };

    let mut close = false;
    ui.horizontal(|ui| {
        render_status_pill(ui, result.status_code);
        ui.label(format_bytes(result.response_size));
        ui.label(format_duration(Some(result.duration_ms)));
        if let Some(err) = &result.error {
            ui.label(RichText::new(err).color(theme::color(ui, Tone::Danger)));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("✕").on_hover_text("Close response").clicked() {
                close = true;
            }
            ui.add(egui::Label::new(RichText::new(&result.url).monospace()).truncate());
        });
    });
    if close {
        fuzzer.selected_idx = None;
        fuzzer.selected_result = None;
        return;
    }
    ui.separator();

    let headers: Vec<(String, String)> = result
        .response_headers
        .iter()
        .map(|h| (h.key.clone(), h.value.clone()))
        .collect();
    let content_type = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map_or("", |(_, v)| v.as_str());

    let cookies = response_cookies(&headers);
    let body = result.response_body.as_deref().unwrap_or_default();

    let pane_data = crate::widgets::message_pane::PaneData {
        cache_key: ((fuzzer.run_id.unwrap_or(0) as u64) << 32) | (result.idx as u64),
        title: "Response",
        headers: &headers,
        body,
        content_type,
        params: None,
        cookies: Some(cookies),
        form_parser: Some(crate::features::request_editor::body::captured_form),
    };

    if let Some(save) = crate::widgets::message_pane::show(
        ui,
        "fuzzer_response_pane",
        &mut fuzzer.response_pane,
        &pane_data,
    ) {
        cmds.push(AppCommand::SaveBody(save));
    }
}

fn split_cookie(pair: &str) -> (String, String) {
    let (key, value) = pair.trim().split_once('=').unwrap_or((pair.trim(), ""));
    (key.to_string(), value.to_string())
}

fn response_cookies(headers: &[(String, String)]) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, v)| split_cookie(v.split(';').next().unwrap_or_default()))
        .collect()
}

fn sort_header(
    ui: &mut egui::Ui,
    title: &str,
    col: FuzzSortColumn,
    current_col: Option<&FuzzSortColumn>,
    desc: bool,
    next_sort: &mut Option<FuzzSortColumn>,
) {
    let is_active = current_col == Some(&col);
    let arrow = if is_active {
        if desc { " ▾" } else { " ▴" }
    } else {
        ""
    };
    if ui.selectable_label(is_active, RichText::new(format!("{title}{arrow}")).strong()).clicked() {
        *next_sort = Some(col);
    }
}
