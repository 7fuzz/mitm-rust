use crate::{db, events::CoreEvent, MitmCore};

impl MitmCore {
    pub fn clear_all_traffic(&self) -> Result<(), String> {
        db::clear_history(&self.db_path)?;
        self.state.reset_history_id();
        self.event_bus.emit(CoreEvent::TrafficCleared);
        Ok(())
    }

    pub fn data_counts(&self) -> Result<db::DataCategoryCounts, String> {
        db::get_data_category_counts(&self.db_path)
    }

    pub fn clear_data_category(&self, category: db::DataCategory) -> Result<(), String> {
        match category {
            db::DataCategory::Traffic => self.clear_all_traffic(),
            db::DataCategory::WebhookDeliveries => self.clear_webhook_deliveries(),
            db::DataCategory::WsMessages => self.clear_ws_messages(),
            db::DataCategory::FuzzRuns => self.clear_fuzz_runs(),
            db::DataCategory::RewriteHistory => self.clear_rewrite_history(),
        }
    }

    pub async fn factory_reset(&self) -> Result<(), String> {
        self.stop_all_listeners().await;
        db::recovery::factory_reset(&self.db_path, Some(&self.event_bus)).map_err(|e| e.to_string())?;
        self.state.reset_history_id();
        self.event_bus.emit(CoreEvent::TrafficCleared);
        Ok(())
    }

    pub fn list_db_tables(&self) -> Result<Vec<db::browser::DbTable>, String> {
        db::browser::list_db_tables(&self.db_path)
    }

    pub fn query_db_table(
        &self,
        table: &str,
        offset: i64,
        limit: i64,
        order_by: Option<&str>,
        order_desc: Option<bool>,
        search: Option<&str>,
    ) -> Result<db::browser::DbRows, String> {
        db::browser::query_db_table(&self.db_path, table, offset, limit, order_by, order_desc, search)
    }

    pub fn get_db_row(&self, table: &str, rowid: i64) -> Result<Vec<serde_json::Value>, String> {
        db::browser::get_db_row(&self.db_path, table, rowid)
    }

    pub fn run_db_query(&self, sql: &str) -> Result<db::browser::DbQueryResult, String> {
        db::browser::run_db_query(&self.db_path, sql)
    }
}
