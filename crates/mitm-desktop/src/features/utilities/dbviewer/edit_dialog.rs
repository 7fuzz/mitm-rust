use std::collections::HashMap;

use dbviewer_core::{display_columns, row_label, Assignment, ForeignKey, QueryResult, TableRef};
use egui::RichText;

use super::fk::FkState;
use super::grid::{self, EditAccess, GridOptions};
use super::tabs::TabId;

pub enum Outcome {
    Open,
    Save(Vec<Assignment>),
    Cancel,
}

pub const PICKER_LIMIT: usize = 200;
const PICKER_COLUMNS: usize = 6;

pub enum Request {
    Lookup { col: usize, generation: u64, fk: ForeignKey, search: String },
    Label { col: usize, fk: ForeignKey, value: String },
    PickerKeys { table: TableRef },
    PickerLabel { column: String, value: String, fk: ForeignKey },
}

struct Picker {
    col: usize,
    fk: ForeignKey,
    search: String,
    requested: Option<String>,
    /// Every column of the referenced rows, used to read the chosen key.
    result: Option<QueryResult>,
    /// The same rows cut down to the columns worth showing.
    view: Option<QueryResult>,
    /// Foreign keys of the referenced table, so its own references show names on hover too.
    state: FkState,
    generation: u64,
    loading: bool,
    error: Option<String>,
}

/// The edit form for one row, shown in a panel beside the table.
pub struct EditDialog {
    pub tab: TabId,
    pub table: TableRef,
    pub saving: bool,
    pub error: Option<String>,
    columns: Vec<String>,
    key_columns: Vec<String>,
    original: Vec<Option<String>>,
    values: Vec<String>,
    nulls: Vec<bool>,
    foreign_keys: Vec<ForeignKey>,
    enums: HashMap<String, Vec<String>>,
    /// Column index -> (key value, name of the referenced row); stale once the value is edited.
    labels: HashMap<usize, (String, String)>,
    picker: Option<Picker>,
    requests: Vec<Request>,
    generation: u64,
}

impl EditDialog {
    pub fn new(tab: TabId, table: TableRef, columns: Vec<String>, original: Vec<Option<String>>, key_columns: Vec<String>) -> Self {
        let values = original.iter().map(|v| v.clone().unwrap_or_default()).collect();
        let nulls = original.iter().map(Option::is_none).collect();
        Self {
            tab,
            table,
            saving: false,
            error: None,
            columns,
            key_columns,
            original,
            values,
            nulls,
            foreign_keys: Vec::new(),
            enums: HashMap::new(),
            labels: HashMap::new(),
            picker: None,
            requests: Vec::new(),
            generation: 0,
        }
    }

    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    fn foreign_key_for(&self, col: usize) -> Option<&ForeignKey> {
        self.foreign_keys.iter().find(|fk| fk.column == self.columns[col])
    }

    pub fn set_foreign_keys(&mut self, foreign_keys: Vec<ForeignKey>) {
        self.foreign_keys = foreign_keys;
        for col in 0..self.columns.len() {
            let (Some(fk), Some(value)) = (self.foreign_key_for(col), self.original[col].as_ref().filter(|v| !v.is_empty())) else { continue };
            self.requests.push(Request::Label { col, fk: fk.clone(), value: value.clone() });
        }
    }

    pub fn set_enums(&mut self, enums: HashMap<String, Vec<String>>) {
        self.enums = enums;
    }

    pub fn set_label(&mut self, col: usize, value: String, result: Result<QueryResult, String>) {
        let Ok(result) = result else { return };
        let Some(fk) = self.foreign_key_for(col) else { return };
        if let Some(label) = result.rows.first().and_then(|row| row_label(&result.columns, row, &fk.target_column)) {
            self.labels.insert(col, (value, label));
        }
    }

    pub fn set_lookup(&mut self, col: usize, generation: u64, result: Result<QueryResult, String>) {
        let Some(picker) = self.picker.as_mut().filter(|p| p.col == col && p.generation == generation) else { return };
        picker.loading = false;
        match result {
            Ok(result) => {
                let shown = display_columns(&result.columns, &picker.fk.target_column, PICKER_COLUMNS);
                picker.view = Some(QueryResult {
                    columns: shown.iter().map(|&c| result.columns[c].clone()).collect(),
                    rows: result.rows.iter().map(|row| shown.iter().map(|&c| row[c].clone()).collect()).collect(),
                    ..QueryResult::default()
                });
                picker.result = Some(result);
            }
            Err(e) => picker.error = Some(e),
        }
    }

    pub fn set_picker_keys(&mut self, keys: Vec<ForeignKey>) {
        if let Some(picker) = self.picker.as_mut() {
            picker.state.keys = keys;
        }
    }

    pub fn set_picker_label(&mut self, column: &str, value: String, result: Result<QueryResult, String>) {
        if let Some(picker) = self.picker.as_mut() {
            picker.state.resolve(column, value, result);
        }
    }

    fn open_picker(&mut self, col: usize) {
        let Some(fk) = self.foreign_key_for(col).cloned() else { return };
        self.requests.push(Request::PickerKeys { table: fk.target.clone() });
        self.picker = Some(Picker {
            col,
            fk,
            search: String::new(),
            requested: None,
            result: None,
            view: None,
            state: FkState::default(),
            generation: 0,
            loading: false,
            error: None,
        });
    }

    /// Identifies the row by its primary key as it was loaded, not as it is being edited.
    pub fn key(&self) -> Result<Vec<Assignment>, String> {
        self.key_columns
            .iter()
            .map(|name| {
                let index = self.columns.iter().position(|c| c == name).ok_or_else(|| format!("primary key column {name} is not in the result"))?;
                Ok((name.clone(), self.original[index].clone()))
            })
            .collect()
    }

    fn changes(&self) -> Vec<Assignment> {
        self.columns
            .iter()
            .enumerate()
            .filter_map(|(i, column)| {
                let new = if self.nulls[i] { None } else { Some(self.values[i].clone()) };
                (new != self.original[i]).then(|| (column.clone(), new))
            })
            .collect()
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Outcome {
        if self.picker.is_some() {
            self.show_picker(ui);
            return Outcome::Open;
        }
        let mut outcome = Outcome::Open;
        let changes = self.changes();
        let mut enter_pressed = false;
        let mut pick = None;

        ui.add_space(4.0);
        ui.heading("Edit row");
        ui.label(RichText::new(self.table.qualified()).weak().small());
        ui.horizontal(|ui| {
            let can_save = !changes.is_empty() && !self.saving;
            if ui.add_enabled(can_save, egui::Button::new("Save")).clicked() || (enter_pressed && can_save) {
                outcome = Outcome::Save(changes.clone());
            }
            if ui.button("Cancel").clicked() || ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                outcome = Outcome::Cancel;
            }
            if self.saving {
                ui.spinner();
            }
            ui.label(RichText::new(format!("{} changed", changes.len())).weak());
        });
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        ui.separator();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for i in 0..self.columns.len() {
                let is_key = self.key_columns.contains(&self.columns[i]);
                let has_fk = self.foreign_keys.iter().any(|fk| fk.column == self.columns[i]);
                ui.horizontal(|ui| {
                    let name = RichText::new(&self.columns[i]);
                    ui.label(if is_key { name.strong() } else { name }).on_hover_text(if is_key { "Primary key" } else { "" });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_enabled(!self.saving, egui::Checkbox::new(&mut self.nulls[i], "NULL"));
                        let usable = !self.saving && !self.nulls[i];
                        if let Some(options) = self.enums.get(&self.columns[i]) {
                            ui.add_enabled_ui(usable, |ui| {
                                egui::ComboBox::from_id_salt(("enum_options", i)).selected_text("Values").width(70.0).show_ui(ui, |ui| {
                                    for option in options {
                                        if ui.selectable_label(self.values[i] == *option, option).clicked() {
                                            self.values[i] = option.clone();
                                        }
                                    }
                                });
                            });
                        }
                        if has_fk && ui.add_enabled(usable, egui::Button::new("Pick...")).clicked() {
                            pick = Some(i);
                        }
                    });
                });
                let multiline = self.values[i].contains('\n') || self.values[i].len() > 80;
                let edit = if multiline {
                    egui::TextEdit::multiline(&mut self.values[i]).desired_rows(3)
                } else {
                    egui::TextEdit::singleline(&mut self.values[i])
                };
                let response = ui.add_enabled(!self.nulls[i] && !self.saving, edit.desired_width(f32::INFINITY));
                enter_pressed |= !multiline && response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if let Some((_, label)) = self.labels.get(&i).filter(|(value, _)| *value == self.values[i] && !self.nulls[i]) {
                    ui.label(RichText::new(format!("-> {label}")).weak().small());
                }
                ui.add_space(8.0);
            }
        });
        if let Some(col) = pick {
            self.open_picker(col);
        }
        if enter_pressed && !changes.is_empty() && !self.saving {
            outcome = Outcome::Save(changes);
        }
        outcome
    }

    fn show_picker(&mut self, ui: &mut egui::Ui) {
        let Some(picker) = self.picker.as_mut() else { return };
        let mut close = ui.ctx().input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let mut chosen = None;
        let mut lookups = Vec::new();

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            close |= ui.button("Back").clicked();
            ui.heading(format!("Pick from {}", picker.fk.target.name));
            if picker.loading {
                ui.spinner();
            }
        });
        let search = ui.add(egui::TextEdit::singleline(&mut picker.search).hint_text("search any column").desired_width(f32::INFINITY));
        if picker.requested.is_none() {
            search.request_focus();
        }
        if picker.requested.as_ref() != Some(&picker.search) {
            self.generation += 1;
            picker.generation = self.generation;
            picker.loading = true;
            picker.error = None;
            picker.requested = Some(picker.search.clone());
            self.requests.push(Request::Lookup { col: picker.col, generation: picker.generation, fk: picker.fk.clone(), search: picker.search.clone() });
        }
        if let Some(error) = &picker.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        if let (Some(result), Some(view)) = (&picker.result, &picker.view) {
            let note = if result.rows.len() == PICKER_LIMIT { format!("first {PICKER_LIMIT} rows; search to narrow") } else { format!("{} row(s)", result.rows.len()) };
            ui.label(RichText::new(note).weak().small());
            let opts = GridOptions { sort: None, sortable: false, edit: &EditAccess::Hidden, fk: Some(&picker.state), pick: true };
            let action = grid::show(ui, view, &opts);
            lookups = action.lookups;
            chosen = action.pick_row;
        }
        for (column, value) in lookups {
            if picker.state.start(&column, &value) {
                if let Some(fk) = picker.state.key_for(&column).cloned() {
                    self.requests.push(Request::PickerLabel { column, value, fk });
                }
            }
        }

        if let Some(row) = chosen.and_then(|i| picker.result.as_ref().and_then(|r| r.rows.get(i).map(|row| (r.columns.clone(), row.clone())))) {
            let (columns, row) = row;
            let key = columns.iter().position(|c| *c == picker.fk.target_column).and_then(|i| row.get(i).cloned().flatten());
            if let Some(value) = key {
                let col = picker.col;
                if let Some(label) = row_label(&columns, &row, &picker.fk.target_column) {
                    self.labels.insert(col, (value.clone(), label));
                }
                self.values[col] = value;
                self.nulls[col] = false;
                close = true;
            }
        }
        if close {
            self.picker = None;
        }
    }
}
