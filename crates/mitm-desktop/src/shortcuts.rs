use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use crate::command::{AppCommand, Commands};
use crate::dock::Tab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    OpenPalette,
    OpenTab(Tab),
    ToggleProxy,
    ToggleInterceptor,
    ClearTraffic,
}

impl Action {
    pub fn label(self) -> String {
        match self {
            Action::OpenPalette => "Command palette".to_string(),
            Action::OpenTab(tab) => format!("Open {}", tab.name()),
            Action::ToggleProxy => "Toggle proxy".to_string(),
            Action::ToggleInterceptor => "Toggle interceptor".to_string(),
            Action::ClearTraffic => "Clear traffic".to_string(),
        }
    }

    pub fn command(self) -> AppCommand {
        match self {
            Action::OpenPalette => AppCommand::OpenPalette,
            Action::OpenTab(tab) => AppCommand::OpenTab(tab),
            Action::ToggleProxy => AppCommand::ToggleProxy,
            Action::ToggleInterceptor => AppCommand::ToggleInterceptor,
            Action::ClearTraffic => AppCommand::ClearTraffic,
        }
    }
}

pub const SHORTCUTS: &[(KeyboardShortcut, Action)] = &[
    (KeyboardShortcut::new(Modifiers::COMMAND, Key::K), Action::OpenPalette),
    (KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma), Action::OpenTab(Tab::Settings)),
];

pub fn shortcut_for(action: Action) -> Option<KeyboardShortcut> {
    SHORTCUTS.iter().find(|(_, a)| *a == action).map(|(shortcut, _)| *shortcut)
}

pub fn palette_actions() -> Vec<Action> {
    let mut actions: Vec<Action> = Tab::ALL.into_iter().map(Action::OpenTab).collect();
    actions.extend([Action::ToggleProxy, Action::ToggleInterceptor, Action::ClearTraffic]);
    actions
}

pub fn consume(ctx: &egui::Context, cmds: &mut Commands) {
    for (shortcut, action) in SHORTCUTS {
        if ctx.input_mut(|i| i.consume_shortcut(shortcut)) {
            cmds.push(action.command());
        }
    }
}
