use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use mitm_core::repeater::HeaderItem;
use crate::app::command::{AppCommand, Commands};
use crate::features::request_editor::body::{content_type_for, REPEATER_BODY_TYPES};
use crate::features::request_editor::body_editor;
use crate::features::request_editor::kv_table::{self, standard_headers, KvLabels};
use crate::features::request_editor::method::method_combo;
use crate::features::request_editor::url_params::{build_url, new_id, parse_query};
use crate::util::format::{format_bytes, format_duration};
use crate::ui::theme::{self, Tone};
use crate::widgets::copy_menu;
use crate::widgets::pills::render_status_pill;
use crate::widgets::tri_state::tri_state_button;
use crate::widgets::{split, template};
use super::{Cmd, AttackType, ConfigTab, FuzzSearchScope, FuzzSortColumn, FuzzerState, MatchRule, PayloadConfig, PayloadKind, RequestTab, RunStatus};
use crate::ui::icons;
use crate::util::format::format_count;
use crate::widgets::button::{icon_button, primary};
use crate::widgets::card::card;
use crate::widgets::search::search_field_sized;
use crate::widgets::segmented::segmented;
use crate::widgets::subtabs::subtabs;

pub fn show(ui: &mut egui::Ui, fuzzer: &mut FuzzerState, cmds: &mut Commands) {
    toolbar(ui, fuzzer, cmds);
    ui.add_space(4.0);
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
            if ui.button(RichText::new("Stop").color(theme::color(ui, Tone::Danger))).clicked() {
                cmds.push(Cmd::Stop);
            }
            ui.spinner();
        } else {
            if ui.add_enabled(!fuzzer.starting, primary(ui, "Start")).clicked() {
                cmds.push(Cmd::Start(fuzzer.config()));
            }
        }
        runs_popover(ui, fuzzer, cmds);
        if fuzzer.status != RunStatus::Idle {
            ui.label(RichText::new(format!("{} / {}", format_count(fuzzer.results.len()), format_count(fuzzer.total))).monospace());
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
                        cmds.push(Cmd::SaveRun { run_id, name });
                        fuzzer.saving_run = false;
                    }
                    if ui.button(icons::X).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
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
        if finished && ui.button("Clear").clicked() {
            fuzzer.clear();
        }
    });
}

fn runs_popover(ui: &mut egui::Ui, fuzzer: &mut FuzzerState, cmds: &mut Commands) {
    let menu = ui.menu_button(icons::CLOCK_COUNTER_CLOCKWISE, |ui| {
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
                        ui.label(&run.name);
                        let date = chrono::DateTime::from_timestamp_millis(run.created_at_ms)
                            .map(|at| at.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_default();
                        ui.weak(format!("{} · {} reqs · {}", run.attack_type, run.total, date));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_button(ui, icons::TRASH, "Delete run").clicked() {
                            to_delete = Some(run.id);
                        }
                        if ui.button("Open").clicked() {
                            to_load = Some(run.id);
                            ui.close();
                        }
                    });
                });
                ui.add_space(4.0);
            }
            if let Some(id) = to_load {
                cmds.push(Cmd::LoadRun(id));
            }
            if let Some(id) = to_delete {
                cmds.push(Cmd::DeleteRun(id));
            }
        });
    });
    if menu.response.clicked() {
        cmds.push(Cmd::ReloadSavedRuns);
    }
    menu.response.on_hover_text("Saved runs");
}

fn config(ui: &mut egui::Ui, fuzzer: &mut FuzzerState) {
    ui.horizontal(|ui| {
        let variables = fuzzer.variable_names().len();
        let rules = fuzzer.match_rules.len();
        let tabs = [
            (ConfigTab::Request, "Request", None),
            (ConfigTab::Payloads, "Payloads", Some(variables)),
            (ConfigTab::Match, "Match", Some(rules)),
            (ConfigTab::Options, "Options", None),
        ];
        subtabs(ui, &mut fuzzer.config_tab, &tabs);
    });
    ui.add_space(4.0);
    match fuzzer.config_tab {
        ConfigTab::Request => request_tab(ui, fuzzer),
        ConfigTab::Payloads => payloads_tab(ui, fuzzer),
        ConfigTab::Match => match_tab(ui, fuzzer),
        ConfigTab::Options => options_tab(ui, fuzzer),
    }
}

fn match_tab(ui: &mut egui::Ui, fuzzer: &mut FuzzerState) {
    ui.horizontal(|ui| {
        if icon_button(ui, icons::PLUS, "Add match rule").clicked() {
            let next_idx = fuzzer.match_rules.len() + 1;
            fuzzer.match_rules.push(MatchRule {
                name: format!("match{}", next_idx),
                kind: "contains".to_string(),
                pattern: String::new(),
            });
        }
    });

    if fuzzer.match_rules.is_empty() {
        ui.weak("No match rules");
        return;
    }

    let mut remove_idx = None;
    egui::ScrollArea::vertical().id_salt("fuzzer_match_scroll").auto_shrink([false, false]).show(ui, |ui| {
        TableBuilder::new(ui)
            .id_salt("fuzzer_match_table")
            .striped(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(130.0).at_least(80.0))
            .column(Column::exact(130.0))
            .column(Column::remainder())
            .column(Column::exact(28.0))
            .header(20.0, |mut header| {
                for title in ["Name", "Type", "Pattern"] {
                    header.col(|ui| {
                        ui.label(RichText::new(title).small().weak());
                    });
                }
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
                        if icon_button(ui, icons::TRASH, "Remove rule").clicked() {
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
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        let params = fuzzer.params.iter().filter(|p| p.enabled && !p.key.trim().is_empty()).count();
        let headers = fuzzer.headers.iter().filter(|h| h.enabled && !h.key.trim().is_empty()).count();
        let tabs = [(RequestTab::Params, "Params", Some(params)), (RequestTab::Headers, "Headers", Some(headers)), (RequestTab::Body, "Body", None)];
        subtabs(ui, &mut fuzzer.request_tab, &tabs);
    });
    ui.add_space(4.0);

    match fuzzer.request_tab {
        RequestTab::Body => {
            ui.horizontal(|ui| {
                body_type_combo(ui, fuzzer);
                if let Some(next) = body_editor::convert_menu(ui, &fuzzer.body_type, &REPEATER_BODY_TYPES) {
                    set_body_type(ui.ctx(), fuzzer, next, true);
                }
            });
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
    if let Some(next) = picked {
        set_body_type(ui.ctx(), fuzzer, next, false);
    }
}

fn set_body_type(ctx: &egui::Context, fuzzer: &mut FuzzerState, next: &str, convert: bool) {
    set_content_type_header(&mut fuzzer.headers, content_type_for(next));
    body_editor::change_type(ctx, "fuzzer", &fuzzer.body_type, next, &mut fuzzer.body_content, convert);
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
    ui.add_space(4.0);

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
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{{{{{name}}}}}")).monospace().strong());
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
    ui.add_space(4.0);
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
        ui.add_space(8.0);
        if ui
            .selectable_label(fuzzer.hide_common_size, "Hide most frequent length")
            .on_hover_text("Hide responses with the most frequent response size")
            .clicked()
        {
            fuzzer.hide_common_size = !fuzzer.hide_common_size;
        }
        ui.add_space(8.0);
        search_field_sized(ui, &mut fuzzer.search_query, "Search", 180.0);
        segmented(ui, &mut fuzzer.search_scope, &[(FuzzSearchScope::Payload, "Payload"), (FuzzSearchScope::Response, "Response")]);

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
        .striped(false)
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
                        cmds.push(Cmd::SelectResult { run_id, idx: r.idx });
                    }
                }
                response.context_menu(|ui| {
                    if let Some(run_id) = fuzzer.run_id {
                        if ui.button("Send to Repeater").clicked() {
                            cmds.push(AppCommand::SendFuzzResultToRepeater { run_id, idx: r.idx });
                            ui.close();
                        }
                    }
                    if ui.button("Copy payload").clicked() {
                        let text = if r.payloads.len() == 1 {
                            r.payloads[0].clone()
                        } else {
                            r.payloads.join(", ")
                        };
                        ui.ctx().copy_text(text);
                        ui.close();
                    }
                    if let Some(run_id) = fuzzer.run_id {
                        ui.menu_button("Copy", |ui| {
                            if let Some(format) = copy_menu::items(ui, r.error.is_none()) {
                                cmds.push(Cmd::Copy { run_id, idx: r.idx, format });
                            }
                        });
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
        let stats = format!("{} · {}", format_duration(Some(result.duration_ms)), format_bytes(result.response_size));
        ui.label(RichText::new(stats).monospace().weak());
        if let Some(err) = &result.error {
            ui.label(RichText::new(err).color(theme::color(ui, Tone::Danger)));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(ui, icons::X, "Close response").clicked() {
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
    ui.add_space(4.0);

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
        if desc { icons::CARET_DOWN } else { icons::CARET_UP }
    } else {
        ""
    };
    if ui.selectable_label(is_active, RichText::new(format!("{title} {arrow}").trim_end()).strong()).clicked() {
        *next_sort = Some(col);
    }
}
