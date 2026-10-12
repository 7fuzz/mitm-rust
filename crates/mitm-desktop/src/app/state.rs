use crate::features::collections::CollectionsState;
use crate::features::credentials::CredentialsState;
use crate::features::utilities::cvss::CvssState;
use crate::features::utilities::encoding::EncodingState;
use crate::features::fuzzer::FuzzerState;
use crate::features::utilities::hashing::HashingState;
use crate::features::utilities::hmac_tool::HmacToolState;
use crate::features::utilities::json_diff::JsonDiffState;
use crate::features::utilities::json_tools::JsonToolsState;
use crate::features::utilities::totp::TotpState;
use crate::features::inspector::InspectorState;
use crate::features::intercept::InterceptState;
use crate::features::repeater::RepeaterState;
use crate::features::settings::{CertificateState, DataSectionState, ListenerForm, ProxyState, SettingsPrefs};
use crate::features::traffic::TrafficState;
use crate::features::webhooks::WebhooksState;
use crate::features::websockets::WebSocketsState;
use mitm_core::db::Db;

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
    pub cvss: CvssState,
    pub encoding: EncodingState,
    pub hashing: HashingState,
    pub hmac_tool: HmacToolState,
    pub json_diff: JsonDiffState,
    pub json_tools: JsonToolsState,
    pub totp: TotpState,
    pub database: crate::features::database::DatabaseState,
    pub environments: crate::features::environments::EnvironmentsState,
    pub credentials: CredentialsState,
}

impl AppState {
    pub fn settings(&mut self) -> crate::features::settings::Settings<'_> {
        crate::features::settings::Settings {
            proxy: &mut self.proxy,
            cert: &mut self.certificate,
            data: &mut self.data,
            traffic: &mut self.traffic,
        }
    }

    pub fn load(db: &Db) -> Self {
        let mut state = Self::default();
        state.traffic.load_from_db(db);
        state.repeater.load(db);
        if let Ok(rules) = mitm_core::db::load_intercept_rules(db) {
            state.intercept.rules = rules;
        }
        state.environments.load(db);
        state.collections.load(db, state.environments.active_workspace_id.clone());
        state.websockets.load_connections(db);
        state.webhooks.load(db);
        state.data.counts = mitm_core::db::get_data_category_counts(db).ok();
        state.fuzzer.load_saved_runs(db);
        state
    }
}
