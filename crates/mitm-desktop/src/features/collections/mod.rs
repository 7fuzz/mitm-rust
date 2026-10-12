mod commands;
mod docs;
mod editor;
mod extract_modal;
mod tree;
mod view;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use mitm_core::collections::execute::ExecutionResult;
use mitm_core::collections::preview::RequestPreview;
use mitm_core::collections::{self as core_collections, CollectionTreeItem, RequestHistoryItem, RequestItem};
use mitm_core::repeater::ExtractRuleItem;
use crate::util::save_queue::SaveQueue;
use crate::features::request_editor::body::{is_multipart, is_urlencoded};
use crate::features::request_editor::captured::CapturedRequest;
use crate::features::request_editor::run_list::RunDetails;
use crate::widgets::message_pane::PaneState;

pub use view::show;


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
pub enum SaveTarget {
    Folder(String),
    NewFolder(String),
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
    pub stacked: bool,
}

impl Default for CollectionsPrefs {
    fn default() -> Self {
        Self { split: 0.5, history_open: false, stacked: false }
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
    pub run_details: RunDetails,
    pub extract_modal: Option<ExtractModal>,
    saves: SaveQueue,
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
            run_details: RunDetails::default(),
            extract_modal: None,
            saves: SaveQueue::default(),
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
        self.saves.edited(id, now);
    }

    pub fn flush_if_idle(&mut self, db_path: &PathBuf, now: f64) {
        if self.saves.is_due(now) {
            self.flush(db_path);
            self.saves.retry_later(now);
        }
    }

    pub fn take_save_errors(&mut self) -> Vec<String> {
        self.saves.take_errors()
    }

    pub fn flush(&mut self, db_path: &PathBuf) {
        for id in self.saves.take() {
            let Some(request) = self.open.iter().find(|r| r.id == id).cloned() else { continue };
            match core_collections::update_request_db(db_path, request.clone()) {
                Ok(_) => {
                    self.saves.saved(&id);
                    patch_tree(&mut self.trees, request);
                }
                Err(e) => self.saves.failed(id, format!("Couldn't save request: {e}")),
            }
        }
    }

    pub fn save_captured(&mut self, db_path: &PathBuf, target: SaveTarget, name: String, captured: CapturedRequest) -> Result<(), String> {
        let folder_id = match target {
            SaveTarget::Folder(id) => id,
            SaveTarget::NewFolder(new_name) => {
                core_collections::create_collection_db(db_path, self.active_workspace_id.clone(), None, new_name)?.id
            }
        };
        let mut request = core_collections::create_request_db(db_path, folder_id.clone(), name)?;
        request.method = captured.method;
        request.url = captured.url;
        request.params = captured.params;
        request.headers = captured.headers;
        let body = Some(captured.body_content);
        match captured.body_type.as_str() {
            "json" => request.body_json = body,
            t if is_multipart(t) => request.body_form_data = body,
            t if is_urlencoded(t) => request.body_urlencoded = body,
            _ => request.body_raw = body,
        }
        request.body_type = captured.body_type;
        core_collections::update_request_db(db_path, request)?;
        self.reveal.insert(folder_id);
        self.reload_tree(db_path);
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use mitm_core::state::HistoryEntry;

    #[test]
    fn captured_flow_saves_as_a_request_in_a_new_folder() {
        let dir = std::env::temp_dir().join(format!("mitm-save-flow-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("test.db");
        mitm_core::db::init_database(&db, None).unwrap();
        let workspace = mitm_core::workspace::create_workspace_db(&db, "W".to_string(), None).unwrap();

        let flow = HistoryEntry {
            id: 1,
            method: "POST".to_string(),
            url: "https://api.test/users?page=2".to_string(),
            host: "api.test".to_string(),
            path: "/users".to_string(),
            content_type: String::new(),
            response_size: 0,
            status_code: 200,
            request_headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            response_headers: vec![],
            request_body: r#"{"name":"a"}"#.to_string(),
            response_body: String::new(),
            phase: "response".to_string(),
            duration_ms: None,
            created_at: String::new(),
            is_intercepted: false,
            is_rewritten: false,
            is_failed: false,
            listener_label: String::new(),
            request_at: None,
            response_at: None,
        };
        let mut state = CollectionsState::default();
        state.load(&db, workspace.id);
        state
            .save_captured(&db, SaveTarget::NewFolder("Captured".to_string()), "POST /users".to_string(), CapturedRequest::from_flow(&flow, None))
            .unwrap();

        let folder = state.trees.iter().find(|f| f.name == "Captured").expect("folder created");
        let request = &folder.requests[0];
        assert_eq!(request.method, "POST");
        assert_eq!(request.url, flow.url);
        assert_eq!(request.body_type, "json");
        assert!(request.body_json.as_deref().is_some_and(|b| b.contains("\"name\"")));
        assert!(request.headers.iter().any(|h| h.key == "Content-Type"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
