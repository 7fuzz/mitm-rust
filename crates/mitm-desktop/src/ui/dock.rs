use eframe::egui;
use egui_dock::{DockState, Node, NodePath, Split, TabIndex};
use crate::app::command::Commands;
use crate::features;
use crate::widgets::template;
use crate::app::state::AppState;
use crate::ui::theme;
use crate::util::format::format_count;
use mitm_core::db::Db;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Tab {
    Traffic,
    Repeater,
    Fuzzer,
    Intercept,
    Rewrite,
    Collections,
    WebSockets,
    Webhooks,
    Containers,
    Environments,
    Credentials,
    Settings,
    Encoding,
    Hashing,
    HmacTool,
    JsonDiff,
    JsonTools,
    Markdown,
    Cvss,
    Totp,
    DbViewer,
    /// The traffic inspector detached from Traffic; opened from the inspector, not from panel menus.
    Inspector,
}

impl Tab {
    pub const ALL: [Tab; 21] = [
        Tab::Traffic,
        Tab::Repeater,
        Tab::Fuzzer,
        Tab::Intercept,
        Tab::Rewrite,
        Tab::Collections,
        Tab::WebSockets,
        Tab::Webhooks,
        Tab::Containers,
        Tab::Environments,
        Tab::Credentials,
        Tab::Settings,
        Tab::Encoding,
        Tab::Hashing,
        Tab::HmacTool,
        Tab::JsonDiff,
        Tab::JsonTools,
        Tab::Markdown,
        Tab::Cvss,
        Tab::Totp,
        Tab::DbViewer,
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
            Tab::Webhooks => "Webhooks",
            Tab::Containers => "Containers",
            Tab::Environments => "Environments",
            Tab::Credentials => "Credentials",
            Tab::Settings => "Settings",
            Tab::Encoding => "Encoding tool",
            Tab::Hashing => "Hashing tool",
            Tab::HmacTool => "HMAC signature",
            Tab::JsonDiff => "JSON Diff",
            Tab::JsonTools => "JSON Tools",
            Tab::Markdown => "Markdown",
            Tab::Cvss => "CVSS",
            Tab::Totp => "TOTP",
            Tab::DbViewer => "DB Viewer",
            Tab::Inspector => "Inspector",
        }
    }

    /// The non-utility tabs as menus list them, a separator between groups.
    pub const GROUPS: &'static [&'static [Tab]] = &[
        &[Tab::Traffic, Tab::Intercept, Tab::Repeater, Tab::Rewrite, Tab::Fuzzer],
        &[Tab::WebSockets, Tab::Webhooks, Tab::Containers],
        &[Tab::Collections, Tab::Environments, Tab::Credentials],
        &[Tab::Settings],
    ];

    /// Utility tabs, in the order they appear under the **Utilities ▸** submenu.
    pub const UTILITIES: &'static [Tab] = &[
        Tab::JsonDiff,
        Tab::JsonTools,
        Tab::Markdown,
        Tab::Cvss,
        Tab::Encoding,
        Tab::Hashing,
        Tab::HmacTool,
        Tab::Totp,
        Tab::DbViewer,
    ];

    pub fn is_utility(&self) -> bool {
        Self::UTILITIES.contains(self)
    }
}

pub fn initial_layout() -> DockState<Tab> {
    DockState::new(vec![Tab::Traffic, Tab::Intercept, Tab::Repeater])
}

/// Moves the inspector into its own panel beside Traffic, or back into Traffic.
pub fn detach_inspector(dock: &mut DockState<Tab>, detach: bool) {
    match (dock.find_tab(&Tab::Inspector), detach) {
        (Some(path), true) => {
            let _ = dock.set_active_tab(path);
        }
        (Some(path), false) => {
            dock.remove_tab(path);
        }
        (None, true) => match dock.find_tab(&Tab::Traffic) {
            Some(traffic) => {
                let beside = NodePath { surface: traffic.surface, node: traffic.node };
                dock.split(beside, Split::Right, 0.5, Node::leaf(Tab::Inspector));
            }
            None => dock.push_to_focused_leaf(Tab::Inspector),
        },
        (None, false) => {}
    }
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
            if let Some(tab) = panel_menu(ui, |tab| leaf.tabs.contains(&tab)) {
                picked = Some((path, tab));
            }
        });
    }
    picked
}

/// Every tab in its group, then Utilities ▸; `is_open` ticks the ones already shown. Returns the clicked tab.
pub fn panel_menu(ui: &mut egui::Ui, is_open: impl Fn(Tab) -> bool) -> Option<Tab> {
    let mut picked = None;
    let mut item = |ui: &mut egui::Ui, tab: Tab| {
        if ui.selectable_label(is_open(tab), tab.name()).clicked() {
            picked = Some(tab);
            ui.close();
        }
    };
    for group in Tab::GROUPS {
        for &tab in *group {
            item(ui, tab);
        }
        ui.separator();
    }
    ui.menu_button("Utilities", |ui| {
        for &tab in Tab::UTILITIES {
            item(ui, tab);
        }
    });
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
    pub db: &'a Db,
    pub focus_search: Option<Tab>,
    pub focused: Option<Tab>,
    pub visuals: egui::Visuals,
}

impl TabViewer<'_> {
    fn draw(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        let state = &mut *self.state;
        let cmds = &mut *self.cmds;
        match tab {
            Tab::Traffic => features::traffic::show(ui, &mut state.traffic, &mut state.inspector, &state.proxy.listeners, &state.collections.trees, cmds),
            Tab::Repeater => {
                template::set_active_env(state.environments.repeater_env.as_ref());
                features::repeater::show(ui, &mut state.repeater, cmds);
                template::set_active_env(state.environments.active_environment());
            }
            Tab::Inspector => {
                if state.traffic.selected_flow().is_some() {
                    features::inspector::show(ui, &mut state.inspector, &state.traffic, &state.collections.trees, cmds);
                } else {
                    ui.centered_and_justified(|ui| ui.weak("Select a request in Traffic"));
                }
            }
            Tab::Fuzzer => features::fuzzer::show(ui, &mut state.fuzzer, cmds),
            Tab::Intercept => features::intercept::show(ui, &mut state.intercept, &state.proxy.listeners, &state.traffic.containers, cmds),
            Tab::Rewrite => features::rewrite::show(ui, &mut state.rewrite, &state.proxy.listeners, &state.traffic.containers, self.db, cmds),
            Tab::Collections => features::collections::show(ui, &mut state.collections, cmds),
            Tab::WebSockets => features::websockets::show(ui, &mut state.websockets, cmds),
            Tab::Webhooks => features::webhooks::show(ui, &mut state.webhooks, cmds),
            Tab::Containers => features::containers::show(ui, &mut state.containers, cmds),
            Tab::Encoding => features::utilities::encoding::show(ui, &mut state.encoding),
            Tab::Hashing => features::utilities::hashing::show(ui, &mut state.hashing, cmds),
            Tab::HmacTool => features::utilities::hmac_tool::show(ui, &mut state.hmac_tool),
            Tab::JsonDiff => features::utilities::json_diff::show(ui, &mut state.json_diff),
            Tab::JsonTools => features::utilities::json_tools::show(ui, &mut state.json_tools),
            Tab::Markdown => features::utilities::markdown::show(ui, &mut state.markdown, cmds),
            Tab::Cvss => features::utilities::cvss::show(ui, &mut state.cvss),
            Tab::Totp => features::utilities::totp::show(ui, &mut state.totp, cmds),
            Tab::DbViewer => features::utilities::dbviewer::show(ui, &mut state.dbviewer, self.db, || state.credentials.mark_stale()),
            Tab::Environments => features::environments::show(ui, &mut state.environments, &state.credentials.linked, cmds),
            Tab::Credentials => {
                features::credentials::show(ui, &mut state.credentials, &state.environments.active_workspace_id, &state.environments.workspaces, self.db, cmds)
            }
            Tab::Settings => features::settings::show(
                ui,
                &mut state.settings,
                &mut state.proxy,
                &mut state.certificate,
                &mut state.data,
                &mut state.database,
                &mut state.listener_form,
                &state.traffic,
                &mut state.credentials,
                self.db,
                cmds,
            ),
        }
    }
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Tab;

    fn id(&mut self, tab: &mut Tab) -> egui::Id {
        egui::Id::new(*tab)
    }

    fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
        let count = match tab {
            Tab::Traffic => usize::try_from(self.state.traffic.stored_count).ok(),
            Tab::Repeater => Some(self.state.repeater.tabs.len()),
            Tab::WebSockets => Some(self.state.websockets.connections.len()),
            Tab::Intercept => Some(self.state.intercept.pending.len()).filter(|&n| n > 0),
            _ => None,
        };
        let Some(count) = count else { return tab.name().into() };
        let count_color = match tab {
            Tab::Intercept => theme::Tone::Warn.color(&self.visuals),
            _ => self.visuals.weak_text_color(),
        };
        let mut job = egui::text::LayoutJob::default();
        // Placeholder colour lets the dock paint the name in its active/inactive tab colour.
        let name = egui::TextFormat::simple(egui::FontId::proportional(theme::BODY_SIZE), egui::Color32::PLACEHOLDER);
        job.append(tab.name(), 0.0, name);
        let count_format = egui::TextFormat::simple(egui::FontId::monospace(theme::SMALL_SIZE), count_color);
        job.append(&format_count(count), 6.0, count_format);
        job.into()
    }

    fn on_tab_button(&mut self, tab: &mut Tab, response: &egui::Response) {
        if self.focused == Some(*tab) {
            let rect = response.rect;
            let accent = self.visuals.selection.stroke.color;
            response.ctx.layer_painter(response.layer_id).hline(rect.x_range(), rect.bottom() - 1.0, egui::Stroke::new(2.0, accent));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        crate::widgets::search::arm(ui.ctx(), self.focus_search == Some(*tab));
        self.draw(ui, tab);
        crate::widgets::search::arm(ui.ctx(), false);
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

    #[test]
    fn groups_list_every_non_utility_tab_once() {
        let grouped: Vec<Tab> = Tab::GROUPS.iter().flat_map(|g| g.iter().copied()).collect();
        let expected: Vec<Tab> = Tab::ALL.into_iter().filter(|t| !t.is_utility()).collect();
        assert_eq!(grouped.len(), expected.len());
        assert!(expected.iter().all(|t| grouped.contains(t)));
    }
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

#[cfg(test)]
mod render_tests {
    use super::*;
    use egui_dock::TabViewer as _;

    /// Draws the shell and every tab on an empty database, in both themes, to catch panics in view code.
    #[test]
    fn every_tab_and_the_shell_render_in_both_themes() {
        let dir = std::env::temp_dir().join(format!("mitm-render-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::new(dir.join("test.db"));
        mitm_core::db::init_database(&db, None).unwrap();
        let mut state = AppState::load(&db);
        let dock = DockState::new(vec![Tab::Traffic]);
        let ctx = egui::Context::default();
        crate::ui::theme::install_fonts(&ctx);
        crate::ui::theme::apply_style(&ctx);
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            ..Default::default()
        };

        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            ctx.set_theme(theme);
            // The second frame lays out with sizes measured in the first.
            for _ in 0..2 {
                let mut cmds = Commands::default();
                let mut output = ctx.run_ui(input(), |ui| {
                    crate::ui::top_bar::show(ui, &mut state, &dock, &mut cmds);
                    crate::ui::status_bar::show(ui, &state);
                    for mut tab in Tab::ALL {
                        let visuals = ui.visuals().clone();
                        let mut viewer = TabViewer { state: &mut state, cmds: &mut cmds, db: &db, focus_search: None, focused: None, visuals };
                        ui.push_id(tab, |ui| viewer.ui(ui, &mut tab));
                    }
                });
                output.textures_delta.clear();
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Side panels grow to fit their content, so content that is wider than its space every frame makes
    /// the sidebar creep wider on each repaint.
    #[test]
    fn sidebars_keep_their_width_across_frames() {
        let dir = std::env::temp_dir().join(format!("mitm-sidebar-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::new(dir.join("test.db"));
        mitm_core::db::init_database(&db, None).unwrap();
        let workspace = mitm_core::workspace::create_workspace_db(&db, "W".into(), None).unwrap();
        let folder = mitm_core::collections::create_collection_db(&db, workspace.id.clone(), None, "Folder".into()).unwrap();
        mitm_core::collections::create_request_db(&db, folder.id, "Request".into()).unwrap();
        let mut state = AppState::load(&db);
        state.collections.load(&db, workspace.id);
        state.repeater.create_blank(&db);
        let ctx = egui::Context::default();
        crate::ui::theme::install_fonts(&ctx);
        crate::ui::theme::apply_style(&ctx);

        for (mut tab, panel) in [(Tab::Repeater, "repeater_sidebar"), (Tab::Collections, "collections_sidebar")] {
            let mut widths = Vec::new();
            for frame in 0..6 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
                    events: vec![egui::Event::PointerMoved(egui::pos2(100.0 + frame as f32 * 40.0, 300.0))],
                    ..Default::default()
                };
                let mut cmds = Commands::default();
                let mut output = ctx.run_ui(input, |ui| {
                    let visuals = ui.visuals().clone();
                    let mut viewer = TabViewer { state: &mut state, cmds: &mut cmds, db: &db, focus_search: None, focused: None, visuals };
                    viewer.ui(ui, &mut tab);
                });
                output.textures_delta.clear();
                widths.push(egui::containers::panel::PanelState::load(&ctx, egui::Id::new(panel)).unwrap().size().x);
            }
            assert!(widths[1..].windows(2).all(|w| (w[0] - w[1]).abs() < 0.01), "{panel} width changed: {widths:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
