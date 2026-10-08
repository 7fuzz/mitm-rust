mod docs;
mod editor;
mod extract_modal;
mod tree;
mod view;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use mitm_core::collections::execute::ExecutionResult;
use mitm_core::collections::preview::{preview_request_db, RequestPreview};
use mitm_core::collections::{self as core_collections, Collection, CollectionTreeItem, RequestHistoryItem, RequestItem};
use mitm_core::repeater::ExtractRuleItem;
use crate::widgets::message_pane::PaneState;

pub use view::show;

const SAVE_DELAY_SECS: f64 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTab {
    Docs,
    Params,
    Headers,
    Body,
    Extract,
    Steps,
    Interpolation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocsMode {
    Preview,
    Edit,
    Split,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewMode {
    Request,
    FullUrl,
    Curl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeEdit {
    NewFolder { parent: Option<String> },
    NewRequest { collection_id: String },
    RenameFolder { id: String },
    RenameRequest { id: String },
}

pub struct ExtractModal {
    pub request_ids: Vec<String>,
    pub selected: String,
    pub rules: Vec<ExtractRuleItem>,
}

#[derive(Clone)]
pub enum CollectionsCmd {
    SelectWorkspace(String),
    CreateFolder { parent: Option<String>, name: String },
    CreateRequest { collection_id: String, name: String },
    RenameFolder { id: String, name: String },
    RenameRequest { id: String, name: String },
    DeleteFolder(String),
    DeleteRequest(String),
    DuplicateFolder(String),
    DuplicateRequest(String),
    MoveFolder { id: String, parent: Option<String> },
    MoveRequest { id: String, collection_id: Option<String> },
    Open(String),
    Activate(String),
    Close(String),
    Execute(String),
    RefreshPreview(String),
    ClearRuns(String),
    SaveExtractRules { request_id: String, rules: Vec<ExtractRuleItem> },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CollectionsPrefs {
    pub split: f32,
    pub history_open: bool,
}

impl Default for CollectionsPrefs {
    fn default() -> Self {
        Self { split: 0.5, history_open: false }
    }
}

pub struct CollectionsState {
    pub prefs: CollectionsPrefs,
    pub active_workspace_id: String,
    pub trees: Vec<CollectionTreeItem>,
    pub search: String,
    pub reveal: HashSet<String>,
    pub tree_edit: Option<TreeEdit>,
    pub edit_text: String,
    pub edit_focus: bool,
    pub open: Vec<RequestItem>,
    pub active_id: Option<String>,
    pub editor_tab: EditorTab,
    pub docs_mode: DocsMode,
    pub preview_mode: PreviewMode,
    pub previews: HashMap<String, Result<RequestPreview, String>>,
    pub executing: HashSet<String>,
    pub latest: HashMap<String, ExecutionResult>,
    pub runs: HashMap<String, Vec<RequestHistoryItem>>,
    pub selected_run: HashMap<String, i64>,
    pub response_pane: PaneState,
    pub extract_modal: Option<ExtractModal>,
    dirty: HashSet<String>,
    last_edit: f64,
}

impl Default for CollectionsState {
    fn default() -> Self {
        Self {
            prefs: CollectionsPrefs::default(),
            active_workspace_id: String::new(),
            trees: Vec::new(),
            search: String::new(),
            reveal: HashSet::new(),
            tree_edit: None,
            edit_text: String::new(),
            edit_focus: false,
            open: Vec::new(),
            active_id: None,
            editor_tab: EditorTab::Params,
            docs_mode: DocsMode::Preview,
            preview_mode: PreviewMode::Request,
            previews: HashMap::new(),
            executing: HashSet::new(),
            latest: HashMap::new(),
            runs: HashMap::new(),
            selected_run: HashMap::new(),
            response_pane: PaneState::default(),
            extract_modal: None,
            dirty: HashSet::new(),
            last_edit: 0.0,
        }
    }
}

impl CollectionsState {
    pub fn load(&mut self, db_path: &PathBuf, workspace_id: String) {
        self.active_workspace_id = workspace_id;
        self.reload_tree(db_path);
    }

    pub fn reload_tree(&mut self, db_path: &PathBuf) {
        self.flush(db_path);
        self.trees = core_collections::get_collections_db(db_path, &self.active_workspace_id).unwrap_or_default();
        let saved: HashMap<String, RequestItem> = all_requests(&self.trees).into_iter().map(|r| (r.id.clone(), r.clone())).collect();
        self.open.retain(|r| saved.contains_key(&r.id));
        for request in &mut self.open {
            *request = saved[&request.id].clone();
        }
        if self.active_id.as_ref().is_some_and(|id| !saved.contains_key(id)) {
            self.active_id = self.open.first().map(|r| r.id.clone());
        }
    }

    pub fn find_request(&self, id: &str) -> Option<&RequestItem> {
        all_requests(&self.trees).into_iter().find(|r| r.id == id)
    }

    pub fn mark_dirty(&mut self, id: &str, now: f64) {
        self.dirty.insert(id.to_string());
        self.last_edit = now;
    }

    pub fn flush_if_idle(&mut self, db_path: &PathBuf, now: f64) {
        if !self.dirty.is_empty() && now - self.last_edit >= SAVE_DELAY_SECS {
            self.flush(db_path);
        }
    }

    pub fn flush(&mut self, db_path: &PathBuf) {
        for id in std::mem::take(&mut self.dirty) {
            let Some(request) = self.open.iter().find(|r| r.id == id).cloned() else { continue };
            if core_collections::update_request_db(db_path, request.clone()).is_ok() {
                patch_tree(&mut self.trees, request);
            }
        }
    }

    pub fn apply(&mut self, cmd: CollectionsCmd, db_path: &PathBuf) {
        let ws = self.active_workspace_id.clone();
        match cmd {
            CollectionsCmd::SelectWorkspace(id) => {
                self.flush(db_path);
                self.active_workspace_id = id;
                self.open.clear();
                self.active_id = None;
                self.reload_tree(db_path);
            }
            CollectionsCmd::CreateFolder { parent, name } => {
                let _ = core_collections::create_collection_db(db_path, ws, parent.clone(), name);
                if let Some(parent) = parent {
                    self.reveal.insert(parent);
                }
                self.reload_tree(db_path);
            }
            CollectionsCmd::CreateRequest { collection_id, name } => {
                let created = core_collections::create_request_db(db_path, collection_id.clone(), name);
                self.reveal.insert(collection_id.clone());
                self.reload_tree(db_path);
                if let Ok(request) = created {
                    self.open_request(request.id, db_path);
                }
            }
            CollectionsCmd::RenameFolder { id, name } => {
                if let Some(folder) = find_folder(&self.trees, &id) {
                    let collection = Collection {
                        id: folder.id.clone(),
                        workspace_id: folder.workspace_id.clone(),
                        parent_id: folder.parent_id.clone(),
                        name,
                        description: folder.description.clone(),
                        order_index: folder.order_index,
                        created_at_ms: folder.created_at_ms,
                        updated_at_ms: chrono::Utc::now().timestamp_millis(),
                    };
                    let _ = core_collections::update_collection_db(db_path, collection);
                }
                self.reload_tree(db_path);
            }
            CollectionsCmd::RenameRequest { id, name } => {
                self.flush(db_path);
                if let Some(mut request) = self.find_request(&id).cloned() {
                    request.name = name;
                    let _ = core_collections::update_request_db(db_path, request);
                }
                self.reload_tree(db_path);
            }
            CollectionsCmd::DeleteFolder(id) => {
                let _ = core_collections::delete_collection_db(db_path, &id);
                self.reload_tree(db_path);
            }
            CollectionsCmd::DeleteRequest(id) => {
                self.dirty.remove(&id);
                let _ = core_collections::delete_request_db(db_path, &id);
                self.reload_tree(db_path);
            }
            CollectionsCmd::DuplicateFolder(id) => {
                let _ = core_collections::duplicate_collection_db(db_path, &id);
                self.reload_tree(db_path);
            }
            CollectionsCmd::DuplicateRequest(id) => {
                self.flush(db_path);
                let _ = core_collections::duplicate_request_db(db_path, &id);
                self.reload_tree(db_path);
            }
            CollectionsCmd::MoveFolder { id, parent } => {
                let into_itself = parent.as_ref().is_some_and(|p| p == &id || folder_contains(&self.trees, &id, p));
                if !into_itself {
                    let _ = core_collections::move_collection_db(db_path, &id, parent.clone());
                    if let Some(parent) = parent {
                        self.reveal.insert(parent);
                    }
                    self.reload_tree(db_path);
                }
            }
            CollectionsCmd::MoveRequest { id, collection_id } => {
                let target = match collection_id {
                    Some(target) => Some(target),
                    None => self.first_root_folder_or_create(db_path),
                };
                if let Some(target) = target {
                    self.flush(db_path);
                    let _ = core_collections::move_request_db(db_path, &id, &target);
                    self.reveal.insert(target.clone());
                    self.reload_tree(db_path);
                }
            }
            CollectionsCmd::Open(id) => self.open_request(id, db_path),
            CollectionsCmd::Activate(id) => {
                self.reload_runs(&id, db_path);
                self.active_id = Some(id);
            }
            CollectionsCmd::Close(id) => {
                self.flush(db_path);
                let index = self.open.iter().position(|r| r.id == id);
                self.open.retain(|r| r.id != id);
                if self.active_id.as_deref() == Some(id.as_str()) {
                    let next = index.and_then(|i| self.open.get(i.min(self.open.len().saturating_sub(1))));
                    self.active_id = next.map(|r| r.id.clone());
                    if let Some(next) = self.active_id.clone() {
                        self.reload_runs(&next, db_path);
                    }
                }
            }
            CollectionsCmd::Execute(_) => {}
            CollectionsCmd::RefreshPreview(id) => {
                self.flush(db_path);
                self.previews.insert(id.clone(), preview_request_db(db_path, &id));
            }
            CollectionsCmd::ClearRuns(id) => {
                let _ = core_collections::clear_request_histories_db(db_path, &id);
                self.selected_run.remove(&id);
                self.latest.remove(&id);
                self.reload_runs(&id, db_path);
            }
            CollectionsCmd::SaveExtractRules { request_id, rules } => {
                self.flush(db_path);
                if let Some(open) = self.open.iter_mut().find(|r| r.id == request_id) {
                    open.extract_rules = rules.clone();
                }
                if let Some(mut request) = self.find_request(&request_id).cloned() {
                    request.extract_rules = rules;
                    let _ = core_collections::update_request_db(db_path, request);
                }
                self.reload_tree(db_path);
            }
        }
    }

    fn open_request(&mut self, id: String, db_path: &PathBuf) {
        if !self.open.iter().any(|r| r.id == id) {
            let Some(request) = self.find_request(&id).cloned() else { return };
            self.open.push(request);
        }
        self.reload_runs(&id, db_path);
        self.active_id = Some(id);
    }

    fn first_root_folder_or_create(&mut self, db_path: &PathBuf) -> Option<String> {
        if let Some(first) = self.trees.iter().find(|c| c.parent_id.is_none()) {
            return Some(first.id.clone());
        }
        let created = core_collections::create_collection_db(
            db_path,
            self.active_workspace_id.clone(),
            None,
            "Unassigned Requests".to_string(),
        );
        created.ok().map(|c| c.id)
    }

    pub fn reload_runs(&mut self, id: &str, db_path: &PathBuf) {
        let runs = core_collections::get_request_histories_db(db_path, id).unwrap_or_default();
        self.runs.insert(id.to_string(), runs);
    }

    pub fn start_run(&mut self, id: &str, db_path: &PathBuf) -> bool {
        self.flush(db_path);
        self.executing.insert(id.to_string())
    }

    pub fn finish_run(&mut self, id: &str, result: Result<ExecutionResult, String>, db_path: &PathBuf) {
        self.executing.remove(id);
        if let Ok(result) = result {
            self.latest.insert(id.to_string(), result);
        }
        self.selected_run.remove(id);
        self.reload_runs(id, db_path);
    }

    pub fn start_edit(&mut self, edit: TreeEdit, text: String) {
        if let TreeEdit::NewFolder { parent: Some(id) } | TreeEdit::NewRequest { collection_id: id } = &edit {
            self.reveal.insert(id.clone());
        }
        self.tree_edit = Some(edit);
        self.edit_text = text;
        self.edit_focus = true;
    }

    pub fn open_extract_modal(&mut self, request_ids: Vec<String>) {
        let Some(first) = request_ids.first().cloned() else { return };
        let rules = self.find_request(&first).map(|r| r.extract_rules.clone()).unwrap_or_default();
        self.extract_modal = Some(ExtractModal { request_ids, selected: first, rules });
    }
}

pub fn all_requests(trees: &[CollectionTreeItem]) -> Vec<&RequestItem> {
    trees.iter().flat_map(|c| c.requests.iter().chain(all_requests(&c.children))).collect()
}

pub fn find_folder<'a>(trees: &'a [CollectionTreeItem], id: &str) -> Option<&'a CollectionTreeItem> {
    trees.iter().find_map(|c| if c.id == id { Some(c) } else { find_folder(&c.children, id) })
}

fn folder_contains(trees: &[CollectionTreeItem], ancestor: &str, id: &str) -> bool {
    find_folder(trees, ancestor).is_some_and(|folder| find_folder(&folder.children, id).is_some())
}

fn patch_tree(trees: &mut [CollectionTreeItem], request: RequestItem) {
    for folder in trees.iter_mut() {
        if let Some(slot) = folder.requests.iter_mut().find(|r| r.id == request.id) {
            *slot = request;
            return;
        }
        patch_tree(&mut folder.children, request.clone());
    }
}
