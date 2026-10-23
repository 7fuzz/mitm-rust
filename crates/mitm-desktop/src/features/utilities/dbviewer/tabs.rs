use std::collections::HashMap;

use dbviewer_core::{complete, Completion, Condition, Engine, Filter, Join, Op, QueryResult, Schema, Sort, Suggestion, TableRef};
use egui::RichText;
use egui_dock::{tab_viewer::OnCloseResponse, NodePath};

use super::fk::FkState;
use super::grid::{self, EditAccess, GridOptions};

pub const PAGE_SIZE: usize = 10;
const PAGE_SIZES: [usize; 7] = [10, 50, 100, 200, 500, 1000, 5000];

pub type TabId = u64;

pub struct Browser {
    pub table: TableRef,
    pub page: usize,
    pub page_size: usize,
    page_input: String,
    /// What the loaded rows were fetched with; `draft` is what the filter panel is editing.
    pub filter: Filter,
    pub draft: Filter,
    pub columns: Vec<String>,
    /// `None` until looked up; empty means no primary key, so rows can't be edited.
    pub primary_key: Option<Vec<String>>,
    pub result: Option<QueryResult>,
    pub loading: bool,
    pub show_filters: bool,
    pub fk: FkState,
    /// Bumped per request so a slow, superseded response can be dropped.
    pub generation: u64,
}

impl Browser {
    pub fn new(table: TableRef) -> Self {
        Self {
            table,
            page: 0,
            page_size: PAGE_SIZE,
            page_input: "1".to_string(),
            filter: Filter::default(),
            draft: blank_draft(),
            columns: Vec::new(),
            primary_key: None,
            result: None,
            loading: false,
            show_filters: false,
            fk: FkState::default(),
            generation: 0,
        }
    }
}

fn blank_draft() -> Filter {
    Filter { conditions: vec![Condition::default()], ..Filter::default() }
}

pub struct QueryTab {
    pub number: usize,
    pub sql: String,
    pub database: String,
    pub result: Option<QueryResult>,
    pub running: bool,
    pub suggest: Option<Suggest>,
    /// Cursor and text length at the last Escape, so the popup stays closed until either changes.
    dismissed: Option<(usize, usize)>,
    suggest_rect: Option<egui::Rect>,
}

impl QueryTab {
    pub fn new(number: usize, database: String) -> Self {
        Self { number, sql: String::new(), database, result: None, running: false, suggest: None, dismissed: None, suggest_rect: None }
    }
}

pub struct Suggest {
    start: usize,
    end: usize,
    items: Vec<Suggestion>,
    selected: usize,
}

/// What autocomplete knows about the connected server.
pub struct SchemaView<'a> {
    pub engine: Engine,
    pub databases: &'a [String],
    pub tables: &'a HashMap<String, Vec<TableRef>>,
    pub columns: &'a HashMap<String, HashMap<String, Vec<String>>>,
}

pub enum Content {
    Table(Browser),
    Query(QueryTab),
}

pub enum Cmd {
    LoadTable(TabId),
    EditRow { tab: TabId, row: usize },
    RunQuery(TabId),
    SaveQuery(TabId),
    FkLabel { tab: TabId, column: String, value: String },
    NewQuery(NodePath),
    Closed(TabId),
}

pub struct Viewer<'a> {
    pub contents: &'a mut HashMap<TabId, Content>,
    pub databases: &'a [String],
    pub connected: bool,
    pub read_only: bool,
    pub schema: SchemaView<'a>,
    pub cmds: &'a mut Vec<Cmd>,
}

impl egui_dock::TabViewer for Viewer<'_> {
    type Tab = TabId;

    fn id(&mut self, tab: &mut TabId) -> egui::Id {
        egui::Id::new(("tab", *tab))
    }

    fn title(&mut self, tab: &mut TabId) -> egui::WidgetText {
        match self.contents.get(tab) {
            Some(Content::Table(b)) if b.table.schema.is_empty() || b.table.schema == "public" => b.table.name.clone().into(),
            Some(Content::Table(b)) => format!("{}.{}", b.table.schema, b.table.name).into(),
            Some(Content::Query(q)) => format!("Query {}", q.number).into(),
            None => "...".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut TabId) {
        let id = *tab;
        match self.contents.get_mut(&id) {
            Some(Content::Table(browser)) => table_ui(ui, id, browser, self.read_only, self.cmds),
            Some(Content::Query(query)) => query_ui(ui, id, query, self.databases, self.connected, &self.schema, self.cmds),
            None => {}
        }
    }

    fn on_add(&mut self, path: NodePath) {
        self.cmds.push(Cmd::NewQuery(path));
    }

    fn on_close(&mut self, tab: &mut TabId) -> OnCloseResponse {
        self.cmds.push(Cmd::Closed(*tab));
        OnCloseResponse::Close
    }

    fn scroll_bars(&self, _tab: &TabId) -> [bool; 2] {
        [false, false]
    }
}

fn table_ui(ui: &mut egui::Ui, id: TabId, b: &mut Browser, read_only: bool, cmds: &mut Vec<Cmd>) {
    let rows = b.result.as_ref().map_or(0, |r| r.rows.len());
    let mut reload = false;
    ui.horizontal_wrapped(|ui| {
        breadcrumb(ui, &b.table);
        if ui.add_enabled(b.page > 0, egui::Button::new("Prev")).clicked() {
            b.page -= 1;
            reload = true;
        }
        let page_edit = ui.add(egui::TextEdit::singleline(&mut b.page_input).desired_width(44.0).hint_text("page"));
        if !page_edit.has_focus() {
            b.page_input = (b.page + 1).to_string();
        }
        if page_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(page) = b.page_input.trim().parse::<usize>().ok().filter(|p| *p >= 1) {
                b.page = page - 1;
                reload = true;
            }
        }
        page_edit.on_hover_text("Type a page number and press Enter to jump");
        if ui.add_enabled(rows == b.page_size, egui::Button::new("Next")).clicked() {
            b.page += 1;
            reload = true;
        }
        let previous_size = b.page_size;
        egui::ComboBox::from_id_salt(("page_size", id)).selected_text(format!("{} rows", b.page_size)).show_ui(ui, |ui| {
            for size in PAGE_SIZES {
                ui.selectable_value(&mut b.page_size, size, size.to_string());
            }
        });
        if b.page_size != previous_size {
            b.page = b.page * previous_size / b.page_size;
            reload = true;
        }
        ui.label(RichText::new(format!("rows {}-{}", b.page * b.page_size + 1, b.page * b.page_size + rows)).weak());
        reload |= ui.button("Reload").clicked();
        let active = b.filter.active_conditions();
        let label = if active > 0 { format!("Filter ({active})") } else { "Filter".to_string() };
        ui.toggle_value(&mut b.show_filters, label);
        if b.loading {
            ui.spinner();
        }
    });

    if b.show_filters {
        ui.separator();
        reload |= filter_panel(ui, id, b);
    }
    ui.separator();

    let access = edit_access(b, read_only);
    if let Some(result) = &b.result {
        let opts = GridOptions { sort: b.filter.sort.as_ref(), sortable: true, edit: &access, fk: Some(&b.fk), pick: false };
        let action = grid::show(ui, result, &opts);
        for (column, value) in action.lookups {
            if b.fk.start(&column, &value) {
                cmds.push(Cmd::FkLabel { tab: id, column, value });
            }
        }
        if let Some(column) = action.sort_by {
            b.filter.sort = next_sort(b.filter.sort.take(), column);
            b.page = 0;
            reload = true;
        }
        if let Some(row) = action.edit_row {
            cmds.push(Cmd::EditRow { tab: id, row });
        }
    }
    if reload {
        cmds.push(Cmd::LoadTable(id));
    }
}

fn breadcrumb(ui: &mut egui::Ui, table: &dbviewer_core::TableRef) {
    ui.spacing_mut().item_spacing.x = 4.0;
    ui.label(RichText::new(format!("{} /", table.database)).weak());
    if !table.schema.is_empty() {
        ui.label(RichText::new(format!("{} /", table.schema)).weak());
    }
    ui.label(RichText::new(&table.name).strong());
    ui.add_space(8.0);
}

fn edit_access(b: &Browser, read_only: bool) -> EditAccess {
    use dbviewer_core::TableKind;
    if matches!(b.table.kind, TableKind::View | TableKind::MaterializedView) {
        return EditAccess::Disabled("Views can't be edited");
    }
    if read_only {
        return EditAccess::Disabled("Read-only connection; untick Read-only session in the connection to edit");
    }
    match &b.primary_key {
        Some(key) if !key.is_empty() => EditAccess::Enabled,
        Some(_) => EditAccess::Disabled("This table has no primary key"),
        None => EditAccess::Disabled("Looking up the primary key..."),
    }
}

/// Ascending, then descending, then unsorted.
fn next_sort(current: Option<Sort>, column: String) -> Option<Sort> {
    match current {
        Some(s) if s.column == column && s.descending => None,
        Some(s) if s.column == column => Some(Sort { column, descending: true }),
        _ => Some(Sort { column, descending: false }),
    }
}

/// Returns true when the filter was applied or cleared and the table should reload.
fn filter_panel(ui: &mut egui::Ui, id: TabId, b: &mut Browser) -> bool {
    let mut apply = false;
    let mut remove = None;
    let columns = &b.columns;
    for (i, condition) in b.draft.conditions.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let column_text = condition.column.as_deref().unwrap_or("(any column)");
            egui::ComboBox::from_id_salt((id, "column", i)).width(150.0).selected_text(column_text).show_ui(ui, |ui| {
                ui.selectable_value(&mut condition.column, None, "(any column)");
                for column in columns {
                    ui.selectable_value(&mut condition.column, Some(column.clone()), column);
                }
            });
            egui::ComboBox::from_id_salt((id, "op", i)).width(120.0).selected_text(condition.op.label()).show_ui(ui, |ui| {
                for op in Op::ALL {
                    ui.selectable_value(&mut condition.op, op, op.label());
                }
            });
            let value = egui::TextEdit::singleline(&mut condition.value).desired_width(220.0).hint_text("value");
            let response = ui.add_enabled(condition.op.needs_value(), value);
            apply |= response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if ui.small_button("Remove").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        b.draft.conditions.remove(i);
    }

    let mut clear = false;
    ui.horizontal(|ui| {
        if ui.button("Add condition").clicked() {
            b.draft.conditions.push(Condition::default());
        }
        if b.draft.conditions.len() > 1 {
            ui.label("Match");
            ui.selectable_value(&mut b.draft.join, Join::And, "all");
            ui.selectable_value(&mut b.draft.join, Join::Or, "any");
        }
        apply |= ui.button("Apply").clicked();
        clear = ui.button("Clear").clicked();
    });

    if clear {
        b.draft = blank_draft();
        apply = true;
    }
    if apply {
        b.filter.conditions = b.draft.conditions.clone();
        b.filter.join = b.draft.join;
        b.page = 0;
    }
    apply
}

fn query_ui(ui: &mut egui::Ui, id: TabId, q: &mut QueryTab, databases: &[String], connected: bool, schema: &SchemaView, cmds: &mut Vec<Cmd>) {
    let editor_id = egui::Id::new(("sql_editor", id));
    let editor_focused = ui.memory(|m| m.has_focus(editor_id));
    let run_shortcut = editor_focused && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));
    let mut accept = if editor_focused { navigate_suggestions(ui, q) } else { None };
    egui::Panel::top(egui::Id::new(("sql_panel", id))).resizable(true).default_size(180.0).show(ui, |ui| {
        if let Some(index) = accept.take() {
            apply_suggestion(ui.ctx(), editor_id, q, index);
        }
        let output = egui::TextEdit::multiline(&mut q.sql)
            .id(editor_id)
            .code_editor()
            .desired_width(f32::INFINITY)
            .desired_rows(8)
            .hint_text("SELECT ...")
            .show(ui);
        update_suggestions(ui, editor_id, q, schema, &output);
        if let Some(index) = show_suggestions(ui, id, q, &output) {
            apply_suggestion(ui.ctx(), editor_id, q, index);
            ui.memory_mut(|m| m.request_focus(editor_id));
        }
        ui.horizontal(|ui| {
            let enabled = connected && !q.running;
            if ui.add_enabled(enabled, egui::Button::new("Run  (Ctrl+Enter)")).clicked() || (run_shortcut && enabled) {
                cmds.push(Cmd::RunQuery(id));
            }
            if ui.add_enabled(!q.sql.trim().is_empty(), egui::Button::new("Save")).clicked() {
                cmds.push(Cmd::SaveQuery(id));
            }
            if databases.len() > 1 {
                egui::ComboBox::from_id_salt(("query_database", id)).selected_text(&q.database).show_ui(ui, |ui| {
                    for database in databases {
                        ui.selectable_value(&mut q.database, database.clone(), database);
                    }
                });
            }
            if q.running {
                ui.spinner();
            }
            if let Some(r) = &q.result {
                let cap = if r.truncated { ", truncated" } else { "" };
                ui.label(RichText::new(format!("{} row(s){cap} in {} ms", r.rows.len(), r.elapsed.as_millis())).weak());
            }
        });
    });
    if let Some(result) = &q.result {
        grid::show(ui, result, &GridOptions { sort: None, sortable: false, edit: &EditAccess::Hidden, fk: None, pick: false });
    }
}

/// Consumes the keys that drive the popup before the editor sees them; returns the accepted item.
fn navigate_suggestions(ui: &mut egui::Ui, q: &mut QueryTab) -> Option<usize> {
    let suggest = q.suggest.as_mut()?;
    let count = suggest.items.len();
    let mut accept = None;
    let mut dismiss = false;
    ui.input_mut(|i| {
        let none = egui::Modifiers::NONE;
        if i.consume_key(none, egui::Key::ArrowDown) {
            suggest.selected = (suggest.selected + 1) % count;
        }
        if i.consume_key(none, egui::Key::ArrowUp) {
            suggest.selected = (suggest.selected + count - 1) % count;
        }
        if i.consume_key(none, egui::Key::Tab) || i.consume_key(none, egui::Key::Enter) {
            accept = Some(suggest.selected);
        }
        dismiss = i.consume_key(none, egui::Key::Escape);
    });
    if dismiss {
        q.dismissed = Some((suggest.end, q.sql.chars().count()));
        q.suggest = None;
    }
    accept
}

fn apply_suggestion(ctx: &egui::Context, editor_id: egui::Id, q: &mut QueryTab, index: usize) {
    let Some(suggest) = q.suggest.take() else { return };
    let Some(item) = suggest.items.get(index) else { return };
    let byte = |chars: usize| q.sql.char_indices().nth(chars).map_or(q.sql.len(), |(i, _)| i);
    let (from, to) = (byte(suggest.start), byte(suggest.end));
    q.sql.replace_range(from..to, &item.text);
    let cursor = suggest.start + item.text.chars().count();
    if let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id) {
        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(cursor))));
        state.store(ctx, editor_id);
    }
    q.dismissed = Some((cursor, q.sql.chars().count()));
}

fn update_suggestions(ui: &egui::Ui, editor_id: egui::Id, q: &mut QueryTab, schema: &SchemaView, output: &egui::text_edit::TextEditOutput) {
    let popup_hovered = q.suggest_rect.zip(ui.input(|i| i.pointer.hover_pos())).is_some_and(|(rect, pos)| rect.contains(pos));
    let Some(range) = output.cursor_range.filter(|_| output.response.has_focus()) else {
        if !popup_hovered && !ui.memory(|m| m.has_focus(editor_id)) {
            q.suggest = None;
        }
        return;
    };
    let cursor: usize = range.primary.index.into();
    if q.dismissed == Some((cursor, q.sql.chars().count())) {
        q.suggest = None;
        return;
    }
    q.dismissed = None;
    let database = if q.database.is_empty() { schema.databases.first() } else { Some(&q.database) };
    let tables: Vec<String> = database
        .and_then(|d| schema.tables.get(d))
        .map(|tables| {
            let mut names: Vec<String> = tables.iter().map(|t| t.name.clone()).collect();
            names.dedup();
            names
        })
        .unwrap_or_default();
    let empty = HashMap::new();
    let columns = database.and_then(|d| schema.columns.get(d)).unwrap_or(&empty);
    let found = complete(&q.sql, cursor, &Schema { engine: schema.engine, databases: schema.databases, tables: &tables, columns });
    q.suggest = found.map(|Completion { start, items }| {
        let selected = q.suggest.as_ref().filter(|s| s.start == start).map_or(0, |s| s.selected.min(items.len() - 1));
        Suggest { start, end: cursor, items, selected }
    });
}

/// Draws the popup under the cursor; returns the item clicked.
fn show_suggestions(ui: &egui::Ui, id: TabId, q: &mut QueryTab, output: &egui::text_edit::TextEditOutput) -> Option<usize> {
    let Some(suggest) = &q.suggest else {
        q.suggest_rect = None;
        return None;
    };
    let caret = output.galley.pos_from_cursor(egui::text::CCursor::new(suggest.end)).left_bottom();
    let pos = output.galley_pos + caret.to_vec2() + egui::vec2(0.0, 2.0);
    let mut clicked = None;
    let area = egui::Area::new(egui::Id::new(("suggestions", id))).order(egui::Order::Foreground).fixed_pos(pos).show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_min_width(260.0);
            for (i, item) in suggest.items.iter().enumerate() {
                let button = egui::Button::new(&item.text).selected(i == suggest.selected).shortcut_text(RichText::new(item.kind.label()).weak().small());
                if ui.add(button).clicked() {
                    clicked = Some(i);
                }
            }
        });
    });
    q.suggest_rect = Some(area.response.rect);
    clicked
}
