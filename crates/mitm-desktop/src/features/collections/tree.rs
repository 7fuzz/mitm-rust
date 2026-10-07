use std::borrow::Cow;
use std::collections::HashSet;
use eframe::egui::{self, collapsing_header::CollapsingState, RichText, Sense};
use mitm_core::collections::{CollectionTreeItem, RequestItem};
use crate::command::{AppCommand, Commands};
use crate::theme::{self, Tone};
use crate::widgets::pills::method_text;
use crate::widgets::search::search_field;
use super::{all_requests, CollectionsCmd, CollectionsState, TreeEdit};

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
    StartEdit(TreeEdit, String),
    CancelEdit,
    CommitEdit,
    ExtractRules(Vec<String>),
    Cmd(CollectionsCmd),
}

struct TreeUi<'a> {
    reveal: &'a HashSet<String>,
    edit: &'a Option<TreeEdit>,
    edit_text: &'a mut String,
    edit_focus: &'a mut bool,
    active_id: Option<&'a str>,
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
        reveal: &state.reveal,
        edit: &state.tree_edit,
        edit_text: &mut state.edit_text,
        edit_focus: &mut state.edit_focus,
        active_id: state.active_id.as_deref(),
        searching,
        actions: Vec::new(),
    };

    egui::ScrollArea::vertical().id_salt("collections_tree").auto_shrink([false, false]).show(ui, |ui| {
        if matches!(tree.edit, Some(TreeEdit::NewFolder { parent: None })) {
            tree.name_input(ui, "Folder name");
        }
        if visible.is_empty() && tree.edit.is_none() {
            ui.weak(if searching { "No matches" } else { "No collections yet" });
        }
        for folder in visible.iter() {
            tree.folder(ui, folder);
        }
        tree.root_drop_zone(ui);
    });

    floating_drag_label(ui);
    let actions = std::mem::take(&mut tree.actions);
    state.reveal.clear();
    for action in actions {
        apply(action, state, cmds);
    }
}

fn header(ui: &mut egui::Ui, state: &mut CollectionsState, _cmds: &mut Commands) {
    egui::Sides::new().show(
        ui,
        |ui| {
            ui.strong("Collections");
        },
        |ui| {
            if ui.button("➕").on_hover_text("New folder").clicked() {
                state.start_edit(TreeEdit::NewFolder { parent: None }, String::new());
            }
        },
    );

    search_field(ui, &mut state.search, "Search");
}

fn apply(action: TreeAction, state: &mut CollectionsState, cmds: &mut Commands) {
    match action {
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
    fn folder(&mut self, ui: &mut egui::Ui, folder: &CollectionTreeItem) {
        let id = ui.make_persistent_id(("collection_folder", &folder.id, self.searching));
        let mut collapsing = CollapsingState::load_with_default_open(ui.ctx(), id, true);
        let adding_here = matches!(self.edit,
            Some(TreeEdit::NewFolder { parent: Some(p) }) | Some(TreeEdit::NewRequest { collection_id: p }) if *p == folder.id);
        if self.searching || adding_here || self.reveal.contains(&folder.id) {
            collapsing.set_open(true);
        }

        let mut label = None;
        let mut header = collapsing.show_header(ui, |ui| {
            if matches!(self.edit, Some(TreeEdit::RenameFolder { id }) if *id == folder.id) {
                self.inline_rename(ui);
            } else {
                let text = RichText::new(format!("📁 {}", folder.name));
                label = Some(ui.add(egui::Label::new(text).truncate().selectable(false).sense(Sense::click_and_drag())));
            }
        });
        if let Some(response) = &label {
            if response.clicked() {
                header.toggle();
            }
            self.folder_interactions(response, folder);
        }

        header.body(|ui| {
            if matches!(self.edit, Some(TreeEdit::NewFolder { parent: Some(p) }) if *p == folder.id) {
                self.name_input(ui, "Folder name");
            }
            if matches!(self.edit, Some(TreeEdit::NewRequest { collection_id }) if *collection_id == folder.id) {
                self.name_input(ui, "Request name");
            }
            for child in &folder.children {
                self.folder(ui, child);
            }
            for request in &folder.requests {
                self.request(ui, request);
            }
        });
    }

    fn folder_interactions(&mut self, response: &egui::Response, folder: &CollectionTreeItem) {
        if response.dnd_hover_payload::<DragPayload>().is_some() {
            let stroke = response.ctx.global_style().visuals.selection.stroke;
            let painter = response.ctx.layer_painter(response.layer_id);
            painter.rect_stroke(response.rect.expand(2.0), 3.0, stroke, egui::StrokeKind::Outside);
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
    }

    fn request(&mut self, ui: &mut egui::Ui, request: &RequestItem) {
        if matches!(self.edit, Some(TreeEdit::RenameRequest { id }) if *id == request.id) {
            self.inline_rename(ui);
            return;
        }

        let mut text = egui::text::LayoutJob::default();
        let style = ui.style().clone();
        method_text(ui.visuals(), &request.method).append_to(&mut text, &style, egui::FontSelection::Default, egui::Align::Center);
        RichText::new(format!("  {}", request.name)).append_to(&mut text, &style, egui::FontSelection::Default, egui::Align::Center);
        if self.searching && !request.url.is_empty() {
            RichText::new(format!("  {}", request.url)).weak().append_to(&mut text, &style, egui::FontSelection::Default, egui::Align::Center);
        }

        let active = self.active_id == Some(request.id.as_str());
        let response = ui.add(
            egui::Button::selectable(active, text)
                .truncate()
                .sense(Sense::click_and_drag())
                .min_size(egui::vec2(ui.available_width(), 0.0)),
        );

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
        let request_ids = all_requests(std::slice::from_ref(folder)).iter().map(|r| r.id.clone()).collect();
        self.menu_items(ui, [
            ("New Request", TreeAction::StartEdit(TreeEdit::NewRequest { collection_id: id.clone() }, String::new())),
            ("New Subfolder", TreeAction::StartEdit(TreeEdit::NewFolder { parent: Some(id.clone()) }, String::new())),
            ("Auto-Extract Rules…", TreeAction::ExtractRules(request_ids)),
            ("Duplicate", TreeAction::Cmd(CollectionsCmd::DuplicateFolder(id.clone()))),
            ("Rename", TreeAction::StartEdit(TreeEdit::RenameFolder { id: id.clone() }, folder.name.clone())),
        ]);
        ui.separator();
        if ui.button(RichText::new("Delete Folder").color(theme::color(ui, Tone::Danger))).clicked() {
            self.actions.push(TreeAction::Cmd(CollectionsCmd::DeleteFolder(id)));
            ui.close();
        }
    }

    fn request_menu(&mut self, ui: &mut egui::Ui, request: &RequestItem) {
        let id = request.id.clone();
        self.menu_items(ui, [
            ("Open", TreeAction::Cmd(CollectionsCmd::Open(id.clone()))),
            ("Auto-Extract Rules…", TreeAction::ExtractRules(vec![id.clone()])),
            ("Duplicate", TreeAction::Cmd(CollectionsCmd::DuplicateRequest(id.clone()))),
            ("Rename", TreeAction::StartEdit(TreeEdit::RenameRequest { id: id.clone() }, request.name.clone())),
        ]);
        ui.separator();
        if ui.button(RichText::new("Delete Request").color(theme::color(ui, Tone::Danger))).clicked() {
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

    fn name_input(&mut self, ui: &mut egui::Ui, hint: &str) {
        let response = ui.add(egui::TextEdit::singleline(self.edit_text).hint_text(hint).desired_width(f32::INFINITY));
        self.take_focus(&response);
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.actions.push(TreeAction::CancelEdit);
        } else if response.lost_focus() {
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            self.actions.push(if enter { TreeAction::CommitEdit } else { TreeAction::CancelEdit });
        }
    }

    fn inline_rename(&mut self, ui: &mut egui::Ui) {
        let response = ui.add(egui::TextEdit::singleline(self.edit_text).desired_width(f32::INFINITY));
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
        let height = ui.available_height().max(48.0);
        let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
        if response.dnd_hover_payload::<DragPayload>().is_some() {
            ui.painter().rect_filled(rect, 4.0, ui.visuals().selection.bg_fill.gamma_multiply(0.15));
        }
        if let Some(payload) = response.dnd_release_payload::<DragPayload>() {
            let cmd = match &payload.item {
                DragItem::Folder { id } => CollectionsCmd::MoveFolder { id: id.clone(), parent: None },
                DragItem::Request { id, .. } => CollectionsCmd::MoveRequest { id: id.clone(), collection_id: None },
            };
            self.actions.push(TreeAction::Cmd(cmd));
        }
        response.context_menu(|ui| {
            if ui.button("New Folder").clicked() {
                self.actions.push(TreeAction::StartEdit(TreeEdit::NewFolder { parent: None }, String::new()));
                ui.close();
            }
        });
    }
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
