use std::path::Path;
use eframe::egui;
use egui_dock::{DockState, NodeIndex, TabIndex};
use crate::command::Commands;
use crate::features;
use crate::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Tab {
    Traffic,
    Repeater,
    Fuzzer,
    Intercept,
    Collections,
    WebSockets,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 7] = [
        Tab::Traffic,
        Tab::Repeater,
        Tab::Fuzzer,
        Tab::Intercept,
        Tab::Collections,
        Tab::WebSockets,
        Tab::Settings,
    ];

    const TOOLS: [Tab; 6] = [
        Tab::Repeater,
        Tab::Fuzzer,
        Tab::Intercept,
        Tab::Collections,
        Tab::WebSockets,
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
            Tab::Settings => "Settings",
        }
    }
}

pub fn initial_layout() -> DockState<Tab> {
    DockState::new(vec![Tab::Traffic])
}

pub fn open_tab(dock: &mut DockState<Tab>, tab: Tab) {
    if let Some(path) = dock.find_tab(&tab) {
        let _ = dock.set_active_tab(path);
        return;
    }

    if tab == Tab::Traffic {
        dock.push_to_first_leaf(tab);
        return;
    }
    let tool_node = Tab::TOOLS.iter().find_map(|t| dock.find_main_surface_tab(t)).map(|(node, _)| node);
    if let Some(leaf) = tool_node.and_then(|node| dock.main_surface_mut().leaf_mut(node).ok()) {
        leaf.tabs.push(tab);
        leaf.active = TabIndex(leaf.tabs.len() - 1);
    } else if dock.find_main_surface_tab(&Tab::Traffic).is_some() {
        dock.main_surface_mut().split_right(NodeIndex::root(), 0.6, vec![tab]);
    } else {
        dock.push_to_first_leaf(tab);
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
            Tab::Traffic => features::traffic::show(ui, &mut state.traffic, &mut state.inspector, &state.proxy.listeners, cmds),
            Tab::Repeater => features::repeater::show(ui, &mut state.repeater, cmds),
            Tab::Fuzzer => features::fuzzer::show(ui, &mut state.fuzzer, cmds),
            Tab::Intercept => features::intercept::show(ui, &mut state.intercept, cmds),
            Tab::Collections => features::collections::show(ui, &mut state.collections, cmds),
            Tab::WebSockets => features::websockets::show(ui, &mut state.websockets, cmds),
            Tab::Settings => features::settings::show(
                ui,
                &mut state.proxy,
                &mut state.listener_form,
                &state.traffic,
                self.db_path,
                cmds,
            ),
        }
    }

    fn scroll_bars(&self, tab: &Tab) -> [bool; 2] {
        match tab {
            Tab::Collections | Tab::Intercept | Tab::Settings => [false, true],
            _ => [false, false],
        }
    }
}
