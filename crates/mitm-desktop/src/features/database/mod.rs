mod view;

use std::path::Path;
use mitm_core::db::browser::{DbColumn, DbQueryResult, DbRows, DbTable};

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
        }
    }
}

impl DatabaseState {
    pub fn ensure_loaded(&mut self, db_path: &Path) {
        if self.tables.is_empty() {
            self.load_tables(db_path);
        }
    }

    pub fn load_tables(&mut self, db_path: &Path) {
        match mitm_core::db::browser::list_db_tables(db_path) {
            Ok(tables) => {
                self.tables = tables;
                self.error = None;
                if self.selected_table.is_none() && !self.tables.is_empty() {
                    let first = self.tables[0].name.clone();
                    self.select_table(db_path, first);
                }
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }

    pub fn select_table(&mut self, db_path: &Path, table: String) {
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
        self.load_current_page(db_path);
    }

    pub fn load_current_page(&mut self, db_path: &Path) {
        let Some(table) = self.selected_table.clone() else { return };
        let offset = self.page * self.page_size;
        let search = if self.search_query.trim().is_empty() {
            None
        } else {
            Some(self.search_query.trim())
        };
        match mitm_core::db::browser::query_db_table(
            db_path,
            &table,
            offset,
            self.page_size,
            self.sort_column.as_deref(),
            Some(self.sort_desc),
            search,
        ) {
            Ok(rows) => {
                self.current_rows = Some(rows);
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }

    pub fn select_row(&mut self, db_path: &Path, index: usize) {
        self.selected_row_index = Some(index);
        let Some(table) = self.selected_table.as_deref() else { return };
        let rowid = self.current_rows.as_ref().and_then(|r| r.rowids.as_ref()).and_then(|ids| ids.get(index)).copied();
        if let Some(rowid) = rowid {
            if let Ok(full_values) = mitm_core::db::browser::get_db_row(db_path, table, rowid) {
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

    pub fn toggle_sort(&mut self, db_path: &Path, column: &str) {
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
        self.load_current_page(db_path);
    }

    pub fn run_query(&mut self, db_path: &Path) {
        let sql = self.sql_query.trim();
        if sql.is_empty() {
            return;
        }
        match mitm_core::db::browser::run_db_query(db_path, sql) {
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
