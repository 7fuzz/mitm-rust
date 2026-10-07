use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use crate::command::{AppCommand, Commands};
use crate::dock::Tab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    OpenTab(Tab),
}

impl Action {
    pub fn command(self) -> AppCommand {
        match self {
            Action::OpenTab(tab) => AppCommand::OpenTab(tab),
        }
    }
}

pub const SHORTCUTS: &[(KeyboardShortcut, Action)] = &[
    (KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma), Action::OpenTab(Tab::Settings)),
];

pub fn consume(ctx: &egui::Context, cmds: &mut Commands) {
    for (shortcut, action) in SHORTCUTS {
        if ctx.input_mut(|i| i.consume_shortcut(shortcut)) {
            cmds.push(action.command());
        }
    }
}
