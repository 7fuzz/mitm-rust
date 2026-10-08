use std::path::Path;
use eframe::egui;
use egui_dock::{DockState, TabIndex};
use crate::command::Commands;
use crate::features;
use crate::widgets::template;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Tab {
    Traffic,
    Repeater,
    Fuzzer,
    Intercept,
    Collections,
    WebSockets,
    Environments,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 8] = [
        Tab::Traffic,
        Tab::Repeater,
        Tab::Fuzzer,
        Tab::Intercept,
        Tab::Collections,
        Tab::WebSockets,
        Tab::Environments,
        Tab::Settings,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Tab::Traffic => "Traffic",
            Tab::Repeater => "Repeater",
            Tab::Fuzzer => "Fuzzer",
            Tab::Intercept => "Intercept",
            Tab::Collections => "Collections",
            Tab::WebSockets => "WebSockets",
            Tab::Environments => "Environments",
            Tab::Settings => "Settings",
        }
    }
}

pub fn initial_layout() -> DockState<Tab> {
    DockState::new(vec![Tab::Traffic, Tab::Intercept, Tab::Repeater])
}

pub fn open_tab(dock: &mut DockState<Tab>, tab: Tab) {
    match dock.find_tab(&tab) {
        Some(path) => {
            let _ = dock.set_active_tab(path);
        }
        None => dock.push_to_focused_leaf(tab),
    }
}

pub fn focus_tab_at(dock: &mut DockState<Tab>, index: usize) {
    let Some(path) = dock.focused_leaf() else { return };
    if let Ok(leaf) = dock.leaf_mut(path) {
        if index < leaf.tabs.len() {
            leaf.active = TabIndex(index);
        }
    }
}

pub fn cycle_tab(dock: &mut DockState<Tab>, forward: bool) {
    let Some(path) = dock.focused_leaf() else { return };
    if let Ok(leaf) = dock.leaf_mut(path) {
        let count = leaf.tabs.len();
        if count > 0 {
            let step = if forward { 1 } else { count - 1 };
            leaf.active = TabIndex((leaf.active.0 + step) % count);
        }
    }
}

pub struct TabViewer<'a> {
    pub state: &'a mut AppState,
    pub cmds: &'a mut Commands,
    pub db_path: &'a Path,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Tab;

    fn id(&mut self, tab: &mut Tab) -> egui::Id {
        egui::Id::new(&*tab)
    }

    fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
        let count = match tab {
            Tab::Traffic => Some(self.state.traffic.flows.len()),
            Tab::Repeater => Some(self.state.repeater.tabs.len()),
            Tab::WebSockets => Some(self.state.websockets.connections.len()),
            Tab::Intercept => Some(self.state.intercept.pending.len()).filter(|&n| n > 0),
            _ => None,
        };
        match count {
            Some(n) => format!("{} ({})", tab.name(), n).into(),
            None => tab.name().into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        let state = &mut *self.state;
        let cmds = &mut *self.cmds;
        match tab {
            Tab::Traffic => features::traffic::show(ui, &mut state.traffic, &mut state.inspector, &state.proxy.listeners, &state.collections.trees, cmds),
            Tab::Repeater => {
                template::set_active_env(state.environments.repeater_env.as_ref());
                features::repeater::show(ui, &mut state.repeater, cmds);
                template::set_active_env(state.environments.active_environment());
            }
            Tab::Fuzzer => features::fuzzer::show(ui, &mut state.fuzzer, cmds),
            Tab::Intercept => features::intercept::show(ui, &mut state.intercept, cmds),
            Tab::Collections => features::collections::show(ui, &mut state.collections, cmds),
            Tab::WebSockets => features::websockets::show(ui, &mut state.websockets, cmds),
            Tab::Environments => features::environments::show(ui, &mut state.environments, cmds),
            Tab::Settings => features::settings::show(
                ui,
                &mut state.settings,
                &mut state.proxy,
                &mut state.certificate,
                &mut state.data,
                &mut state.database,
                &mut state.listener_form,
                &state.traffic,
                self.db_path,
                cmds,
            ),
        }
    }

    fn scroll_bars(&self, tab: &Tab) -> [bool; 2] {
        match tab {
            Tab::Collections => [false, true],
            _ => [false, false],
        }
    }
}
