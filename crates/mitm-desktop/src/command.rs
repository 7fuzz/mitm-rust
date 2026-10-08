use mitm_core::db::InterceptRule;
use mitm_core::fuzzer::FuzzTemplate;
use mitm_core::state::{HistorySettings, ListenerConfig, ProxyMode};
use crate::dock::Tab;
use crate::features::collections::{CollectionsCmd, SaveTarget};
use crate::format::{CopyFormat, CopyOptions};
use crate::widgets::message_pane::SaveBody;

#[derive(Debug, Clone, Default)]
pub struct RequestDraft {
    pub method: String,
    pub url: String,
    pub headers: String,
    pub body: String,
}

#[derive(Clone)]
pub enum AppCommand {
    OpenPalette,
    ToggleQuickVariables,
    OpenTab(Tab),
    FocusTabAt(usize),
    CycleTab { forward: bool },
    ToggleProxy,
    SetProxyMode(ProxyMode),
    SaveListeners(Vec<ListenerConfig>),
    SetListenerEnabled { id: u32, enabled: bool },
    ExportCa(mitm_core::CaFormat),
    CopyCaPem,
    OpenCaFolder,
    RegenerateCa,

    ClearDataCategory(mitm_core::DataCategory),
    RefreshDataCounts,
    FactoryReset,

    SelectFlow(i64),
    SaveFlowToCollection { flow_id: i64, target: SaveTarget },
    LoadOlderFlows,
    ClearTraffic,
    DeleteFlow(i64),
    CopyFlow(i64, CopyFormat),
    CopyFlowCustom(i64, CopyOptions),
    SaveBody(SaveBody),
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
    ForwardAllIntercepts,
    DropAllIntercepts,
    ConfigureInterceptor { enabled: bool, mode: String },
    SetInterceptScope(mitm_core::state::SourceScope),
    ToggleInterceptor,
    SaveInterceptRules(Vec<InterceptRule>),

    Collections(CollectionsCmd),
    ReloadWsConnections,
    SelectWsConnection(i64),

    SelectWorkspace(String),
    SelectEnvironment(Option<String>),
    CreateWorkspace(String),
    RenameWorkspace { id: String, name: String },
    UpdateWorkspaceDescription { id: String, description: Option<String> },
    DeleteWorkspace(String),

    CreateEnvironment { workspace_id: String, name: String },
    RenameEnvironment { id: String, name: String },
    DuplicateEnvironment(String),
    DeleteEnvironment(String),

    OpenImport,
    ExportWorkspace,
    PickImportFile,
    ImportWorkspace { json: String, target: Option<String>, name: Option<String> },
}

impl AppCommand {
    pub fn touches_environments(&self) -> bool {
        matches!(
            self,
            AppCommand::SelectWorkspace(_)
                | AppCommand::SelectEnvironment(_)
                | AppCommand::CreateWorkspace(_)
                | AppCommand::DeleteWorkspace(_)
                | AppCommand::CreateEnvironment { .. }
                | AppCommand::RenameEnvironment { .. }
                | AppCommand::DuplicateEnvironment(_)
                | AppCommand::DeleteEnvironment(_)
                | AppCommand::ImportWorkspace { .. }
        )
    }
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
