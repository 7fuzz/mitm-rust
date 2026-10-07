pub mod ca;
pub mod collections;
pub mod db;
pub mod encoding;
pub mod events;
pub mod fuzzer;
pub mod instance;
pub mod proxy;
pub mod repeater;
pub mod request_steps;
pub mod state;
pub mod webhook;
pub mod workspace;
pub mod ws;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
pub use events::{CoreEvent, EventBus};
pub use state::{AppState, HistoryEntry, RewriteHistoryEntry};

pub struct MitmCore {
    pub state: Arc<AppState>,
    pub event_bus: EventBus,
    pub db_path: PathBuf,
}

impl MitmCore {
    pub fn init() -> Result<Self, String> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let db_path = db::get_default_db_path();
        let event_bus = EventBus::default();

        db::init_database(&db_path, Some(&event_bus))?;

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
