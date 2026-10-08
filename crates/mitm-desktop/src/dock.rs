use std::path::Path;
use eframe::egui;
use egui_dock::{DockState, NodePath, TabIndex};
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
    Rewrite,
    Collections,
    WebSockets,
    Environments,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 9] = [
        Tab::Traffic,
        Tab::Repeater,
        Tab::Fuzzer,
        Tab::Intercept,
        Tab::Rewrite,
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
            Tab::Rewrite => "Rewrite",
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

/// egui_dock's tab bar background only senses hover, so a right-click on its empty part goes to an area
/// registered here before the dock draws. Tabs are added later and still take their own clicks.
pub fn tab_bar_menus(ui: &mut egui::Ui, dock: &DockState<Tab>) -> Option<(NodePath, Tab)> {
    let mut picked = None;
    for (path, leaf) in dock.iter_leaves() {
        if !path.surface.is_main() || leaf.tab_bar_hidden || leaf.collapsed {
            continue;
        }
        let bar = egui::Rect::from_min_max(leaf.rect.min, egui::pos2(leaf.rect.max.x, leaf.viewport.min.y));
        if !bar.is_positive() {
            continue;
        }
        let response = ui.interact(bar, ui.id().with(("tab_bar_menu", path)), egui::Sense::click());
        response.context_menu(|ui| {
            for tab in Tab::ALL {
                if ui.selectable_label(leaf.tabs.contains(&tab), tab.name()).clicked() {
                    picked = Some((path, tab));
                    ui.close();
                }
            }
        });
    }
    picked
}

/// Opens `tab` in the leaf at `target`, moving it there if it's open in another leaf.
pub fn open_in_leaf(dock: &mut DockState<Tab>, target: NodePath, tab: Tab) {
    let anchor = dock.leaf(target).ok().and_then(|leaf| leaf.tabs.first().copied());
    if let Some(from) = dock.find_tab(&tab) {
        if from.surface == target.surface && from.node == target.node {
            let _ = dock.set_active_tab(from);
            return;
        }
        dock.remove_tab(from);
    }
    // Removing the tab can collapse its old leaf and renumber the tree, so find the target again by a tab it holds.
    let target = anchor
        .and_then(|anchor| dock.find_tab(&anchor))
        .map_or(target, |p| NodePath { surface: p.surface, node: p.node });
    if let Ok(leaf) = dock.leaf_mut(target) {
        leaf.append_tab(tab);
    }
    dock.set_focused_node_and_surface(target);
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
            Tab::Intercept => features::intercept::show(ui, &mut state.intercept, &state.proxy.listeners, cmds),
            Tab::Rewrite => features::rewrite::show(ui, &mut state.rewrite, &state.proxy.listeners, self.db_path, cmds),
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

#[cfg(test)]
mod tests {
    use super::*;
    use egui_dock::{NodeIndex, SurfaceIndex};

    fn leaf_of(dock: &DockState<Tab>, tab: Tab) -> NodePath {
        let p = dock.find_tab(&tab).unwrap();
        NodePath { surface: p.surface, node: p.node }
    }

    #[test]
    fn opening_into_a_leaf_adds_activates_or_moves() {
        let mut dock = DockState::new(vec![Tab::Traffic, Tab::Intercept]);
        dock.main_surface_mut().split_right(NodeIndex::root(), 0.5, vec![Tab::Repeater]);
        let left = leaf_of(&dock, Tab::Traffic);
        assert_eq!(left.surface, SurfaceIndex::main());

        open_in_leaf(&mut dock, left, Tab::Fuzzer);
        assert_eq!(leaf_of(&dock, Tab::Fuzzer), left);

        open_in_leaf(&mut dock, left, Tab::Traffic);
        assert_eq!(dock.leaf(left).unwrap().active, TabIndex(0));

        // Moving the right leaf's only tab collapses that leaf and renumbers the tree.
        open_in_leaf(&mut dock, left, Tab::Repeater);
        let traffic_leaf = leaf_of(&dock, Tab::Traffic);
        assert_eq!(leaf_of(&dock, Tab::Repeater), traffic_leaf);
        assert_eq!(dock.iter_leaves().count(), 1);
        let leaf = dock.leaf(traffic_leaf).unwrap();
        assert_eq!(leaf.tabs.len(), 4);
        assert_eq!(leaf.tabs[leaf.active.0], Tab::Repeater);
    }
}
