use std::path::PathBuf;
use crate::features::collections::CollectionsState;
use crate::features::fuzzer::FuzzerState;
use crate::features::inspector::InspectorState;
use crate::features::intercept::InterceptState;
use crate::features::repeater::RepeaterState;
use crate::features::settings::{ListenerForm, ProxyState};
use crate::features::traffic::TrafficState;
use crate::features::websockets::WebSocketsState;

#[derive(Default)]
pub struct AppState {
    pub proxy: ProxyState,
    pub listener_form: ListenerForm,
    pub traffic: TrafficState,
    pub inspector: InspectorState,
    pub intercept: InterceptState,
    pub repeater: RepeaterState,
    pub fuzzer: FuzzerState,
    pub collections: CollectionsState,
    pub websockets: WebSocketsState,
}

impl AppState {
    pub fn load(db_path: &PathBuf) -> Self {
        let mut state = Self::default();
        state.traffic.load_from_db(db_path);
        state.repeater.load(db_path);
        if let Ok(rules) = mitm_core::db::load_intercept_rules(db_path) {
            state.intercept.rules = rules;
        }
        state.collections.load(db_path);
        state.websockets.load_connections(db_path);
        state
    }
}
