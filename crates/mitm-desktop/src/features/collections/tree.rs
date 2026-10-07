use std::borrow::Cow;
use std::collections::HashSet;
use eframe::egui::{self, RichText, Sense};
use mitm_core::collections::{CollectionTreeItem, RequestItem};
use crate::command::{AppCommand, Commands};
use crate::widgets::pills::{render_method_pill, AMBER, ROSE};
use super::{all_requests, CollectionsCmd, CollectionsState, TreeEdit};

const ROW_HEIGHT: f32 = 24.0;
const INDENT: f32 = 14.0;

#[derive(Clone)]
enum DragItem {
    Folder { id: String },
    Request { id: String, collection_id: String },
}

#[derive(Clone)]
struct DragPayload {
    item: DragItem,
    label: String,
}

enum TreeAction {
    Toggle(String),
    StartEdit(TreeEdit, String),
    CancelEdit,
    CommitEdit,
    ExtractRules(Vec<String>),
    Cmd(CollectionsCmd),
}

struct TreeUi<'a> {
    collapsed: &'a HashSet<String>,
    edit: &'a Option<TreeEdit>,
    edit_text: &'a mut String,
    edit_focus: &'a mut bool,
    searching: bool,
    actions: Vec<TreeAction>,
}

pub fn show(ui: &mut egui::Ui, state: &mut CollectionsState, cmds: &mut Commands) {
    header(ui, state, cmds);
    ui.separator();

    let searching = !state.search.trim().is_empty();
    let visible: Cow<[CollectionTreeItem]> = if searching {
        Cow::Owned(filter_tree(&state.trees, &state.search))
    } else {
        Cow::Borrowed(&state.trees)
    };
    let mut tree = TreeUi {
        collapsed: &state.collapsed,
        edit: &state.tree_edit,
        edit_text: &mut state.edit_text,
        edit_focus: &mut state.edit_focus,
        searching,
        actions: Vec::new(),
    };

    egui::ScrollArea::vertical().id_salt("collections_tree").auto_shrink([false, false]).show(ui, |ui| {
        if matches!(tree.edit, Some(TreeEdit::NewFolder { parent: None })) {
            tree.name_input(ui, 0, "Root folder name...");
        }

        if visible.is_empty() && searching {
            empty_state(ui, &format!("No collections or requests match “{}”", state.search.trim()));
            if ui.vertical_centered(|ui| ui.link("Clear search").clicked()).inner {
                state.search.clear();
            }
        } else if visible.is_empty() && tree.edit.is_none() {
            empty_state(ui, "No collections created yet");
            if ui.vertical_centered(|ui| ui.button("➕ Create Root Folder").clicked()).inner {
                tree.actions.push(TreeAction::StartEdit(TreeEdit::NewFolder { parent: None }, String::new()));
            }
        }

        for folder in visible.iter() {
            tree.folder(ui, folder, 0);
        }
        tree.root_drop_zone(ui);
    });

    floating_drag_label(ui);
    let actions = std::mem::take(&mut tree.actions);
    for action in actions {
        apply(action, state, cmds);
    }
}

fn header(ui: &mut egui::Ui, state: &mut CollectionsState, cmds: &mut Commands) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("📁").color(AMBER));
        ui.label(RichText::new("Collections").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("➕ Folder").on_hover_text("Create root folder").clicked() {
                state.start_edit(TreeEdit::NewFolder { parent: None }, String::new());
            }
        });
    });

    if state.workspaces.len() > 1 {
        let current = state
            .workspaces
            .iter()
            .find(|w| w.id == state.active_workspace_id)
            .map_or("Workspace".to_string(), |w| w.name.clone());
        egui::ComboBox::from_id_salt("collections_workspace")
            .selected_text(current)
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                for ws in &state.workspaces {
                    if ui.selectable_label(ws.id == state.active_workspace_id, &ws.name).clicked() {
                        cmds.push(AppCommand::Collections(CollectionsCmd::SelectWorkspace(ws.id.clone())));
                    }
                }
            });
    }

    ui.horizontal(|ui| {
        let width = ui.available_width() - if state.search.is_empty() { 0.0 } else { 24.0 };
        ui.add(egui::TextEdit::singleline(&mut state.search).hint_text("🔍 Search collections & requests...").desired_width(width));
        if !state.search.is_empty() && ui.small_button("✖").clicked() {
            state.search.clear();
        }
    });
}

fn empty_state(ui: &mut egui::Ui, text: &str) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| ui.label(RichText::new(text).weak().italics()));
}

fn apply(action: TreeAction, state: &mut CollectionsState, cmds: &mut Commands) {
    match action {
        TreeAction::Toggle(id) => {
            if !state.collapsed.remove(&id) {
                state.collapsed.insert(id);
            }
        }
        TreeAction::StartEdit(edit, text) => state.start_edit(edit, text),
        TreeAction::CancelEdit => state.tree_edit = None,
        TreeAction::CommitEdit => {
            let name = state.edit_text.trim().to_string();
            if let (Some(edit), false) = (state.tree_edit.take(), name.is_empty()) {
                let cmd = match edit {
                    TreeEdit::NewFolder { parent } => CollectionsCmd::CreateFolder { parent, name },
                    TreeEdit::NewRequest { collection_id } => CollectionsCmd::CreateRequest { collection_id, name },
                    TreeEdit::RenameFolder { id } => CollectionsCmd::RenameFolder { id, name },
                    TreeEdit::RenameRequest { id } => CollectionsCmd::RenameRequest { id, name },
                };
                cmds.push(AppCommand::Collections(cmd));
            }
        }
        TreeAction::ExtractRules(ids) => state.open_extract_modal(ids),
        TreeAction::Cmd(cmd) => cmds.push(AppCommand::Collections(cmd)),
    }
}

impl TreeUi<'_> {
    fn folder(&mut self, ui: &mut egui::Ui, folder: &CollectionTreeItem, depth: usize) {
        let expanded = self.searching || !self.collapsed.contains(&folder.id);
        let renaming = matches!(self.edit, Some(TreeEdit::RenameFolder { id }) if *id == folder.id);

        let (rect, response) = row(ui);
        let dropping = response.dnd_hover_payload::<DragPayload>().is_some();
        if dropping {
            ui.painter().rect_stroke(rect, 3.0, ui.visuals().selection.stroke, egui::StrokeKind::Inside);
        }

        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(egui::vec2(4.0, 0.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
        child.add_space(depth as f32 * INDENT);
        child.label(RichText::new(if expanded { "📂" } else { "📁" }).color(AMBER));
        if renaming {
            self.inline_edit(&mut child);
        } else {
            child.add(egui::Label::new(RichText::new(&folder.name).strong()).truncate().selectable(false));
            if response.hovered() || child.ui_contains_pointer() {
                child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(RichText::new("✖").color(ROSE)).on_hover_text("Delete folder").clicked() {
                        self.actions.push(TreeAction::Cmd(CollectionsCmd::DeleteFolder(folder.id.clone())));
                    }
                    if ui.small_button("➕").on_hover_text("Add request").clicked() {
                        self.actions.push(TreeAction::StartEdit(TreeEdit::NewRequest { collection_id: folder.id.clone() }, String::new()));
                    }
                    if ui.small_button("📁").on_hover_text("Add subfolder").clicked() {
                        self.actions.push(TreeAction::StartEdit(TreeEdit::NewFolder { parent: Some(folder.id.clone()) }, String::new()));
                    }
                });
            }
        }

        if response.clicked() {
            self.actions.push(TreeAction::Toggle(folder.id.clone()));
        }
        if response.drag_started() {
            response.dnd_set_drag_payload(DragPayload {
                item: DragItem::Folder { id: folder.id.clone() },
                label: folder.name.clone(),
            });
        }
        if let Some(payload) = response.dnd_release_payload::<DragPayload>() {
            let cmd = match &payload.item {
                DragItem::Folder { id } if *id != folder.id => {
                    Some(CollectionsCmd::MoveFolder { id: id.clone(), parent: Some(folder.id.clone()) })
                }
                DragItem::Request { id, collection_id } if *collection_id != folder.id => {
                    Some(CollectionsCmd::MoveRequest { id: id.clone(), collection_id: Some(folder.id.clone()) })
                }
                _ => None,
            };
            self.actions.extend(cmd.map(TreeAction::Cmd));
        }
        response.context_menu(|ui| self.folder_menu(ui, folder));

        if !expanded {
            return;
        }
        if matches!(self.edit, Some(TreeEdit::NewFolder { parent: Some(id) }) if *id == folder.id) {
            self.name_input(ui, depth + 1, "Subfolder name...");
        }
        if matches!(self.edit, Some(TreeEdit::NewRequest { collection_id }) if *collection_id == folder.id) {
            self.name_input(ui, depth + 1, "Request name...");
        }
        for child_folder in &folder.children {
            self.folder(ui, child_folder, depth + 1);
        }
        for request in &folder.requests {
            self.request(ui, request, depth + 1);
        }
    }

    fn request(&mut self, ui: &mut egui::Ui, request: &RequestItem, depth: usize) {
        let renaming = matches!(self.edit, Some(TreeEdit::RenameRequest { id }) if *id == request.id);
        let (rect, response) = row(ui);

        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(egui::vec2(4.0, 0.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
        child.add_space(depth as f32 * INDENT);
        render_method_pill(&mut child, &request.method);
        if renaming {
            self.inline_edit(&mut child);
        } else {
            let text = if self.searching && !request.url.is_empty() {
                RichText::new(format!("{}  {}", request.name, request.url))
            } else {
                RichText::new(&request.name)
            };
            child.add(egui::Label::new(text).truncate().selectable(false));
            if response.hovered() || child.ui_contains_pointer() {
                child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(RichText::new("✖").color(ROSE)).on_hover_text("Delete request").clicked() {
                        self.actions.push(TreeAction::Cmd(CollectionsCmd::DeleteRequest(request.id.clone())));
                    }
                });
            }
        }

        if response.clicked() {
            self.actions.push(TreeAction::Cmd(CollectionsCmd::Open(request.id.clone())));
        }
        if response.drag_started() {
            response.dnd_set_drag_payload(DragPayload {
                item: DragItem::Request { id: request.id.clone(), collection_id: request.collection_id.clone() },
                label: request.name.clone(),
            });
        }
        response.context_menu(|ui| self.request_menu(ui, request));
    }

    fn folder_menu(&mut self, ui: &mut egui::Ui, folder: &CollectionTreeItem) {
        let id = folder.id.clone();
        let items: [(&str, TreeAction); 5] = [
            ("➕ Add Request", TreeAction::StartEdit(TreeEdit::NewRequest { collection_id: id.clone() }, String::new())),
            ("📁 Add Subfolder", TreeAction::StartEdit(TreeEdit::NewFolder { parent: Some(id.clone()) }, String::new())),
            ("⚡ Auto-Extract Rules", TreeAction::ExtractRules(all_requests(std::slice::from_ref(folder)).iter().map(|r| r.id.clone()).collect())),
            ("📋 Duplicate Folder", TreeAction::Cmd(CollectionsCmd::DuplicateFolder(id.clone()))),
            ("✏ Rename Folder", TreeAction::StartEdit(TreeEdit::RenameFolder { id: id.clone() }, folder.name.clone())),
        ];
        self.menu_items(ui, items);
        ui.separator();
        if ui.button(RichText::new("✖ Delete Folder").color(ROSE)).clicked() {
            self.actions.push(TreeAction::Cmd(CollectionsCmd::DeleteFolder(id)));
            ui.close();
        }
    }

    fn request_menu(&mut self, ui: &mut egui::Ui, request: &RequestItem) {
        let id = request.id.clone();
        let items: [(&str, TreeAction); 4] = [
            ("👁 Open in Tab", TreeAction::Cmd(CollectionsCmd::Open(id.clone()))),
            ("⚡ Auto-Extract Rules", TreeAction::ExtractRules(vec![id.clone()])),
            ("📋 Duplicate Request", TreeAction::Cmd(CollectionsCmd::DuplicateRequest(id.clone()))),
            ("✏ Rename Request", TreeAction::StartEdit(TreeEdit::RenameRequest { id: id.clone() }, request.name.clone())),
        ];
        self.menu_items(ui, items);
        ui.separator();
        if ui.button(RichText::new("✖ Delete Request").color(ROSE)).clicked() {
            self.actions.push(TreeAction::Cmd(CollectionsCmd::DeleteRequest(id)));
            ui.close();
        }
    }

    fn menu_items<const N: usize>(&mut self, ui: &mut egui::Ui, items: [(&str, TreeAction); N]) {
        for (label, action) in items {
            if ui.button(label).clicked() {
                self.actions.push(action);
                ui.close();
            }
        }
    }

    fn name_input(&mut self, ui: &mut egui::Ui, depth: usize, hint: &str) {
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * INDENT + 4.0);
            let response = ui.add(egui::TextEdit::singleline(self.edit_text).hint_text(hint).desired_width(ui.available_width() - 56.0));
            self.take_focus(&response);
            let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if enter || ui.small_button("✔").on_hover_text("Save").clicked() {
                self.actions.push(TreeAction::CommitEdit);
            }
            if ui.small_button("✖").on_hover_text("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.actions.push(TreeAction::CancelEdit);
            }
        });
    }

    fn inline_edit(&mut self, ui: &mut egui::Ui) {
        let response = ui.add(egui::TextEdit::singleline(self.edit_text).desired_width(ui.available_width()));
        self.take_focus(&response);
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.actions.push(TreeAction::CancelEdit);
        } else if response.lost_focus() {
            self.actions.push(TreeAction::CommitEdit);
        }
    }

    fn take_focus(&mut self, response: &egui::Response) {
        if *self.edit_focus {
            response.request_focus();
            *self.edit_focus = false;
        }
    }

    fn root_drop_zone(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height().max(ROW_HEIGHT * 2.0);
        let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
        if response.dnd_hover_payload::<DragPayload>().is_some() {
            ui.painter().rect_filled(rect, 4.0, ui.visuals().selection.bg_fill.gamma_multiply(0.15));
            ui.painter().text(
                rect.center_top() + egui::vec2(0.0, 14.0),
                egui::Align2::CENTER_CENTER,
                "Drop here to move to the root level",
                egui::FontId::proportional(12.0),
                ui.visuals().selection.stroke.color,
            );
        }
        if let Some(payload) = response.dnd_release_payload::<DragPayload>() {
            let cmd = match &payload.item {
                DragItem::Folder { id } => CollectionsCmd::MoveFolder { id: id.clone(), parent: None },
                DragItem::Request { id, .. } => CollectionsCmd::MoveRequest { id: id.clone(), collection_id: None },
            };
            self.actions.push(TreeAction::Cmd(cmd));
        }
        response.context_menu(|ui| {
            if ui.button("📁 New Root Folder").clicked() {
                self.actions.push(TreeAction::StartEdit(TreeEdit::NewFolder { parent: None }, String::new()));
                ui.close();
            }
        });
    }
}

fn row(ui: &mut egui::Ui) -> (egui::Rect, egui::Response) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), Sense::click_and_drag());
    if response.hovered() {
        ui.painter().rect_filled(rect, 3.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    (rect, response)
}

fn floating_drag_label(ui: &egui::Ui) {
    let Some(payload) = egui::DragAndDrop::payload::<DragPayload>(ui.ctx()) else { return };
    let Some(pos) = ui.ctx().pointer_interact_pos() else { return };
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    egui::Area::new(egui::Id::new("collections_drag_label"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos + egui::vec2(12.0, 8.0))
        .interactable(false)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| ui.label(&payload.label));
        });
}

fn filter_tree(items: &[CollectionTreeItem], query: &str) -> Vec<CollectionTreeItem> {
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if terms.is_empty() {
        return items.to_vec();
    }
    let matches = |text: String| {
        let text = text.to_lowercase();
        terms.iter().all(|t| text.contains(t))
    };
    items
        .iter()
        .filter_map(|item| {
            if matches(format!("{} {}", item.name, item.description.clone().unwrap_or_default())) {
                return Some(item.clone());
            }
            let children = filter_tree(&item.children, query);
            let requests: Vec<RequestItem> = item
                .requests
                .iter()
                .filter(|r| matches(format!("{} {} {} {}", r.name, r.url, r.method, r.description.clone().unwrap_or_default())))
                .cloned()
                .collect();
            (!children.is_empty() || !requests.is_empty()).then(|| CollectionTreeItem { children, requests, ..item.clone() })
        })
        .collect()
}
