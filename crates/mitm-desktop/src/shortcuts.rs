use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use crate::command::{AppCommand, Commands};
use mitm_core::state::ProxyMode;
use crate::dock::Tab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    OpenPalette,
    QuickVariables,
    ImportWorkspace,
    ExportWorkspace,
    ImportCurl,
    InterceptForward,
    InterceptForwardUnmodified,
    InterceptDrop,
    OpenTab(Tab),
    FocusTabAt(usize),
    NextTab,
    PreviousTab,
    ToggleProxy,
    SetProxyMode(ProxyMode),
    ToggleInterceptor,
    ClearTraffic,
    FocusSearch,
}

impl Action {
    pub fn label(self) -> String {
        match self {
            Action::OpenPalette => "Command palette".to_string(),
            Action::QuickVariables => "Quick variables".to_string(),
            Action::ImportWorkspace => "Import workspace…".to_string(),
            Action::ExportWorkspace => "Export workspace…".to_string(),
            Action::ImportCurl => "Import cURL…".to_string(),
            Action::InterceptForward => "Intercept: forward with changes".to_string(),
            Action::InterceptForwardUnmodified => "Intercept: forward unmodified".to_string(),
            Action::InterceptDrop => "Intercept: drop".to_string(),
            Action::OpenTab(tab) if tab.is_utility() => format!("Utilities: {}", tab.name()),
            Action::OpenTab(tab) => format!("Open {}", tab.name()),
            Action::FocusTabAt(index) => format!("Tab {}", index + 1),
            Action::NextTab => "Next tab".to_string(),
            Action::PreviousTab => "Previous tab".to_string(),
            Action::ToggleProxy => "Toggle proxy".to_string(),
            Action::SetProxyMode(mode) => format!("Proxy: {}", crate::top_bar::mode_label(mode).to_lowercase()),
            Action::ToggleInterceptor => "Toggle interceptor".to_string(),
            Action::ClearTraffic => "Clear traffic".to_string(),
            Action::FocusSearch => "Focus search".to_string(),
        }
    }

    pub fn command(self) -> AppCommand {
        match self {
            Action::OpenPalette => AppCommand::OpenPalette,
            Action::QuickVariables => AppCommand::ToggleQuickVariables,
            Action::ImportWorkspace => AppCommand::OpenImport,
            Action::ExportWorkspace => AppCommand::ExportWorkspace,
            Action::ImportCurl => AppCommand::OpenCurlImport,
            Action::InterceptForward => AppCommand::ForwardActiveIntercept { modified: true },
            Action::InterceptForwardUnmodified => AppCommand::ForwardActiveIntercept { modified: false },
            Action::InterceptDrop => AppCommand::DropActiveIntercept,
            Action::OpenTab(tab) => AppCommand::OpenTab(tab),
            Action::FocusTabAt(index) => AppCommand::FocusTabAt(index),
            Action::NextTab => AppCommand::CycleTab { forward: true },
            Action::PreviousTab => AppCommand::CycleTab { forward: false },
            Action::ToggleProxy => AppCommand::ToggleProxy,
            Action::SetProxyMode(mode) => AppCommand::SetProxyMode(mode),
            Action::ToggleInterceptor => AppCommand::ToggleInterceptor,
            Action::ClearTraffic => AppCommand::ClearTraffic,
            Action::FocusSearch => AppCommand::FocusSearch,
        }
    }
}

pub const SHORTCUTS: &[(KeyboardShortcut, Action)] = &[
    (KeyboardShortcut::new(Modifiers::COMMAND, Key::K), Action::OpenPalette),
    (KeyboardShortcut::new(Modifiers::COMMAND, Key::E), Action::QuickVariables),
    (KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Enter), Action::InterceptForwardUnmodified),
    (KeyboardShortcut::new(Modifiers::COMMAND, Key::Enter), Action::InterceptForward),
    (KeyboardShortcut::new(Modifiers::COMMAND, Key::Backspace), Action::InterceptDrop),
    (KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma), Action::OpenTab(Tab::Settings)),
    (KeyboardShortcut::new(Modifiers::COMMAND, Key::F), Action::FocusSearch),
    // Shift variants first: consume_shortcut also matches a shortcut when extra shift is held.
    (KeyboardShortcut::new(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Tab), Action::PreviousTab),
    (KeyboardShortcut::new(Modifiers::CTRL, Key::Tab), Action::NextTab),
    (KeyboardShortcut::new(Modifiers::ALT, Key::Num1), Action::FocusTabAt(0)),
    (KeyboardShortcut::new(Modifiers::ALT, Key::Num2), Action::FocusTabAt(1)),
    (KeyboardShortcut::new(Modifiers::ALT, Key::Num3), Action::FocusTabAt(2)),
    (KeyboardShortcut::new(Modifiers::ALT, Key::Num4), Action::FocusTabAt(3)),
    (KeyboardShortcut::new(Modifiers::ALT, Key::Num5), Action::FocusTabAt(4)),
    (KeyboardShortcut::new(Modifiers::ALT, Key::Num6), Action::FocusTabAt(5)),
    (KeyboardShortcut::new(Modifiers::ALT, Key::Num7), Action::FocusTabAt(6)),
    (KeyboardShortcut::new(Modifiers::ALT, Key::Num8), Action::FocusTabAt(7)),
    (KeyboardShortcut::new(Modifiers::ALT, Key::Num9), Action::FocusTabAt(8)),
];

/// Rows for the Settings list: keys, action, and the tab the shortcut is limited to. Alt+N runs collapse into one row.
pub fn listing(ctx: &egui::Context) -> Vec<(String, String, Option<&'static str>)> {
    let mut rows = Vec::new();
    let mut focus_keys: Vec<String> = Vec::new();
    for &(shortcut, action) in SHORTCUTS {
        let keys = ctx.format_shortcut(&shortcut);
        if let Action::FocusTabAt(_) = action {
            focus_keys.push(keys);
            continue;
        }
        rows.push((keys, action.label(), scope(action).map(|tab| tab.name())));
    }
    if let (Some(first), Some(last)) = (focus_keys.first(), focus_keys.last()) {
        rows.push((format!("{first} … {last}"), "Focus tab by position".to_string(), None));
    }
    rows
}

pub fn shortcut_for(action: Action) -> Option<KeyboardShortcut> {
    SHORTCUTS.iter().find(|(_, a)| *a == action).map(|(shortcut, _)| *shortcut)
}

pub fn palette_actions() -> Vec<Action> {
    let mut actions: Vec<Action> = Tab::ALL.into_iter().map(Action::OpenTab).collect();
    actions.extend([Action::NextTab, Action::PreviousTab, Action::ToggleProxy]);
    actions.extend(ProxyMode::ALL.map(Action::SetProxyMode));
    actions.extend([Action::ToggleInterceptor, Action::ClearTraffic, Action::QuickVariables, Action::FocusSearch]);
    actions.extend([Action::ImportWorkspace, Action::ExportWorkspace, Action::ImportCurl]);
    actions.extend([Action::InterceptForward, Action::InterceptForwardUnmodified, Action::InterceptDrop]);
    actions
}

/// The tab whose shortcuts only fire while it has focus, so Ctrl+Enter stays free elsewhere.
fn scope(action: Action) -> Option<Tab> {
    match action {
        Action::InterceptForward | Action::InterceptForwardUnmodified | Action::InterceptDrop => Some(Tab::Intercept),
        _ => None,
    }
}

pub fn consume(ctx: &egui::Context, focused_tab: Option<Tab>, cmds: &mut Commands) {
    for &(shortcut, action) in SHORTCUTS {
        if scope(action).is_some_and(|tab| Some(tab) != focused_tab) {
            continue;
        }
        // Ctrl+Backspace deletes a word while typing; dropping a flow by accident would lose it.
        if action == Action::InterceptDrop && ctx.egui_wants_keyboard_input() {
            continue;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
            cmds.push(action.command());
        }
    }
}
