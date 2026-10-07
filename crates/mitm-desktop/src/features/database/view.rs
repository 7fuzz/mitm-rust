use std::path::Path;
use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use crate::theme::{self, Tone};
use crate::widgets::split;
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
        ui.add(egui::TextEdit::singleline(&mut state.table_filter).hint_text("Filter tables…").desired_width(ui.available_width() - 28.0));
        if ui.small_button("⟳").on_hover_text("Refresh table list").clicked() {
            state.load_tables(db_path);
        }
    });
    left.add_space(4.0);

    let filter = state.table_filter.trim().to_lowercase();
    let tables: Vec<_> = state
        .tables
        .iter()
        .filter(|t| filter.is_empty() || t.name.to_lowercase().contains(&filter))
        .cloned()
        .collect();

    egui::ScrollArea::vertical()
        .id_salt("db_tables_list")
        .auto_shrink([false, false])
        .show(&mut left, |ui| {
            for table in tables {
                let selected = state.selected_table.as_deref() == Some(&table.name);
                let label = format!("{} ({})", table.name, table.row_count);
                if ui.selectable_label(selected, label).clicked() {
                    state.select_table(db_path, table.name);
                }
            }
        });

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

fn render_grid(ui: &mut egui::Ui, state: &mut DatabaseState, db_path: &Path) {
    let Some(current_rows) = &state.current_rows else {
        ui.label(RichText::new("No rows loaded").weak().italics());
        return;
    };

    let columns = current_rows.columns.clone();
    let rows = current_rows.rows.clone();

    if rows.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(RichText::new("Table is empty or no rows match filter").weak().italics());
        });
        return;
    }

    egui::ScrollArea::horizontal()
        .id_salt("db_table_hscroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut builder = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .sense(egui::Sense::click())
                .auto_shrink([false, false]);

            for _ in 0..columns.len() {
                builder = builder.column(Column::initial(130.0).at_least(60.0).clip(true));
            }

            builder
                .header(24.0, |mut header| {
                    for col_name in &columns {
                        let col_info = state.table_columns.iter().find(|c| &c.name == col_name).cloned();
                        let is_sorted = state.sort_column.as_deref() == Some(col_name);
                        let sort_arrow = if is_sorted {
                            if state.sort_desc { " ▼" } else { " ▲" }
                        } else {
                            ""
                        };
                        let label = format!("{}{}", col_name, sort_arrow);
                        header.col(|ui| {
                            let resp = ui.selectable_label(is_sorted, RichText::new(label).strong().size(11.0));
                            if resp.clicked() {
                                state.toggle_sort(db_path, col_name);
                            }
                            if let Some(info) = &col_info {
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
                        let idx = row.index();
                        let selected = state.selected_row_index == Some(idx);
                        row.set_selected(selected);
                        for cell in &rows[idx] {
                            row.col(|ui| match cell {
                                serde_json::Value::Null => {
                                    ui.label(RichText::new("NULL").weak().italics());
                                }
                                serde_json::Value::String(s) => {
                                    ui.add(egui::Label::new(s).truncate());
                                }
                                other => {
                                    ui.add(egui::Label::new(other.to_string()).truncate());
                                }
                            });
                        }
                        if row.response().clicked() {
                            state.select_row(db_path, idx);
                        }
                    });
                });
        });
}

fn render_detail(ui: &mut egui::Ui, state: &mut DatabaseState) {
    ui.horizontal(|ui| {
        ui.strong("Row Details");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("✕").clicked() {
                state.selected_row_index = None;
                state.detail_row_values = None;
            }
            if ui.button("Copy row as JSON").clicked() {
                if let (Some(current), Some(values)) = (&state.current_rows, &state.detail_row_values) {
                    let mut obj = serde_json::Map::new();
                    for (c, v) in current.columns.iter().zip(values.iter()) {
                        obj.insert(c.clone(), v.clone());
                    }
                    if let Ok(json) = serde_json::to_string_pretty(&serde_json::Value::Object(obj)) {
                        ui.ctx().copy_text(json);
                    }
                }
            }
        });
    });
    ui.separator();

    let columns = state.current_rows.as_ref().map(|r| r.columns.clone()).unwrap_or_default();
    let values = state.detail_row_values.clone().unwrap_or_default();

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
                        let mut copy = s.clone();
                        ui.add(egui::TextEdit::multiline(&mut copy).desired_rows(1).desired_width(f32::INFINITY));
                    }
                    other => {
                        let mut copy = other.to_string();
                        ui.add(egui::TextEdit::multiline(&mut copy).desired_rows(1).desired_width(f32::INFINITY));
                    }
                }
            }
        });
}

fn query_view(ui: &mut egui::Ui, _state: &mut DatabaseState, _db_path: &Path) {
    ui.label(RichText::new("SQL Query editor").weak().italics());
}
