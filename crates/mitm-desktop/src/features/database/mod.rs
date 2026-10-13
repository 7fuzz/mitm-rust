mod view;

use mitm_core::db::browser::{DbColumn, DbQueryResult, DbRows, DbTable};
use mitm_core::db::Db;
use crate::app::jobs::Latest;

pub use view::show;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseTab {
    Tables,
    Query,
}

pub struct DatabaseState {
    pub tab: DatabaseTab,
    pub tables: Vec<DbTable>,
    pub table_filter: String,
    pub selected_table: Option<String>,
    pub table_columns: Vec<DbColumn>,
    pub tables_fraction: f32,

    pub current_rows: Option<DbRows>,
    pub page: i64,
    pub page_size: i64,
    pub sort_column: Option<String>,
    pub sort_desc: bool,
    pub search_query: String,

    pub selected_row_index: Option<usize>,
    pub detail_row_values: Option<Vec<serde_json::Value>>,
    pub detail_split_fraction: f32,

    pub error: Option<String>,

    pub sql_query: String,
    pub query_result: Option<DbQueryResult>,
    pub query_error: Option<String>,

    page_wanted: bool,
    page_job: Latest<Result<DbRows, String>>,
    query_wanted: bool,
    query_job: Latest<Result<DbQueryResult, String>>,
}

impl Default for DatabaseState {
    fn default() -> Self {
        Self {
            tab: DatabaseTab::Tables,
            tables: Vec::new(),
            table_filter: String::new(),
            selected_table: None,
            table_columns: Vec::new(),
            tables_fraction: 0.22,

            current_rows: None,
            page: 0,
            page_size: 50,
            sort_column: None,
            sort_desc: false,
            search_query: String::new(),

            selected_row_index: None,
            detail_row_values: None,
            detail_split_fraction: 0.65,

            error: None,

            sql_query: "SELECT * FROM history LIMIT 50;".to_string(),
            query_result: None,
            query_error: None,

            page_wanted: false,
            page_job: Latest::default(),
            query_wanted: false,
            query_job: Latest::default(),
        }
    }
}

impl DatabaseState {
    pub fn ensure_loaded(&mut self, db: &Db) {
        if self.tables.is_empty() {
            self.load_tables(db);
        }
    }

    pub fn load_tables(&mut self, db: &Db) {
        match mitm_core::db::browser::list_db_tables(db) {
            Ok(tables) => {
                self.tables = tables;
                self.error = None;
                if self.selected_table.is_none() && !self.tables.is_empty() {
                    let first = self.tables[0].name.clone();
                    self.select_table(first);
                }
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }

    pub fn select_table(&mut self, table: String) {
        self.selected_table = Some(table.clone());
        self.table_columns = self
            .tables
            .iter()
            .find(|t| t.name == table)
            .map(|t| t.columns.clone())
            .unwrap_or_default();
        self.page = 0;
        self.search_query.clear();
        self.sort_column = None;
        self.sort_desc = false;
        self.selected_row_index = None;
        self.detail_row_values = None;
        self.request_page();
    }

    /// Reloads the current page (with the search and sort) on a background thread; see `run_jobs`.
    pub fn request_page(&mut self) {
        self.page_wanted = true;
    }

    pub fn request_query(&mut self) {
        self.query_wanted = !self.sql_query.trim().is_empty();
    }

    pub fn is_loading_page(&self) -> bool {
        self.page_wanted || self.page_job.is_running()
    }

    pub fn is_running_query(&self) -> bool {
        self.query_wanted || self.query_job.is_running()
    }

    /// Starts requested queries and applies finished ones; a search over a large table must not hold up the UI.
    pub fn run_jobs(&mut self, db: &Db, ctx: &eframe::egui::Context) {
        if std::mem::take(&mut self.page_wanted) {
            if let Some(table) = self.selected_table.clone() {
                let db = db.clone();
                let (offset, limit) = (self.page * self.page_size, self.page_size);
                let (sort, desc) = (self.sort_column.clone(), self.sort_desc);
                let search = Some(self.search_query.trim().to_string()).filter(|s| !s.is_empty());
                self.page_job.start(ctx, move || {
                    mitm_core::db::browser::query_db_table(&db, &table, offset, limit, sort.as_deref(), Some(desc), search.as_deref())
                });
            }
        }
        if let Some(result) = self.page_job.take() {
            match result {
                Ok(rows) => {
                    self.current_rows = Some(rows);
                    self.error = None;
                }
                Err(e) => self.error = Some(e),
            }
        }
        if std::mem::take(&mut self.query_wanted) {
            let (db, sql) = (db.clone(), self.sql_query.trim().to_string());
            self.query_job.start(ctx, move || mitm_core::db::browser::run_db_query(&db, &sql));
        }
        if let Some(result) = self.query_job.take() {
            match result {
                Ok(result) => {
                    self.query_result = Some(result);
                    self.query_error = None;
                }
                Err(e) => {
                    self.query_result = None;
                    self.query_error = Some(e);
                }
            }
        }
    }

    pub fn select_row(&mut self, db: &Db, index: usize) {
        self.selected_row_index = Some(index);
        let Some(table) = self.selected_table.as_deref() else { return };
        let rowid = self.current_rows.as_ref().and_then(|r| r.rowids.as_ref()).and_then(|ids| ids.get(index)).copied();
        if let Some(rowid) = rowid {
            if let Ok(full_values) = mitm_core::db::browser::get_db_row(db, table, rowid) {
                self.detail_row_values = Some(full_values);
                return;
            }
        }
        if let Some(rows) = &self.current_rows {
            if let Some(row) = rows.rows.get(index) {
                self.detail_row_values = Some(row.clone());
            }
        }
    }

    pub fn toggle_sort(&mut self, column: &str) {
        if self.sort_column.as_deref() == Some(column) {
            if !self.sort_desc {
                self.sort_desc = true;
            } else {
                self.sort_column = None;
                self.sort_desc = false;
            }
        } else {
            self.sort_column = Some(column.to_string());
            self.sort_desc = false;
        }
        self.page = 0;
        self.request_page();
    }
}
