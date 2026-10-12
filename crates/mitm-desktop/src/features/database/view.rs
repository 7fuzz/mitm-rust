use std::path::Path;
use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use crate::ui::theme::{self, Tone};
use crate::widgets::split;
use mitm_core::db::browser::DbColumn;
use super::{DatabaseState, DatabaseTab};

pub fn show(ui: &mut egui::Ui, state: &mut DatabaseState, db_path: &Path) {
    state.ensure_loaded(db_path);

    ui.horizontal(|ui| {
        if ui.selectable_label(state.tab == DatabaseTab::Tables, "Tables").clicked() {
            state.tab = DatabaseTab::Tables;
        }
        if ui.selectable_label(state.tab == DatabaseTab::Query, "SQL Query").clicked() {
            state.tab = DatabaseTab::Query;
        }
    });
    ui.separator();

    match state.tab {
        DatabaseTab::Tables => tables_view(ui, state, db_path),
        DatabaseTab::Query => query_view(ui, state, db_path),
    }
}

fn tables_view(ui: &mut egui::Ui, state: &mut DatabaseState, db_path: &Path) {
    let [mut left, mut right] = split::side_by_side(ui, "db_tables_split", &mut state.tables_fraction);

    left.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut state.table_filter).hint_text("Filter tables…").desired_width((ui.available_width() - 28.0).max(60.0)));
        if ui.small_button("⟳").on_hover_text("Refresh table list").clicked() {
            state.load_tables(db_path);
        }
    });
    left.add_space(4.0);

    let filter = state.table_filter.trim().to_lowercase();
    let mut clicked = None;
    egui::ScrollArea::vertical()
        .id_salt("db_tables_list")
        .auto_shrink([false, false])
        .show(&mut left, |ui| {
            for table in state.tables.iter().filter(|t| filter.is_empty() || t.name.to_lowercase().contains(&filter)) {
                let selected = state.selected_table.as_deref() == Some(&table.name);
                let label = format!("{} ({})", table.name, table.row_count);
                if ui.selectable_label(selected, label).clicked() {
                    clicked = Some(table.name.clone());
                }
            }
        });
    if let Some(name) = clicked {
        state.select_table(db_path, name);
    }

    if let Some(error) = &state.error {
        right.label(RichText::new(error).color(theme::color(&right, Tone::Danger)));
        right.add_space(4.0);
    }

    if state.selected_table.is_none() {
        right.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(RichText::new("Select a table to browse").weak().italics());
        });
        return;
    }

    table_content(&mut right, state, db_path);
}

fn table_content(ui: &mut egui::Ui, state: &mut DatabaseState, db_path: &Path) {
    ui.horizontal(|ui| {
        let search_edit = ui.add(
            egui::TextEdit::singleline(&mut state.search_query)
                .hint_text("Search across columns…")
                .desired_width(180.0),
        );
        crate::widgets::search::claim_focus(&search_edit);
        if search_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            state.page = 0;
            state.load_current_page(db_path);
        }
        if ui.small_button("Search").clicked() {
            state.page = 0;
            state.load_current_page(db_path);
        }
        if !state.search_query.is_empty() && ui.small_button("Clear").clicked() {
            state.search_query.clear();
            state.page = 0;
            state.load_current_page(db_path);
        }

        ui.separator();

        let total = state.current_rows.as_ref().map(|r| r.total).unwrap_or(0);
        let total_pages = ((total + state.page_size - 1) / state.page_size).max(1);

        if ui.add_enabled(state.page > 0, egui::Button::new("⏮")).clicked() {
            state.page = 0;
            state.load_current_page(db_path);
        }
        if ui.add_enabled(state.page > 0, egui::Button::new("◀")).clicked() {
            state.page -= 1;
            state.load_current_page(db_path);
        }
        ui.label(format!("Page {} / {} ({} rows)", state.page + 1, total_pages, total));
        if ui.add_enabled(state.page + 1 < total_pages, egui::Button::new("▶")).clicked() {
            state.page += 1;
            state.load_current_page(db_path);
        }
        if ui.add_enabled(state.page + 1 < total_pages, egui::Button::new("⏭")).clicked() {
            state.page = total_pages - 1;
            state.load_current_page(db_path);
        }
    });

    ui.add_space(4.0);

    if state.selected_row_index.is_some() {
        let [mut grid_pane, mut detail_pane] = split::side_by_side(ui, "db_detail_split", &mut state.detail_split_fraction);
        render_grid(&mut grid_pane, state, db_path);
        render_detail(&mut detail_pane, state);
    } else {
        render_grid(ui, state, db_path);
    }
}

enum GridClick {
    Sort(String),
    Row(usize),
}

fn render_grid(ui: &mut egui::Ui, state: &mut DatabaseState, db_path: &Path) {
    let Some(current_rows) = &state.current_rows else {
        return;
    };
    if current_rows.rows.is_empty() {
        empty_note(ui, "No rows");
        return;
    }
    let sort = state.sort_column.as_deref().map(|c| (c, state.sort_desc));
    let click = grid(
        ui,
        "db_table_hscroll",
        &current_rows.columns,
        &current_rows.rows,
        &state.table_columns,
        Some(sort),
        state.selected_row_index,
    );
    match click {
        Some(GridClick::Sort(column)) => state.toggle_sort(db_path, &column),
        Some(GridClick::Row(index)) => state.select_row(db_path, index),
        None => {}
    }
}

fn empty_note(ui: &mut egui::Ui, text: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.label(RichText::new(text).weak().italics());
    });
}

/// `sort` is `None` for a grid without sortable headers, `Some(None)` for sortable but unsorted.
fn grid(
    ui: &mut egui::Ui,
    id_salt: &str,
    columns: &[String],
    rows: &[Vec<serde_json::Value>],
    column_info: &[DbColumn],
    sort: Option<Option<(&str, bool)>>,
    selected: Option<usize>,
) -> Option<GridClick> {
    let mut click = None;
    egui::ScrollArea::horizontal()
        .id_salt(id_salt)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut builder = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .sense(egui::Sense::click())
                .auto_shrink([false, false]);
            for _ in columns {
                builder = builder.column(Column::initial(130.0).at_least(60.0).clip(true));
            }
            builder
                .header(24.0, |mut header| {
                    for name in columns {
                        header.col(|ui| {
                            let Some(sort) = sort else {
                                ui.strong(name);
                                return;
                            };
                            let direction = sort.filter(|(col, _)| col == name).map(|(_, desc)| desc);
                            let arrow = match direction {
                                Some(true) => " ▼",
                                Some(false) => " ▲",
                                None => "",
                            };
                            let resp = ui.selectable_label(direction.is_some(), RichText::new(format!("{name}{arrow}")).strong().size(11.0));
                            if resp.clicked() {
                                click = Some(GridClick::Sort(name.clone()));
                            }
                            if let Some(info) = column_info.iter().find(|c| &c.name == name) {
                                resp.on_hover_text(format!(
                                    "Type: {}\nPrimary Key: {}\nNot Null: {}",
                                    if info.decl_type.is_empty() { "ANY" } else { &info.decl_type },
                                    info.primary_key,
                                    info.not_null
                                ));
                            }
                        });
                    }
                })
                .body(|body| {
                    body.rows(22.0, rows.len(), |mut row| {
                        let index = row.index();
                        row.set_selected(selected == Some(index));
                        for cell in &rows[index] {
                            row.col(|ui| cell_label(ui, cell));
                        }
                        if row.response().clicked() {
                            click = Some(GridClick::Row(index));
                        }
                    });
                });
        });
    click
}

fn cell_label(ui: &mut egui::Ui, cell: &serde_json::Value) {
    match cell {
        serde_json::Value::Null => {
            ui.label(RichText::new("NULL").weak().italics());
        }
        serde_json::Value::String(s) => {
            ui.add(egui::Label::new(s).truncate());
        }
        other => {
            ui.add(egui::Label::new(other.to_string()).truncate());
        }
    }
}

fn render_detail(ui: &mut egui::Ui, state: &mut DatabaseState) {
    let columns = state.current_rows.as_ref().map_or(&[][..], |r| &r.columns[..]);
    let values = state.detail_row_values.as_deref().unwrap_or_default();
    let mut close = false;
    ui.horizontal(|ui| {
        ui.strong("Row Details");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            close = ui.small_button("✖").clicked();
            if ui.button("Copy row as JSON").clicked() {
                let obj: serde_json::Map<_, _> = columns.iter().cloned().zip(values.iter().cloned()).collect();
                if let Ok(json) = serde_json::to_string_pretty(&serde_json::Value::Object(obj)) {
                    ui.ctx().copy_text(json);
                }
            }
        });
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("db_row_detail_scroll")
        .show(ui, |ui| {
            for (i, col_name) in columns.iter().enumerate() {
                let val = values.get(i).unwrap_or(&serde_json::Value::Null);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(col_name).strong());
                    if let Some(col_info) = state.table_columns.iter().find(|c| &c.name == col_name) {
                        ui.label(RichText::new(&col_info.decl_type).weak().small());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("📋").on_hover_text("Copy value").clicked() {
                            let text = match val {
                                serde_json::Value::Null => "NULL".to_string(),
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            ui.ctx().copy_text(text);
                        }
                    });
                });
                match val {
                    serde_json::Value::Null => {
                        ui.label(RichText::new("NULL").weak().italics());
                    }
                    serde_json::Value::String(s) => {
                        ui.add(egui::TextEdit::multiline(&mut s.as_str()).desired_rows(1).desired_width(f32::INFINITY));
                    }
                    other => {
                        ui.add(egui::TextEdit::multiline(&mut other.to_string().as_str()).desired_rows(1).desired_width(f32::INFINITY));
                    }
                }
            }
        });

    if close {
        state.selected_row_index = None;
        state.detail_row_values = None;
    }
}

fn query_view(ui: &mut egui::Ui, state: &mut DatabaseState, db_path: &Path) {
    ui.horizontal(|ui| {
        ui.strong("SQL Editor");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_run = !state.sql_query.trim().is_empty();
            let is_ctrl_enter = ui.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Enter));
            if ui.add_enabled(can_run, egui::Button::new("Run (Ctrl+Enter)")).clicked() || (can_run && is_ctrl_enter) {
                state.run_query(db_path);
            }
        });
    });
    ui.add_space(4.0);

    let mut layouter = crate::widgets::code_view::sql_layouter();

    ui.add(
        egui::TextEdit::multiline(&mut state.sql_query)
            .font(egui::TextStyle::Monospace)
            .desired_rows(6)
            .desired_width(f32::INFINITY)
            .layouter(&mut layouter),
    );

    if let Some(error) = &state.query_error {
        ui.add_space(6.0);
        ui.label(RichText::new(error).color(theme::color(ui, Tone::Danger)).strong());
    }

    if let Some(result) = &state.query_result {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} rows returned in {}ms{}",
                    result.rows.len(),
                    result.elapsed_ms,
                    if result.truncated { " (truncated to 1000 rows)" } else { "" }
                ))
                .weak()
                .small(),
            );
        });
        ui.add_space(4.0);
        if result.rows.is_empty() {
            empty_note(ui, "No rows");
        } else {
            grid(ui, "db_query_table_hscroll", &result.columns, &result.rows, &[], None, None);
        }
    }
}
