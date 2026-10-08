use std::path::PathBuf;
use crate::features::collections::CollectionsState;
use crate::features::encoding::EncodingState;
use crate::features::fuzzer::FuzzerState;
use crate::features::hashing::HashingState;
use crate::features::hmac_tool::HmacToolState;
use crate::features::inspector::InspectorState;
use crate::features::intercept::InterceptState;
use crate::features::repeater::RepeaterState;
use crate::features::settings::{CertificateState, DataSectionState, ListenerForm, ProxyState, SettingsPrefs};
use crate::features::traffic::TrafficState;
use crate::features::webhooks::WebhooksState;
use crate::features::websockets::WebSocketsState;

#[derive(Default)]
pub struct AppState {
    pub proxy: ProxyState,
    pub certificate: CertificateState,
    pub data: DataSectionState,
    pub listener_form: ListenerForm,
    pub settings: SettingsPrefs,
    pub traffic: TrafficState,
    pub inspector: InspectorState,
    pub intercept: InterceptState,
    pub rewrite: crate::features::rewrite::RewriteState,
    pub repeater: RepeaterState,
    pub fuzzer: FuzzerState,
    pub collections: CollectionsState,
    pub websockets: WebSocketsState,
    pub webhooks: WebhooksState,
    pub encoding: EncodingState,
    pub hashing: HashingState,
    pub hmac_tool: HmacToolState,
    pub database: crate::features::database::DatabaseState,
    pub environments: crate::features::environments::EnvironmentsState,
}

impl AppState {
    pub fn load(db_path: &PathBuf) -> Self {
        let mut state = Self::default();
        state.traffic.load_from_db(db_path);
        state.repeater.load(db_path);
        if let Ok(rules) = mitm_core::db::load_intercept_rules(db_path) {
            state.intercept.rules = rules;
        }
        state.environments.load(db_path);
        state.collections.load(db_path, state.environments.active_workspace_id.clone());
        state.websockets.load_connections(db_path);
        state.webhooks.load(db_path);
        state.data.counts = mitm_core::db::get_data_category_counts(db_path).ok();
        state.fuzzer.load_saved_runs(db_path);
        state
    }
}
