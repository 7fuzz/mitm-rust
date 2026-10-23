use std::collections::HashMap;

use dbviewer_core::{SavedQuery, TableKind, TableRef};
use egui::containers::menu::MenuButton;
use egui::{Align, Layout, RichText};

use mitm_core::credentials;

use super::{App, Connection};
use crate::ui::icons;
use crate::ui::theme::{color, Tone};
use crate::widgets::search::search_field;

const ROW_HEIGHT: f32 = 22.0;
const MENU_WIDTH: f32 = 200.0;
const RECENT_LIMIT: usize = 6;

pub type RowCounts = HashMap<String, HashMap<String, i64>>;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarTab {
    #[default]
    Tables,
    Queries,
}

enum TableAction {
    Open(TableRef),
    Query(TableRef),
}

impl App {
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        self.connection_header(ui);
        if self.credentials_locked() {
            ui.label(RichText::new("Connections are stored in credentials, which are locked.").weak());
            self.unlock_form(ui);
        }
        if self.session().is_none() {
            return;
        }
        ui.add_space(4.0);
        let hint = match self.sidebar_tab {
            SidebarTab::Tables => "Filter tables",
            SidebarTab::Queries => "Filter queries",
        };
        search_field(ui, &mut self.filter, hint);
        ui.add_space(2.0);
        self.sidebar_tabs(ui);
        ui.separator();
        match self.sidebar_tab {
            SidebarTab::Tables => self.table_panel(ui),
            SidebarTab::Queries => self.query_panel(ui),
        }
    }

    pub(super) fn request_connect(&mut self, ctx: &egui::Context, index: usize) {
        let profile = &self.profiles[index];
        let ask_db = profile.password.is_empty();
        let ask_ssh = profile.ssh.as_ref().is_some_and(|s| s.secret.is_empty());
        if ask_db || ask_ssh {
            self.prompt = Some(super::PasswordPrompt { index, password: String::new(), ssh_secret: String::new(), ask_db, ask_ssh, focused: false, reveal: false });
        } else {
            self.connect(ctx, index, String::new(), String::new());
        }
    }

    pub(super) fn disconnect(&mut self) {
        self.connection = Connection::Disconnected;
        self.reset_databases();
        self.close_table_tabs();
    }

    fn unlock_form(&mut self, ui: &mut egui::Ui) {
        let mut submit = false;
        ui.horizontal(|ui| {
            let edit = ui.add(egui::TextEdit::singleline(&mut self.unlock_password).password(true).hint_text("master password").desired_width(ui.available_width() - 70.0));
            submit = ui.button("Unlock").clicked() || (edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        });
        if submit {
            match credentials::unlock(&self.db, &self.unlock_password) {
                Ok(()) => {
                    self.unlock_password.clear();
                    self.unlock_error = None;
                    self.unlocked = true;
                    self.reload_profiles();
                }
                Err(e) => self.unlock_error = Some(e.to_string()),
            }
        }
        if let Some(error) = &self.unlock_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }

    fn active_profile_name(&self) -> Option<&str> {
        match &self.connection {
            Connection::Disconnected => None,
            Connection::Connecting(name) | Connection::Connected { name, .. } => Some(name),
        }
    }

    fn connection_header(&mut self, ui: &mut egui::Ui) {
        let failed = self.error.is_some() && self.last_connect.is_some() && matches!(self.connection, Connection::Disconnected);
        let (tone, title) = match &self.connection {
            Connection::Disconnected if failed => (Tone::Danger, RichText::new("Connection failed").strong()),
            Connection::Disconnected => (Tone::Neutral, RichText::new("Not connected").weak()),
            Connection::Connecting(name) => (Tone::Warn, RichText::new(format!("Connecting to {name}…")).strong()),
            Connection::Connected { name, .. } => (Tone::Success, RichText::new(name).strong()),
        };
        let button = egui::Button::new((RichText::new("●").color(color(ui, tone)), title, RichText::new(icons::CARET_DOWN).weak())).frame_when_inactive(false);

        let active = self.active_profile_name().map(str::to_string);
        let connected = matches!(self.connection, Connection::Connected { .. });
        let (mut connect, mut edit, mut delete, mut new, mut disconnect) = (None, None, None, false, false);
        let profiles = &self.profiles;
        MenuButton::from_button(button).ui(ui, |ui| {
            ui.set_width(MENU_WIDTH);
            if profiles.is_empty() {
                ui.label(RichText::new("No connections yet").weak());
            }
            for (i, profile) in profiles.iter().enumerate() {
                let label = if profile.ssh.is_some() { format!("{} (ssh)", profile.name) } else { profile.name.clone() };
                let is_active = active.as_deref() == Some(profile.name.as_str());
                let (rect, row) = ui.allocate_exact_size(egui::vec2(MENU_WIDTH, ROW_HEIGHT), egui::Sense::click());
                let actions = egui::Rect::from_min_max(egui::pos2(rect.right() - 48.0, rect.top()), rect.right_bottom());
                let mut buttons = ui.new_child(egui::UiBuilder::new().max_rect(actions).layout(Layout::right_to_left(Align::Center)));
                if icon_button(&mut buttons, icons::TRASH, "Delete").clicked() {
                    delete = Some(i);
                    ui.close();
                }
                if icon_button(&mut buttons, icons::PENCIL_SIMPLE, "Edit").clicked() {
                    edit = Some(i);
                    ui.close();
                }
                if row.hovered() || is_active {
                    let fill = if is_active { ui.visuals().selection.bg_fill.gamma_multiply(0.5) } else { ui.visuals().widgets.hovered.weak_bg_fill };
                    ui.painter().rect_filled(rect, 4, fill);
                }
                let text_rect = egui::Rect::from_min_max(egui::pos2(rect.left() + 8.0, rect.top()), egui::pos2(actions.left(), rect.bottom()));
                ui.painter().with_clip_rect(text_rect).text(
                    egui::pos2(text_rect.left(), rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::TextStyle::Body.resolve(ui.style()),
                    ui.visuals().text_color(),
                );
                if row.clicked() && connect.is_none() && edit.is_none() && delete.is_none() {
                    connect = Some(i);
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("New connection").clicked() {
                new = true;
                ui.close();
            }
            if connected && ui.button("Disconnect").clicked() {
                disconnect = true;
                ui.close();
            }
        });

        if let Some(profile) = self.connected_profile() {
            let mut parts = vec![profile.engine.label().to_string()];
            if profile.ssh.is_some() {
                parts.push("ssh".into());
            }
            if profile.read_only {
                parts.push("read-only".into());
            }
            ui.label(RichText::new(parts.join(" · ")).weak().small());
        }

        if let Some(i) = connect {
            self.request_connect(ui.ctx(), i);
        }
        if let Some(i) = edit {
            self.form = Some(super::ProfileForm::new(Some(i), self.profiles[i].clone()));
        }
        if let Some(i) = delete {
            if let Err(e) = super::store::delete_profile(&self.db, &self.profiles[i].name) {
                self.error = Some(e);
            }
            self.reload_profiles();
        }
        if new {
            self.form = Some(super::ProfileForm::new(None, dbviewer_core::Profile::default()));
        }
        if disconnect {
            self.disconnect();
        }
    }

    fn connected_profile(&self) -> Option<&dbviewer_core::Profile> {
        match &self.connection {
            Connection::Connected { name, .. } => self.profiles.iter().find(|p| p.name == *name),
            _ => None,
        }
    }

    fn connection_saved_queries(&self) -> Vec<SavedQuery> {
        let Some(connection) = self.connection_name() else { return Vec::new() };
        self.saved.iter().filter(|q| q.connection == connection).cloned().collect()
    }

    fn sidebar_tabs(&mut self, ui: &mut egui::Ui) {
        let saved = self.connection_saved_queries().len();
        let mut new_query = false;
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.sidebar_tab, SidebarTab::Tables, "Tables");
            ui.selectable_value(&mut self.sidebar_tab, SidebarTab::Queries, format!("Queries ({saved})"));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                new_query = icon_button(ui, icons::PLUS, "New query").clicked();
            });
        });
        if new_query {
            self.open_query(None, None);
        }
    }

    fn query_panel(&mut self, ui: &mut egui::Ui) {
        let filter = self.filter.to_lowercase();
        let mine: Vec<SavedQuery> = self.connection_saved_queries().into_iter().filter(|q| q.name.to_lowercase().contains(&filter)).collect();
        if mine.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new(if self.filter.is_empty() { "No saved queries. Use Save on a query tab." } else { "No matching queries" }).weak().small());
            return;
        }
        let (mut open, mut delete) = (None, None);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for query in &mine {
                let response = compact_row(ui, &query.name, "", false, false).on_hover_text(&query.sql);
                if response.clicked() {
                    open = Some(query.clone());
                }
                response.context_menu(|ui| {
                    if ui.button("Delete").clicked() {
                        delete = Some(query.clone());
                        ui.close();
                    }
                });
            }
        });
        if let Some(query) = open {
            self.open_saved(&query);
        }
        if let Some(query) = delete {
            self.saved.retain(|q| *q != query);
            self.persist_saved();
        }
    }

    fn table_panel(&mut self, ui: &mut egui::Ui) {
        let filter = self.filter.trim().to_lowercase();
        let multiple = self.databases.len() > 1;
        let focused: Option<TableRef> = self.focused_table();
        let open_tables = self.open_tables();
        let (mut action, mut expand, mut query_database) = (None, None, None);
        let mut wanted_counts = Vec::new();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for database in &self.databases {
                let mut list = |ui: &mut egui::Ui| match self.tables.get(database) {
                    Some(tables) => {
                        wanted_counts.push(database.clone());
                        let counts = self.row_counts.get(database);
                        table_list(ui, tables, &filter, counts, &open_tables, focused.as_ref(), &mut action);
                    }
                    None => {
                        ui.spinner();
                    }
                };
                if multiple {
                    let header = egui::CollapsingHeader::new(database).id_salt(("db", database)).show(ui, |ui| {
                        expand = Some(database.clone());
                        list(ui);
                    });
                    header.header_response.context_menu(|ui| {
                        if ui.button("Query this database").clicked() {
                            query_database = Some(database.clone());
                            ui.close();
                        }
                    });
                } else {
                    list(ui);
                }
            }
        });

        for database in wanted_counts {
            self.load_row_counts(ui.ctx(), database);
        }
        if let Some(database) = expand {
            self.load_tables(ui.ctx(), database);
        }
        match action {
            Some(TableAction::Open(table)) => self.open_table(ui.ctx(), table),
            Some(TableAction::Query(table)) => self.open_table_query(&table),
            None => {}
        }
        if let Some(database) = query_database {
            self.open_query(Some(database), None);
        }
    }

    fn focused_table(&mut self) -> Option<TableRef> {
        let id = self.dock.find_active_focused().map(|(_, id)| *id)?;
        match self.contents.get(&id)? {
            super::Content::Table(b) => Some(b.table.clone()),
            super::Content::Query(_) => None,
        }
    }

    fn open_tables(&self) -> Vec<TableRef> {
        self.contents
            .values()
            .filter_map(|c| match c {
                super::Content::Table(b) => Some(b.table.clone()),
                super::Content::Query(_) => None,
            })
            .collect()
    }

    pub(super) fn remember_recent(&mut self, table: &TableRef) {
        self.recent.retain(|t| t != table);
        self.recent.insert(0, table.clone());
        self.recent.truncate(RECENT_LIMIT);
    }

    fn open_table_query(&mut self, table: &TableRef) {
        self.open_query(Some(table.database.clone()), None);
        if let Some(super::Content::Query(q)) = self.contents.get_mut(&self.next_tab) {
            q.sql = format!("SELECT * FROM {} LIMIT 100", table.qualified());
        }
    }

    pub(super) fn empty_state(&mut self, ui: &mut egui::Ui) {
        let width = 360.0_f32.min(ui.available_width());
        ui.add_space((ui.available_height() * 0.2).max(16.0));
        let mut open = None;
        let mut open_saved = None;
        let mut connect = None;
        let mut new_query = false;
        let mut unlock = false;
        let locked = self.credentials_locked();
        let saved = self.connection_saved_queries();
        ui.horizontal(|ui| {
            ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
            ui.vertical(|ui| {
                ui.set_width(width);
                match &self.connection {
                    Connection::Disconnected if locked => {
                        ui.heading("Credentials locked");
                        ui.label(RichText::new("Connections are stored in credentials. Enter the master password to use them.").weak());
                        ui.add_space(4.0);
                        unlock = true;
                    }
                    Connection::Disconnected => {
                        ui.heading("Not connected");
                        if self.profiles.is_empty() {
                            ui.label(RichText::new("Add a connection from the sidebar to get started.").weak());
                        } else {
                            ui.label(RichText::new("Pick a connection").weak());
                            ui.add_space(4.0);
                            for (i, profile) in self.profiles.iter().enumerate() {
                                if ui.button(&profile.name).clicked() {
                                    connect = Some(i);
                                }
                            }
                        }
                    }
                    Connection::Connecting(name) => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(format!("Connecting to {name}…"));
                        });
                    }
                    Connection::Connected { name, .. } => {
                        ui.heading(name);
                        let tables: usize = self.tables.values().map(Vec::len).sum();
                        let engine = self.connected_profile().map(|p| p.engine.label()).unwrap_or_default();
                        ui.label(RichText::new(format!("{engine} · {} databases · {tables} tables loaded", self.databases.len())).weak());
                        ui.add_space(6.0);
                        new_query = ui.button("New query").clicked();
                        if !self.recent.is_empty() {
                            ui.add_space(10.0);
                            ui.label(RichText::new("Recent tables").weak().small());
                            for table in &self.recent {
                                if ui.link(&table.name).clicked() {
                                    open = Some(table.clone());
                                }
                            }
                        }
                        if !saved.is_empty() {
                            ui.add_space(10.0);
                            ui.label(RichText::new("Saved queries").weak().small());
                            for query in saved.iter().take(5) {
                                if ui.link(&query.name).on_hover_text(&query.sql).clicked() {
                                    open_saved = Some(query.clone());
                                }
                            }
                        }
                        if self.recent.is_empty() && saved.is_empty() {
                            ui.add_space(10.0);
                            ui.label(RichText::new("Pick a table from the sidebar, or open a new query.").weak());
                        }
                    }
                }
            });
        });
        if unlock {
            ui.horizontal(|ui| {
                ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
                ui.vertical(|ui| {
                    ui.set_width(width);
                    self.unlock_form(ui);
                });
            });
        }
        if let Some(i) = connect {
            self.request_connect(ui.ctx(), i);
        }
        if let Some(table) = open {
            self.open_table(ui.ctx(), table);
        }
        if let Some(query) = open_saved {
            self.open_saved(&query);
        }
        if new_query {
            self.open_query(None, None);
        }
    }
}

fn table_list(
    ui: &mut egui::Ui,
    tables: &[TableRef],
    filter: &str,
    counts: Option<&HashMap<String, HashMap<String, i64>>>,
    open_tables: &[TableRef],
    focused: Option<&TableRef>,
    action: &mut Option<TableAction>,
) {
    let matching: Vec<&TableRef> = tables.iter().filter(|t| t.name.to_lowercase().contains(filter)).collect();
    if matching.is_empty() {
        ui.label(RichText::new(if filter.is_empty() { "No tables" } else { "No matching tables" }).weak().small());
        return;
    }
    let grouped = matching.first().is_some_and(|first| matching.iter().any(|t| t.schema != first.schema));
    if !grouped {
        for table in matching {
            table_row(ui, table, counts, open_tables, focused, action);
        }
        return;
    }
    for group in matching.chunk_by(|a, b| a.schema == b.schema) {
        let schema = &group[0].schema;
        let header = RichText::new(format!("{schema}  {}", group.len())).small();
        let mut section = egui::CollapsingHeader::new(header).id_salt(("schema", schema)).default_open(schema == "public");
        if !filter.is_empty() {
            section = section.open(Some(true));
        }
        section.show(ui, |ui| {
            for table in group {
                table_row(ui, table, counts, open_tables, focused, action);
            }
        });
    }
}

fn table_row(
    ui: &mut egui::Ui,
    table: &TableRef,
    counts: Option<&HashMap<String, HashMap<String, i64>>>,
    open_tables: &[TableRef],
    focused: Option<&TableRef>,
    action: &mut Option<TableAction>,
) {
    let meta = match table.kind {
        TableKind::Table => counts
            .and_then(|c| c.get(&table.schema))
            .and_then(|c| c.get(&table.name))
            .filter(|n| **n > 0)
            .map(|n| format_count(*n))
            .unwrap_or_default(),
        TableKind::View => "view".to_string(),
        TableKind::MaterializedView => "matview".to_string(),
        TableKind::Foreign => "foreign".to_string(),
    };
    let response = compact_row(ui, &table.name, &meta, focused == Some(table), open_tables.contains(table));
    if response.clicked() {
        *action = Some(TableAction::Open(table.clone()));
    }
    response.context_menu(|ui| {
        if ui.button("Open").clicked() {
            *action = Some(TableAction::Open(table.clone()));
            ui.close();
        }
        if ui.button("Query this table").clicked() {
            *action = Some(TableAction::Query(table.clone()));
            ui.close();
        }
        if ui.button("Copy name").clicked() {
            ui.ctx().copy_text(table.name.clone());
            ui.close();
        }
        if ui.button("Copy SELECT").clicked() {
            ui.ctx().copy_text(format!("SELECT * FROM {}", table.qualified()));
            ui.close();
        }
    });
}

/// A dense list row: name on the left, faint meta on the right, an accent dot while the item is open in a tab.
fn compact_row(ui: &mut egui::Ui, name: &str, meta: &str, selected: bool, open: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let visuals = ui.visuals().clone();
    let accent = visuals.selection.stroke.color;
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 4, accent.gamma_multiply(0.18));
        painter.vline(rect.left() + 1.0, rect.y_range().shrink(4.0), egui::Stroke::new(2.0, accent));
    } else if response.hovered() {
        painter.rect_filled(rect, 4, visuals.widgets.hovered.weak_bg_fill);
    }
    let font = egui::TextStyle::Body.resolve(ui.style());
    let small = egui::TextStyle::Small.resolve(ui.style());
    let right = rect.right() - 8.0;
    let meta_width = if meta.is_empty() { 0.0 } else { painter.layout_no_wrap(meta.to_string(), small.clone(), visuals.weak_text_color()).size().x + 8.0 };
    if !meta.is_empty() {
        painter.text(egui::pos2(right, rect.center().y), egui::Align2::RIGHT_CENTER, meta, small, visuals.weak_text_color());
    }
    if open {
        painter.circle_filled(egui::pos2(rect.left() + 8.0, rect.center().y), 2.5, accent);
    }
    let text_rect = egui::Rect::from_min_max(egui::pos2(rect.left() + 18.0, rect.top()), egui::pos2(right - meta_width, rect.bottom()));
    painter
        .with_clip_rect(text_rect)
        .text(egui::pos2(text_rect.left(), rect.center().y), egui::Align2::LEFT_CENTER, name, font, visuals.text_color());
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn icon_button(ui: &mut egui::Ui, icon: &str, tooltip: &str) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(icon).weak()).frame(false)).on_hover_text(tooltip)
}

fn format_count(n: i64) -> String {
    match n {
        n if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1_000_000.0),
        n if n >= 10_000 => format!("{}k", n / 1_000),
        n if n >= 1_000 => format!("{:.1}k", n as f64 / 1_000.0),
        n => n.to_string(),
    }
}
