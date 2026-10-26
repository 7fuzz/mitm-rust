use mitm_core::http::curl::ParsedCurl;
use crate::ui::dock::Tab;
use crate::features::collections::{CollectionsCmd, SaveTarget};
use crate::widgets::message_pane::SaveBody;

/// Where a decoded QR code goes.
#[derive(Clone)]
pub enum QrTarget {
    TotpTool,
    CredentialField { id: String, key: String },
}

#[derive(Clone)]
pub enum AppCommand {
    OpenPalette,
    ToggleQuickVariables,
    OpenTab(Tab),
    DetachInspector(bool),
    OpenSettings(crate::features::settings::SettingsSection),
    FocusTabAt(usize),
    CycleTab { forward: bool },
    Settings(crate::features::settings::Cmd),
    FactoryReset,

    SaveFlowToCollection { flow_id: i64, target: SaveTarget },
    Traffic(crate::features::traffic::Cmd),
    FocusSearch,
    SaveBody(SaveBody),
    AddInterceptRuleForHost(String),
    SendFlowToRepeater(i64),
    SendFlowToFuzzer(i64),

    OpenCurlImport,
    RepeaterImportCurl(ParsedCurl),
    Repeater(crate::features::repeater::Cmd),
    RepeaterSend(String),
    Fuzzer(crate::features::fuzzer::Cmd),
    SendFuzzResultToRepeater { run_id: i64, idx: u32 },

    Intercept(crate::features::intercept::Cmd),
    Rewrite(crate::features::rewrite::Cmd),
    ShowRewriteFor(i64),

    Collections(CollectionsCmd),
    WebSockets(crate::features::websockets::Cmd),

    Webhooks(crate::features::webhooks::Cmd),
    Containers(crate::features::containers::Cmd),
    PickMarkdownFile,
    DecodeInEncodingTool(String),
    Hashing(crate::features::utilities::hashing::Cmd),
    RecomputeHmac { secret: String, payload: String },

    SelectWorkspace(String),
    CreateWorkspace(String),
    DeleteWorkspace(String),

    Environments(crate::features::environments::Cmd),
    MoveToCredentials { env_id: String, var_idx: usize, vault: String, entry: String, field: String },
    Credentials(crate::features::credentials::Cmd),
    PickCredentialsJson,
    PickQrImage(QrTarget),

    OpenImport,
    ExportWorkspace,
    ExportWorkspaceFile { options: mitm_core::workspace::file::ExportOptions, passphrase: Option<String> },
    PickImportFile,
    ImportWorkspace { json: String, target: Option<String>, name: Option<String>, passphrase: Option<String> },
}

impl AppCommand {
    pub fn touches_environments(&self) -> bool {
        matches!(
            self,
            AppCommand::SelectWorkspace(_)
                | AppCommand::CreateWorkspace(_)
                | AppCommand::DeleteWorkspace(_)
                | AppCommand::ImportWorkspace { .. }
        ) || matches!(self, AppCommand::Environments(cmd) if cmd.reloads())
    }
}

#[derive(Default)]
pub struct Commands(Vec<AppCommand>);

impl Commands {
    pub fn push(&mut self, cmd: impl Into<AppCommand>) {
        self.0.push(cmd.into());
    }

    pub fn take(&mut self) -> Vec<AppCommand> {
        std::mem::take(&mut self.0)
    }
}
