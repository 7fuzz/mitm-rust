pub mod ca;
pub mod collections;
pub mod credentials;
pub mod db;
pub mod events;
pub mod fuzzer;
pub mod http;
pub mod instance;
pub mod proxy;
pub mod repeater;
pub mod state;
pub mod utils;
pub mod webhook;
pub mod workspace;
pub mod ws;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
pub use ca::{CaFormat, CaInfo};
pub use db::{DataCategory, DataCategoryCounts};
pub use events::{CoreEvent, EventBus};
pub use state::{AppState, HistoryEntry, RewriteHistoryEntry};
pub use workspace::{Environment, EnvironmentVariable, Unresolved, VariableVariant, Workspace};

pub struct MitmCore {
    pub state: Arc<AppState>,
    pub event_bus: EventBus,
    pub db_path: PathBuf,
}

impl MitmCore {
    pub fn init() -> Result<Self, db::InitError> {
        Self::init_with_events(EventBus::default())
    }

    /// Like `init`, but on a bus the caller already subscribed to, so migration progress is observable.
    pub fn init_with_events(event_bus: EventBus) -> Result<Self, db::InitError> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let db_path = db::get_default_db_path();

        db::init_database(&db_path, Some(&event_bus))?;
        let _ = fuzzer::discard_unsaved_runs_db(&db_path);
        let _ = db::ws_db::close_stale_ws_connections(&db_path, chrono::Local::now().timestamp_millis());

        let (history_tx, history_rx) = mpsc::channel::<HistoryEntry>(2000);
        let (rewrite_tx, rewrite_rx) = mpsc::channel::<RewriteHistoryEntry>(2000);

        db::actor::start_history_actor(db_path.clone(), history_rx);
        db::actor::start_rewrite_history_actor(db_path.clone(), rewrite_rx);

        let state = Arc::new(AppState::new(db_path.clone(), history_tx, rewrite_tx));

        Ok(Self {
            state,
            event_bus,
            db_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;

    fn test_core() -> (MitmCore, PathBuf) {
        let dir = std::env::temp_dir().join(format!("mitm-core-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test.db");
        db::init_database(&db_path, None).unwrap();
        let (history_tx, _) = mpsc::channel(1);
        let (rewrite_tx, _) = mpsc::channel(1);
        let state = Arc::new(AppState::new(db_path.clone(), history_tx, rewrite_tx));
        (MitmCore { state, event_bus: EventBus::default(), db_path }, dir)
    }

    fn pause(core: &MitmCore, id: &str) -> oneshot::Receiver<state::InterceptAction> {
        let (tx, rx) = oneshot::channel();
        core.state.pending_flows.insert(
            id.to_string(),
            state::PendingFlow {
                flow_id: id.to_string(),
                phase: state::InterceptPhase::Request,
                method: "GET".to_string(),
                url: "http://a".to_string(),
                headers: vec![],
                body: vec![],
                tx,
            },
        );
        rx
    }

    #[test]
    fn interceptor_settings_survive_a_restart() {
        let (core, dir) = test_core();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            core.toggle_interceptor(true, "both".to_string()).await.unwrap();
            core.set_intercept_source_scope(state::SourceScope { all: false, ids: vec![2] }).await.unwrap();
        });
        let (history_tx, _) = mpsc::channel(1);
        let (rewrite_tx, _) = mpsc::channel(1);
        let reopened = MitmCore {
            state: Arc::new(AppState::new(core.db_path.clone(), history_tx, rewrite_tx)),
            event_bus: EventBus::default(),
            db_path: core.db_path.clone(),
        };
        let (enabled, mode, scope) = rt.block_on(reopened.intercept_config());
        assert!(enabled);
        assert_eq!(mode, "both");
        assert!(!scope.all && scope.ids == vec![2]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rewrite_settings_and_rule_order_persist() {
        let (core, dir) = test_core();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let rule = |id: &str| state::RewriteRule {
            id: id.to_string(),
            name: id.to_string(),
            enabled: true,
            action_type: "partial_request".to_string(),
            match_field: "all".to_string(),
            match_operator: "contains".to_string(),
            match_value: String::new(),
            target_part: "body".to_string(),
            target_header: None,
            match_pattern: String::new(),
            replacement_value: String::new(),
            is_regex: false,
            mock_status_code: None,
            mock_headers_json: None,
            mock_body: None,
            order_index: 9,
            created_at_ms: 0,
        };
        rt.block_on(async {
            core.save_rewrite_rules(vec![rule("b"), rule("a")]).await.unwrap();
            core.set_rewrite_source_scope(state::SourceScope { all: false, ids: vec![1] }).await.unwrap();
        });
        core.set_rewrite_enabled(false).unwrap();

        let loaded = db::load_rewrite_rules(&core.db_path).unwrap();
        assert_eq!(loaded.iter().map(|r| (r.id.as_str(), r.order_index)).collect::<Vec<_>>(), [("b", 0), ("a", 1)]);
        assert_eq!(db::get_preference(&core.db_path, "rewrite_enabled").as_deref(), Some("false"));
        assert!(!rt.block_on(core.rewrite_source_scope()).all);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn forward_all_and_drop_all_release_every_paused_flow() {
        let (core, dir) = test_core();
        let mut forwarded = [pause(&core, "a"), pause(&core, "b")];
        assert_eq!(core.forward_all_intercepted(), 2);
        assert!(forwarded.iter_mut().all(|rx| matches!(rx.try_recv(), Ok(state::InterceptAction::Forward { modified_url: None, .. }))));

        let mut dropped = pause(&core, "c");
        assert_eq!(core.drop_all_intercepted(), 1);
        assert!(matches!(dropped.try_recv(), Ok(state::InterceptAction::Drop)));
        assert!(core.state.pending_flows.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
