mod edit_dialog;
mod fk;
mod grid;
mod tabs;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crossbeam_channel::{unbounded, Receiver, Sender};
use dbviewer_core::{upsert, Engine, ForeignKey, SavedQuery, SavedQueryStore, Profile, ProfileStore, QueryResult, Session, SshConfig, TableKind, TableRef};
use egui::RichText;
use egui_dock::{DockArea, DockState};

use self::edit_dialog::{EditDialog, Outcome, Request, PICKER_LIMIT};
use self::tabs::{Browser, Cmd, Content, QueryTab, SchemaView, TabId, Viewer};

enum Event {
    Connected(Result<(Arc<Session>, Vec<String>), String>),
    Tables { database: String, result: Result<Vec<TableRef>, String> },
    Table { tab: TabId, generation: u64, result: Result<QueryResult, String> },
    Query { tab: TabId, result: Result<QueryResult, String> },
    PrimaryKey { tab: TabId, result: Result<Vec<String>, String> },
    Updated { tab: TabId, result: Result<(), String> },
    TableKeys { tab: TabId, result: Result<Vec<ForeignKey>, String> },
    PickerKeys { tab: TabId, result: Result<Vec<ForeignKey>, String> },
    TableLabel { tab: TabId, column: String, value: String, result: Result<QueryResult, String> },
    PickerLabel { tab: TabId, column: String, value: String, result: Result<QueryResult, String> },
    Columns { database: String, result: Result<HashMap<String, Vec<String>>, String> },
    Enums { tab: TabId, result: Result<HashMap<String, Vec<String>>, String> },
    Lookup { tab: TabId, col: usize, generation: u64, result: Result<QueryResult, String> },
    Label { tab: TabId, col: usize, value: String, result: Result<QueryResult, String> },
}

enum Connection {
    Disconnected,
    Connecting(String),
    Connected { name: String, session: Arc<Session> },
}

struct ProfileForm {
    index: Option<usize>,
    profile: Profile,
    use_ssh: bool,
    ssh: SshConfig,
    ssh_port: String,
    show_password: bool,
    show_ssh_secret: bool,
}

struct SaveDialog {
    tab: TabId,
    name: String,
    focused: bool,
}

struct PasswordPrompt {
    index: usize,
    password: String,
    ssh_secret: String,
    ask_db: bool,
    ask_ssh: bool,
    focused: bool,
    reveal: bool,
}

#[derive(Default)]
pub struct DbViewerState(Option<App>);

pub fn show(ui: &mut egui::Ui, state: &mut DbViewerState) {
    state.0.get_or_insert_with(App::new).show(ui);
}

struct App {
    rt: tokio::runtime::Runtime,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    store: ProfileStore,
    profiles: Vec<Profile>,
    saved_store: SavedQueryStore,
    saved: Vec<SavedQuery>,
    save_dialog: Option<SaveDialog>,
    form: Option<ProfileForm>,
    prompt: Option<PasswordPrompt>,
    connection: Connection,
    databases: Vec<String>,
    tables: HashMap<String, Vec<TableRef>>,
    loading_databases: HashSet<String>,
    columns: HashMap<String, HashMap<String, Vec<String>>>,
    loading_columns: HashSet<String>,
    filter: String,
    edit: Option<EditDialog>,
    dock: DockState<TabId>,
    contents: HashMap<TabId, Content>,
    next_tab: TabId,
    next_query_number: usize,
    error: Option<String>,
    last_connect: Option<(usize, String, String)>,
    reload_tabs_on_connect: bool,
}

impl App {
    fn new() -> Self {
        let (tx, rx) = unbounded();
        let store = ProfileStore::default_location();
        let (profiles, error) = match store.load() {
            Ok(profiles) => (profiles, None),
            Err(e) => (Vec::new(), Some(format!("{e:#}"))),
        };
        let saved_store = SavedQueryStore::default_location();
        let (saved, saved_error) = match saved_store.load() {
            Ok(saved) => (saved, None),
            Err(e) => (Vec::new(), Some(format!("{e:#}"))),
        };
        let error = error.or(saved_error);
        Self {
            rt: tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("tokio runtime"),
            tx,
            rx,
            store,
            profiles,
            saved_store,
            saved,
            save_dialog: None,
            form: None,
            prompt: None,
            connection: Connection::Disconnected,
            databases: Vec::new(),
            tables: HashMap::new(),
            loading_databases: HashSet::new(),
            columns: HashMap::new(),
            loading_columns: HashSet::new(),
            filter: String::new(),
            edit: None,
            dock: DockState::new(Vec::new()),
            contents: HashMap::new(),
            next_tab: 0,
            next_query_number: 1,
            error,
            last_connect: None,
            reload_tabs_on_connect: false,
        }
    }

    fn session(&self) -> Option<Arc<Session>> {
        match &self.connection {
            Connection::Connected { session, .. } => Some(Arc::clone(session)),
            _ => None,
        }
    }

    fn connect(&mut self, ctx: &egui::Context, index: usize, password: String, ssh_secret: String) {
        self.start_connect(ctx, index, password, ssh_secret, false);
    }

    /// Reconnects with the credentials of the last attempt, keeping tabs open and reloading them.
    fn retry(&mut self, ctx: &egui::Context) {
        if let Some((index, password, ssh_secret)) = self.last_connect.clone() {
            self.start_connect(ctx, index, password, ssh_secret, true);
        }
    }

    fn start_connect(&mut self, ctx: &egui::Context, index: usize, password: String, ssh_secret: String, keep_tabs: bool) {
        self.last_connect = Some((index, password.clone(), ssh_secret.clone()));
        self.reload_tabs_on_connect = keep_tabs;
        let mut profile = self.profiles[index].clone();
        if !password.is_empty() {
            profile.password = password;
        }
        if let Some(ssh) = profile.ssh.as_mut().filter(|_| !ssh_secret.is_empty()) {
            ssh.secret = ssh_secret;
        }
        self.connection = Connection::Connecting(profile.name.clone());
        self.reset_databases();
        if !keep_tabs {
            self.close_table_tabs();
        }
        self.error = None;
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        self.rt.spawn(async move {
            let result = async {
                let session = Session::connect(&profile, &profile.password).await?;
                let databases = session.databases().await?;
                Ok::<_, anyhow::Error>((Arc::new(session), databases))
            }
            .await
            .map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::Connected(result));
            ctx.request_repaint();
        });
    }

    fn reset_databases(&mut self) {
        self.databases.clear();
        self.tables.clear();
        self.loading_databases.clear();
        self.columns.clear();
        self.loading_columns.clear();
    }

    fn close_table_tabs(&mut self) {
        self.edit = None;
        self.contents.retain(|_, content| matches!(content, Content::Query(_)));
        let contents = &self.contents;
        self.dock.retain_tabs(|id| contents.contains_key(id));
    }

    fn load_tables(&mut self, ctx: &egui::Context, database: String) {
        let Some(session) = self.session() else { return };
        if self.tables.contains_key(&database) || !self.loading_databases.insert(database.clone()) {
            return;
        }
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        self.rt.spawn(async move {
            let result = session.tables(&database).await.map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::Tables { database, result });
            ctx.request_repaint();
        });
    }

    fn load_columns(&mut self, ctx: &egui::Context, database: String) {
        let Some(session) = self.session() else { return };
        if self.columns.contains_key(&database) || !self.loading_columns.insert(database.clone()) {
            return;
        }
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        self.rt.spawn(async move {
            let result = session.columns(&database).await.map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::Columns { database, result });
            ctx.request_repaint();
        });
    }

    /// Autocomplete needs the tables and columns of every database a query tab points at.
    fn ensure_schema(&mut self, ctx: &egui::Context) {
        let wanted: HashSet<String> = self
            .contents
            .values()
            .filter_map(|c| match c {
                Content::Query(q) => Some(if q.database.is_empty() { self.databases.first().cloned().unwrap_or_default() } else { q.database.clone() }),
                Content::Table(_) => None,
            })
            .filter(|d| !d.is_empty())
            .collect();
        for database in wanted {
            self.load_tables(ctx, database.clone());
            self.load_columns(ctx, database);
        }
    }

    fn new_tab_id(&mut self) -> TabId {
        self.next_tab += 1;
        self.next_tab
    }

    fn open_table(&mut self, ctx: &egui::Context, table: TableRef) {
        let existing = self
            .contents
            .iter()
            .find(|(_, c)| matches!(c, Content::Table(b) if b.table == table))
            .map(|(id, _)| *id);
        if let Some(path) = existing.and_then(|id| self.dock.find_tab(&id)) {
            let _ = self.dock.set_active_tab(path);
            self.dock.set_focused_node_and_surface(path.node_path());
            return;
        }
        let id = self.new_tab_id();
        self.contents.insert(id, Content::Table(Browser::new(table.clone())));
        self.dock.push_to_focused_leaf(id);
        self.load_table(ctx, id);
        self.load_primary_key(ctx, id, table.clone());
        self.load_foreign_keys(ctx, id, table);
    }

    fn open_query(&mut self, database: Option<String>, at: Option<egui_dock::NodePath>) {
        let id = self.new_tab_id();
        let number = self.next_query_number;
        self.next_query_number += 1;
        let database = database.or_else(|| self.databases.first().cloned()).unwrap_or_default();
        self.contents.insert(id, Content::Query(QueryTab::new(number, database)));
        if let Some(path) = at {
            self.dock.set_focused_node_and_surface(path);
        }
        self.dock.push_to_focused_leaf(id);
    }

    fn load_table(&mut self, ctx: &egui::Context, tab: TabId) {
        let Some(session) = self.session() else { return };
        let Some(Content::Table(browser)) = self.contents.get_mut(&tab) else { return };
        browser.loading = true;
        browser.generation += 1;
        let (table, page, page_size, filter, columns, generation) =
            (browser.table.clone(), browser.page, browser.page_size, browser.filter.clone(), browser.columns.clone(), browser.generation);
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        self.rt.spawn(async move {
            let result = session.browse(&table, page_size, page * page_size, &filter, &columns).await.map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::Table { tab, generation, result });
            ctx.request_repaint();
        });
    }

    fn load_primary_key(&mut self, ctx: &egui::Context, tab: TabId, table: TableRef) {
        let Some(session) = self.session() else { return };
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        self.rt.spawn(async move {
            let result = session.primary_key(&table).await.map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::PrimaryKey { tab, result });
            ctx.request_repaint();
        });
    }

    fn open_edit_dialog(&mut self, ctx: &egui::Context, tab: TabId, row: usize) {
        let Some(Content::Table(browser)) = self.contents.get(&tab) else { return };
        let (Some(result), Some(key)) = (&browser.result, &browser.primary_key) else { return };
        let Some(values) = result.rows.get(row) else { return };
        let mut dialog = EditDialog::new(tab, browser.table.clone(), result.columns.clone(), values.clone(), key.clone());
        dialog.set_foreign_keys(browser.fk.keys.clone());
        let table = browser.table.clone();
        self.edit = Some(dialog);
        let Some(session) = self.session() else { return };
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        self.rt.spawn(async move {
            let result = session.enum_options(&table).await.map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::Enums { tab, result });
            ctx.request_repaint();
        });
    }

    fn load_foreign_keys(&mut self, ctx: &egui::Context, tab: TabId, table: TableRef) {
        let Some(session) = self.session() else { return };
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        self.rt.spawn(async move {
            let result = session.foreign_keys(&table).await.map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::TableKeys { tab, result });
            ctx.request_repaint();
        });
    }

    fn load_fk_label(&mut self, ctx: &egui::Context, tab: TabId, column: String, value: String) {
        let Some(session) = self.session() else { return };
        let Some(Content::Table(browser)) = self.contents.get(&tab) else { return };
        let Some(fk) = browser.fk.key_for(&column).cloned() else { return };
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        self.rt.spawn(async move {
            let result = session.lookup_value(&fk, &value).await.map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::TableLabel { tab, column, value, result });
            ctx.request_repaint();
        });
    }

    fn spawn_requests(&mut self, ctx: &egui::Context, tab: TabId, requests: Vec<Request>) {
        let Some(session) = self.session() else { return };
        for request in requests {
            let (session, tx, ctx) = (Arc::clone(&session), self.tx.clone(), ctx.clone());
            self.rt.spawn(async move {
                let event = match request {
                    Request::Lookup { col, generation, fk, search } => {
                        let result = session.lookup(&fk, &search, PICKER_LIMIT).await.map_err(|e| format!("{e:#}"));
                        Event::Lookup { tab, col, generation, result }
                    }
                    Request::Label { col, fk, value } => {
                        let result = session.lookup_value(&fk, &value).await.map_err(|e| format!("{e:#}"));
                        Event::Label { tab, col, value, result }
                    }
                    Request::PickerKeys { table } => {
                        let result = session.foreign_keys(&table).await.map_err(|e| format!("{e:#}"));
                        Event::PickerKeys { tab, result }
                    }
                    Request::PickerLabel { column, value, fk } => {
                        let result = session.lookup_value(&fk, &value).await.map_err(|e| format!("{e:#}"));
                        Event::PickerLabel { tab, column, value, result }
                    }
                };
                let _ = tx.send(event);
                ctx.request_repaint();
            });
        }
    }

    fn save_edit(&mut self, ctx: &egui::Context, changes: Vec<dbviewer_core::Assignment>) {
        let (Some(session), Some(dialog)) = (self.session(), self.edit.as_mut()) else { return };
        let key = match dialog.key() {
            Ok(key) => key,
            Err(e) => {
                dialog.error = Some(e);
                return;
            }
        };
        dialog.saving = true;
        dialog.error = None;
        let (tab, table) = (dialog.tab, dialog.table.clone());
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        self.rt.spawn(async move {
            let result = session.update_row(&table, &changes, &key).await.map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::Updated { tab, result });
            ctx.request_repaint();
        });
    }

    fn run_query(&mut self, ctx: &egui::Context, tab: TabId) {
        let Some(session) = self.session() else { return };
        let Some(Content::Query(query)) = self.contents.get_mut(&tab) else { return };
        query.running = true;
        self.error = None;
        let (tx, ctx, sql, database) = (self.tx.clone(), ctx.clone(), query.sql.clone(), query.database.clone());
        self.rt.spawn(async move {
            let result = session.query(&database, &sql).await.map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::Query { tab, result });
            ctx.request_repaint();
        });
    }

    fn handle_commands(&mut self, ctx: &egui::Context, cmds: Vec<Cmd>) {
        for cmd in cmds {
            match cmd {
                Cmd::LoadTable(id) => self.load_table(ctx, id),
                Cmd::RunQuery(id) => self.run_query(ctx, id),
                Cmd::FkLabel { tab, column, value } => self.load_fk_label(ctx, tab, column, value),
                Cmd::SaveQuery(id) => self.save_dialog = Some(SaveDialog { tab: id, name: String::new(), focused: false }),
                Cmd::EditRow { tab, row } => self.open_edit_dialog(ctx, tab, row),
                Cmd::NewQuery(path) => self.open_query(None, Some(path)),
                Cmd::Closed(id) => {
                    self.contents.remove(&id);
                }
            }
        }
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
        let mut pending_load = None;
        let mut reload = None;
        let mut reload_all = false;
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Connected(Ok((session, databases))) => {
                    let name = match std::mem::replace(&mut self.connection, Connection::Disconnected) {
                        Connection::Connecting(name) => name,
                        _ => String::new(),
                    };
                    self.connection = Connection::Connected { name, session };
                    for content in self.contents.values_mut() {
                        if let Content::Query(q) = content {
                            if !databases.contains(&q.database) {
                                q.database = databases.first().cloned().unwrap_or_default();
                            }
                        }
                    }
                    self.databases = databases;
                    pending_load = (self.databases.len() == 1).then(|| self.databases[0].clone());
                    if std::mem::take(&mut self.reload_tabs_on_connect) {
                        reload_all = true;
                    }
                }
                Event::Connected(Err(e)) => {
                    self.connection = Connection::Disconnected;
                    self.error = Some(e);
                }
                Event::Tables { database, result } => {
                    self.loading_databases.remove(&database);
                    match result {
                        Ok(tables) => {
                            self.tables.insert(database, tables);
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
                Event::Table { tab, generation, result } => {
                    let Some(Content::Table(browser)) = self.contents.get_mut(&tab).filter(|c| matches!(c, Content::Table(b) if b.generation == generation)) else {
                        continue;
                    };
                    browser.loading = false;
                    match result {
                        Ok(r) => {
                            if !r.columns.is_empty() {
                                browser.columns = r.columns.clone();
                            }
                            browser.result = Some(r);
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
                Event::PrimaryKey { tab, result } => {
                    let Some(Content::Table(browser)) = self.contents.get_mut(&tab) else { continue };
                    match result {
                        Ok(key) => browser.primary_key = Some(key),
                        Err(e) => {
                            browser.primary_key = Some(Vec::new());
                            self.error = Some(e);
                        }
                    }
                }
                Event::Updated { tab, result } => match result {
                    Ok(()) => {
                        self.edit = None;
                        reload = Some(tab);
                    }
                    Err(e) => {
                        if let Some(dialog) = self.edit.as_mut() {
                            dialog.saving = false;
                            dialog.error = Some(e);
                        }
                    }
                },
                Event::TableKeys { tab, result } => {
                    let keys = result.unwrap_or_default();
                    if let Some(Content::Table(browser)) = self.contents.get_mut(&tab) {
                        browser.fk.keys = keys.clone();
                    }
                    if let Some(dialog) = self.edit.as_mut().filter(|d| d.tab == tab) {
                        dialog.set_foreign_keys(keys);
                    }
                }
                Event::PickerKeys { tab, result } => {
                    if let Some(dialog) = self.edit.as_mut().filter(|d| d.tab == tab) {
                        dialog.set_picker_keys(result.unwrap_or_default());
                    }
                }
                Event::TableLabel { tab, column, value, result } => {
                    if let Some(Content::Table(browser)) = self.contents.get_mut(&tab) {
                        browser.fk.resolve(&column, value, result);
                    }
                }
                Event::PickerLabel { tab, column, value, result } => {
                    if let Some(dialog) = self.edit.as_mut().filter(|d| d.tab == tab) {
                        dialog.set_picker_label(&column, value, result);
                    }
                }
                Event::Columns { database, result } => {
                    self.loading_columns.remove(&database);
                    if let Ok(columns) = result {
                        self.columns.insert(database, columns);
                    }
                }
                Event::Enums { tab, result } => {
                    if let Some(dialog) = self.edit.as_mut().filter(|d| d.tab == tab) {
                        dialog.set_enums(result.unwrap_or_default());
                    }
                }
                Event::Lookup { tab, col, generation, result } => {
                    if let Some(dialog) = self.edit.as_mut().filter(|d| d.tab == tab) {
                        dialog.set_lookup(col, generation, result);
                    }
                }
                Event::Label { tab, col, value, result } => {
                    if let Some(dialog) = self.edit.as_mut().filter(|d| d.tab == tab) {
                        dialog.set_label(col, value, result);
                    }
                }
                Event::Query { tab, result } => {
                    let Some(Content::Query(query)) = self.contents.get_mut(&tab) else { continue };
                    query.running = false;
                    match result {
                        Ok(r) => query.result = Some(r),
                        Err(e) => self.error = Some(e),
                    }
                }
            }
        }
        if let Some(database) = pending_load {
            self.load_tables(ctx, database);
        }
        if let Some(tab) = reload {
            self.load_table(ctx, tab);
        }
        if reload_all {
            let tabs: Vec<TabId> = self.contents.iter().filter(|(_, c)| matches!(c, Content::Table(_))).map(|(id, _)| *id).collect();
            for tab in tabs {
                self.load_table(ctx, tab);
            }
        }
    }

    fn connection_name(&self) -> Option<String> {
        match &self.connection {
            Connection::Connected { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    fn save_query(&mut self, tab: TabId, name: String) {
        let (Some(connection), Some(Content::Query(q))) = (self.connection_name(), self.contents.get(&tab)) else { return };
        upsert(&mut self.saved, SavedQuery { connection, name, database: q.database.clone(), sql: q.sql.clone() });
        self.persist_saved();
    }

    fn open_saved(&mut self, query: &SavedQuery) {
        let database = Some(query.database.clone()).filter(|d| self.databases.contains(d));
        self.open_query(database, None);
        if let Some(Content::Query(q)) = self.contents.get_mut(&self.next_tab) {
            q.sql = query.sql.clone();
        }
    }

    fn persist_saved(&mut self) {
        if let Err(e) = self.saved_store.save(&self.saved) {
            self.error = Some(format!("{e:#}"));
        }
    }

    fn saved_queries_section(&mut self, ui: &mut egui::Ui) {
        let Some(connection) = self.connection_name() else { return };
        let mine: Vec<SavedQuery> = self.saved.iter().filter(|q| q.connection == connection).cloned().collect();
        let mut open = None;
        let mut delete = None;
        egui::CollapsingHeader::new(format!("Saved queries ({})", mine.len())).id_salt("saved_queries").show(ui, |ui| {
            if mine.is_empty() {
                ui.label(RichText::new("Use Save on a query tab").weak().small());
            }
            egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                for query in &mine {
                    let response = ui.selectable_label(false, &query.name).on_hover_text(&query.sql);
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
        });
        if let Some(query) = open {
            self.open_saved(&query);
        }
        if let Some(query) = delete {
            self.saved.retain(|q| *q != query);
            self.persist_saved();
        }
    }

    fn persist(&mut self) {
        if let Err(e) = self.store.save(&self.profiles) {
            self.error = Some(format!("{e:#}"));
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading("Connections");
            if ui.small_button("New").clicked() {
                self.form = Some(ProfileForm::new(None, Profile::default()));
            }
        });
        let mut connect = None;
        let mut edit = None;
        let mut delete = None;
        for (i, profile) in self.profiles.iter().enumerate() {
            let active = matches!(&self.connection, Connection::Connected { name, .. } if *name == profile.name);
            let label = if profile.ssh.is_some() { format!("{} (ssh)", profile.name) } else { profile.name.clone() };
            let response = ui.selectable_label(active, label);
            if response.double_clicked() {
                connect = Some(i);
            }
            response.context_menu(|ui| {
                if ui.button("Connect").clicked() {
                    connect = Some(i);
                    ui.close();
                }
                if ui.button("Edit").clicked() {
                    edit = Some(i);
                    ui.close();
                }
                if ui.button("Delete").clicked() {
                    delete = Some(i);
                    ui.close();
                }
            });
        }
        if let Some(i) = connect {
            let profile = &self.profiles[i];
            let ask_db = profile.password.is_empty();
            let ask_ssh = profile.ssh.as_ref().is_some_and(|s| s.secret.is_empty());
            if ask_db || ask_ssh {
                self.prompt = Some(PasswordPrompt { index: i, password: String::new(), ssh_secret: String::new(), ask_db, ask_ssh, focused: false, reveal: false });
            } else {
                self.connect(ui.ctx(), i, String::new(), String::new());
            }
        }
        if let Some(i) = edit {
            self.form = Some(ProfileForm::new(Some(i), self.profiles[i].clone()));
        }
        if let Some(i) = delete {
            self.profiles.remove(i);
            self.persist();
        }

        ui.separator();
        match &self.connection {
            Connection::Disconnected => {
                ui.label(RichText::new("Double-click a connection").weak());
            }
            Connection::Connecting(name) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(format!("Connecting to {name}..."));
                });
            }
            Connection::Connected { name, .. } => {
                let mut disconnect = false;
                let mut new_query = false;
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).strong());
                    disconnect = ui.small_button("Disconnect").clicked();
                    new_query = ui.small_button("New query").clicked();
                });
                if new_query {
                    self.open_query(None, None);
                }
                if disconnect {
                    self.connection = Connection::Disconnected;
                    self.reset_databases();
                    self.close_table_tabs();
                }
            }
        }

        if self.session().is_some() {
            self.saved_queries_section(ui);
            ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filter tables"));
            let filter = self.filter.to_lowercase();
            let mut open = None;
            let mut expand = None;
            let multiple = self.databases.len() > 1;
            let active_tab = self.dock.find_active_focused().map(|(_, id)| *id);
            let open_tables: Vec<&TableRef> = active_tab
                .and_then(|id| self.contents.get(&id))
                .and_then(|c| match c {
                    Content::Table(b) => Some(&b.table),
                    Content::Query(_) => None,
                })
                .into_iter()
                .collect();
            let mut query_database = None;
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for database in &self.databases {
                    let mut list = |ui: &mut egui::Ui| match self.tables.get(database) {
                        Some(tables) => table_list(ui, tables, &filter, &open_tables, &mut open),
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
            if let Some(database) = expand {
                self.load_tables(ui.ctx(), database);
            }
            if let Some(table) = open {
                self.open_table(ui.ctx(), table);
            }
            if let Some(database) = query_database {
                self.open_query(Some(database), None);
            }
        }
    }

    fn windows(&mut self, ctx: &egui::Context) {
        if let Some(dialog) = &mut self.save_dialog {
            let (mut save, mut cancel) = (false, false);
            egui::Window::new("Save query").collapsible(false).resizable(false).show(ctx, |ui| {
                let edit = ui.add(egui::TextEdit::singleline(&mut dialog.name).hint_text("name"));
                if !std::mem::replace(&mut dialog.focused, true) {
                    edit.request_focus();
                }
                let valid = !dialog.name.trim().is_empty();
                save = valid && edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    save |= ui.add_enabled(valid, egui::Button::new("Save")).clicked();
                    cancel = ui.button("Cancel").clicked();
                });
                cancel |= ui.input(|i| i.key_pressed(egui::Key::Escape));
            });
            if save {
                let dialog = self.save_dialog.take().expect("dialog present");
                self.save_query(dialog.tab, dialog.name.trim().to_string());
            } else if cancel {
                self.save_dialog = None;
            }
        }

        if let Some(prompt) = &mut self.prompt {
            let name = self.profiles[prompt.index].name.clone();
            let (mut go, mut cancel) = (false, false);
            egui::Window::new(format!("Connect to {name}")).collapsible(false).resizable(false).show(ctx, |ui| {
                let mut focus_first = !prompt.focused;
                prompt.focused = true;
                let mut field = |ui: &mut egui::Ui, label: &str, value: &mut String, prompt_reveal: &mut bool| {
                    ui.label(label);
                    let edit = secret_field(ui, value, "", prompt_reveal);
                    if std::mem::take(&mut focus_first) {
                        edit.request_focus();
                    }
                    edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                };
                if prompt.ask_ssh {
                    go |= field(ui, "SSH key passphrase or password (empty if none)", &mut prompt.ssh_secret, &mut prompt.reveal);
                }
                if prompt.ask_db {
                    go |= field(ui, "Database password", &mut prompt.password, &mut prompt.reveal);
                }
                ui.horizontal(|ui| {
                    go |= ui.button("Connect").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
                cancel |= ui.input(|i| i.key_pressed(egui::Key::Escape));
            });
            if go {
                let prompt = self.prompt.take().expect("prompt present");
                self.connect(ctx, prompt.index, prompt.password, prompt.ssh_secret);
            } else if cancel {
                self.prompt = None;
            }
        }

        if let Some(form) = &mut self.form {
            let mut outcome = None;
            egui::Window::new("Connection").collapsible(false).resizable(false).show(ctx, |ui| {
                outcome = form.show(ui);
            });
            match outcome {
                Some(true) => {
                    let form = self.form.take().expect("form present");
                    let (index, profile) = form.finish();
                    match index {
                        Some(i) => self.profiles[i] = profile,
                        None => self.profiles.push(profile),
                    }
                    self.persist();
                }
                Some(false) => self.form = None,
                None => {}
            }
        }
    }
}

impl App {
    fn show(&mut self, ui: &mut egui::Ui) {
        self.drain_events(ui.ctx());
        self.ensure_schema(ui.ctx());

        egui::Panel::left("dbviewer_sidebar").resizable(true).default_size(240.0).show(ui, |ui| self.sidebar(ui));

        if let Some(error) = self.error.clone() {
            let can_retry = self.last_connect.is_some()
                && !matches!(self.connection, Connection::Connecting(_))
                && (matches!(self.connection, Connection::Disconnected) || is_connection_error(&error));
            egui::Panel::bottom("dbviewer_error").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                    if can_retry && ui.small_button("Retry").clicked() {
                        self.retry(ui.ctx());
                    }
                    if ui.small_button("Dismiss").clicked() {
                        self.error = None;
                    }
                });
            });
        }

        if let Some(dialog) = &mut self.edit {
            let panel = egui::Panel::right("dbviewer_edit_row").resizable(true).default_size(440.0).size_range(320.0..=900.0);
            let (outcome, tab, requests) = panel
                .show(ui, |ui| {
                    let outcome = dialog.show(ui);
                    (outcome, dialog.tab, dialog.take_requests())
                })
                .inner;
            match outcome {
                Outcome::Open => {}
                Outcome::Cancel => self.edit = None,
                Outcome::Save(changes) => self.save_edit(ui.ctx(), changes),
            }
            self.spawn_requests(ui.ctx(), tab, requests);
        }

        egui::CentralPanel::no_frame().show(ui, |ui| {
            if self.contents.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("Pick a table, or open a new query").weak());
                });
                return;
            }
            let mut cmds = Vec::new();
            let read_only = self.session().is_none_or(|s| s.read_only());
            let engine = self.session().map_or(Engine::Postgres, |s| s.engine());
            let mut viewer = Viewer {
                contents: &mut self.contents,
                databases: &self.databases,
                connected: matches!(self.connection, Connection::Connected { .. }),
                read_only,
                schema: SchemaView { engine, databases: &self.databases, tables: &self.tables, columns: &self.columns },
                cmds: &mut cmds,
            };
            DockArea::new(&mut self.dock).id(egui::Id::new("dbviewer_dock")).show_add_buttons(true).show_inside(ui, &mut viewer);
            self.handle_commands(ui.ctx(), cmds);
        });

        self.windows(ui.ctx());
    }
}

impl ProfileForm {
    fn new(index: Option<usize>, profile: Profile) -> Self {
        let ssh = profile.ssh.clone().unwrap_or_default();
        Self {
            index,
            use_ssh: profile.ssh.is_some(),
            ssh_port: ssh.port.map(|p| p.to_string()).unwrap_or_default(),
            ssh,
            profile,
            show_password: false,
            show_ssh_secret: false,
        }
    }

    /// `Some(true)` = save, `Some(false)` = cancel.
    fn show(&mut self, ui: &mut egui::Ui) -> Option<bool> {
        let mut port = self.profile.port.to_string();
        egui::Grid::new("profile_form").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut self.profile.name);
            ui.end_row();
            ui.label("Engine");
            let previous = self.profile.engine;
            egui::ComboBox::from_id_salt("profile_engine").selected_text(previous.label()).show_ui(ui, |ui| {
                for engine in Engine::ALL {
                    ui.selectable_value(&mut self.profile.engine, engine, engine.label());
                }
            });
            if self.profile.engine != previous {
                self.profile.switch_engine_defaults(previous);
                port = self.profile.port.to_string();
            }
            ui.end_row();
            ui.label("Host");
            ui.text_edit_singleline(&mut self.profile.host);
            ui.end_row();
            ui.label("Port");
            ui.text_edit_singleline(&mut port);
            ui.end_row();
            ui.label("User");
            ui.text_edit_singleline(&mut self.profile.user);
            ui.end_row();
            ui.label("Database");
            ui.add(egui::TextEdit::singleline(&mut self.profile.database).hint_text("empty = all databases"));
            ui.end_row();
            ui.label("Password");
            secret_field(ui, &mut self.profile.password, "empty = ask on connect", &mut self.show_password);
            ui.end_row();
            ui.label("");
            ui.checkbox(&mut self.profile.read_only, "Read-only session");
            ui.end_row();
            ui.label("");
            ui.checkbox(&mut self.use_ssh, "Connect through SSH tunnel");
            ui.end_row();
            if self.use_ssh {
                ui.label("SSH host");
                ui.text_edit_singleline(&mut self.ssh.host);
                ui.end_row();
                ui.label("SSH port");
                ui.text_edit_singleline(&mut self.ssh_port);
                ui.end_row();
                ui.label("SSH user");
                ui.text_edit_singleline(&mut self.ssh.user);
                ui.end_row();
                ui.label("Identity file");
                ui.text_edit_singleline(&mut self.ssh.identity_file);
                ui.end_row();
                ui.label("Passphrase");
                secret_field(ui, &mut self.ssh.secret, "key passphrase or SSH password; empty = ask", &mut self.show_ssh_secret);
                ui.end_row();
            }
        });
        if let Ok(p) = port.trim().parse() {
            self.profile.port = p;
        }
        if self.use_ssh {
            ui.label(RichText::new("Host/port above are resolved from the SSH server.").weak().small());
        }
        let mut outcome = None;
        let valid = !self.profile.name.trim().is_empty() && (!self.use_ssh || !self.ssh.host.trim().is_empty());
        if valid && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            outcome = Some(true);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            outcome = Some(false);
        }
        ui.horizontal(|ui| {
            if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                outcome = Some(true);
            }
            if ui.button("Cancel").clicked() {
                outcome = Some(false);
            }
        });
        outcome
    }

    fn finish(mut self) -> (Option<usize>, Profile) {
        self.ssh.port = self.ssh_port.trim().parse().ok();
        self.profile.ssh = self.use_ssh.then_some(self.ssh);
        (self.index, self.profile)
    }
}

fn table_list(ui: &mut egui::Ui, tables: &[TableRef], filter: &str, open_tables: &[&TableRef], open: &mut Option<TableRef>) {
    let mut current_schema = "";
    for table in tables.iter().filter(|t| t.name.to_lowercase().contains(filter)) {
        if !table.schema.is_empty() && table.schema != current_schema {
            current_schema = &table.schema;
            ui.label(RichText::new(current_schema).small().weak());
        }
        let selected = open_tables.contains(&table);
        let suffix = match table.kind {
            TableKind::Table => "",
            TableKind::View => "  view",
            TableKind::MaterializedView => "  matview",
            TableKind::Foreign => "  foreign",
        };
        if ui.selectable_label(selected, format!("{}{suffix}", table.name)).clicked() {
            *open = Some(table.clone());
        }
    }
}

fn secret_field(ui: &mut egui::Ui, value: &mut String, hint: &str, reveal: &mut bool) -> egui::Response {
    ui.horizontal(|ui| {
        let edit = ui.add(egui::TextEdit::singleline(value).password(!*reveal).hint_text(hint));
        if ui.small_button(if *reveal { "Hide" } else { "Show" }).clicked() {
            *reveal = !*reveal;
        }
        edit
    })
    .inner
}

fn is_connection_error(error: &str) -> bool {
    let error = error.to_lowercase();
    ["connection", "closed", "broken pipe", "input/output", "timed out", "reset by peer", "ssh"].iter().any(|needle| error.contains(needle))
}
