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
    OpenTab(Tab),
    FocusTabAt(usize),
    NextTab,
    PreviousTab,
    ToggleProxy,
    SetProxyMode(ProxyMode),
    ToggleInterceptor,
    ClearTraffic,
}

impl Action {
    pub fn label(self) -> String {
        match self {
            Action::OpenPalette => "Command palette".to_string(),
            Action::QuickVariables => "Quick variables".to_string(),
            Action::ImportWorkspace => "Import workspace…".to_string(),
            Action::ExportWorkspace => "Export workspace…".to_string(),
            Action::OpenTab(tab) => format!("Open {}", tab.name()),
            Action::FocusTabAt(index) => format!("Tab {}", index + 1),
            Action::NextTab => "Next tab".to_string(),
            Action::PreviousTab => "Previous tab".to_string(),
            Action::ToggleProxy => "Toggle proxy".to_string(),
            Action::SetProxyMode(mode) => format!("Proxy: {}", crate::top_bar::mode_label(mode).to_lowercase()),
            Action::ToggleInterceptor => "Toggle interceptor".to_string(),
            Action::ClearTraffic => "Clear traffic".to_string(),
        }
    }

    pub fn command(self) -> AppCommand {
        match self {
            Action::OpenPalette => AppCommand::OpenPalette,
            Action::QuickVariables => AppCommand::ToggleQuickVariables,
            Action::ImportWorkspace => AppCommand::OpenImport,
            Action::ExportWorkspace => AppCommand::ExportWorkspace,
            Action::OpenTab(tab) => AppCommand::OpenTab(tab),
            Action::FocusTabAt(index) => AppCommand::FocusTabAt(index),
            Action::NextTab => AppCommand::CycleTab { forward: true },
            Action::PreviousTab => AppCommand::CycleTab { forward: false },
            Action::ToggleProxy => AppCommand::ToggleProxy,
            Action::SetProxyMode(mode) => AppCommand::SetProxyMode(mode),
            Action::ToggleInterceptor => AppCommand::ToggleInterceptor,
            Action::ClearTraffic => AppCommand::ClearTraffic,
        }
    }
}

pub const SHORTCUTS: &[(KeyboardShortcut, Action)] = &[
    (KeyboardShortcut::new(Modifiers::COMMAND, Key::K), Action::OpenPalette),
    (KeyboardShortcut::new(Modifiers::COMMAND, Key::E), Action::QuickVariables),
    (KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma), Action::OpenTab(Tab::Settings)),
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

pub fn shortcut_for(action: Action) -> Option<KeyboardShortcut> {
    SHORTCUTS.iter().find(|(_, a)| *a == action).map(|(shortcut, _)| *shortcut)
}

pub fn palette_actions() -> Vec<Action> {
    let mut actions: Vec<Action> = Tab::ALL.into_iter().map(Action::OpenTab).collect();
    actions.extend([Action::NextTab, Action::PreviousTab, Action::ToggleProxy]);
    actions.extend(ProxyMode::ALL.map(Action::SetProxyMode));
    actions.extend([Action::ToggleInterceptor, Action::ClearTraffic, Action::QuickVariables]);
    actions.extend([Action::ImportWorkspace, Action::ExportWorkspace]);
    actions
}

pub fn consume(ctx: &egui::Context, cmds: &mut Commands) {
    for (shortcut, action) in SHORTCUTS {
        if ctx.input_mut(|i| i.consume_shortcut(shortcut)) {
            cmds.push(action.command());
        }
    }
}
