use mitm_core::db::InterceptRule;
use mitm_core::fuzzer::FuzzTemplate;
use mitm_core::state::{HistorySettings, ListenerConfig};
use crate::dock::Tab;
use crate::features::collections::CollectionsCmd;
use crate::format::CopyFormat;

#[derive(Debug, Clone, Default)]
pub struct RequestDraft {
    pub method: String,
    pub url: String,
    pub headers: String,
    pub body: String,
}

pub enum AppCommand {
    OpenTab(Tab),
    ToggleProxy,
    SaveListeners(Vec<ListenerConfig>),

    SelectFlow(i64),
    ClearTraffic,
    DeleteFlow(i64),
    CopyFlow(i64, CopyFormat),
    SetHistoryLimit(HistorySettings),
    AddInterceptRuleForHost(String),
    SendFlowToRepeater(i64),
    SendFlowToFuzzer(i64),

    RepeaterNew,
    RepeaterSelect(String),
    RepeaterDelete(String),
    RepeaterSend(String),
    StartFuzzer { template: FuzzTemplate, payloads: Vec<String>, concurrency: usize },

    ForwardIntercept {
        flow_id: String,
        url: Option<String>,
        method: Option<String>,
        headers: Option<Vec<(String, String)>>,
        body: Option<Vec<u8>>,
    },
    DropIntercept { flow_id: String },
    ConfigureInterceptor { enabled: bool, mode: String },
    SaveInterceptRules(Vec<InterceptRule>),

    Collections(CollectionsCmd),
    ReloadWsConnections,
    SelectWsConnection(i64),
}

#[derive(Default)]
pub struct Commands(Vec<AppCommand>);

impl Commands {
    pub fn push(&mut self, cmd: AppCommand) {
        self.0.push(cmd);
    }

    pub fn take(&mut self) -> Vec<AppCommand> {
        std::mem::take(&mut self.0)
    }
}
