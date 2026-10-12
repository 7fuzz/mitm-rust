use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use mitm_core::collections::preview::preview_request_db;
use mitm_core::collections::{self as core_collections, Collection};
use super::{find_folder, folder_contains, CollectionsCmd, CollectionsState};

impl From<CollectionsCmd> for AppCommand {
    fn from(cmd: CollectionsCmd) -> Self {
        AppCommand::Collections(cmd)
    }
}

impl CollectionsState {
    pub fn apply(&mut self, cmd: CollectionsCmd, cx: &Cx) {
        let db_path = cx.db_path();
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
                cx.report(core_collections::create_collection_db(db_path, ws, parent.clone(), name));
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
                    cx.report(core_collections::update_collection_db(db_path, collection));
                }
                self.reload_tree(db_path);
            }
            CollectionsCmd::RenameRequest { id, name } => {
                self.flush(db_path);
                if let Some(mut request) = self.find_request(&id).cloned() {
                    request.name = name;
                    cx.report(core_collections::update_request_db(db_path, request));
                }
                self.reload_tree(db_path);
            }
            CollectionsCmd::DeleteFolder(id) => {
                cx.report(core_collections::delete_collection_db(db_path, &id));
                self.reload_tree(db_path);
            }
            CollectionsCmd::DeleteRequest(id) => {
                self.saves.remove(&id);
                cx.report(core_collections::delete_request_db(db_path, &id));
                self.reload_tree(db_path);
            }
            CollectionsCmd::DuplicateFolder(id) => {
                cx.report(core_collections::duplicate_collection_db(db_path, &id));
                self.reload_tree(db_path);
            }
            CollectionsCmd::DuplicateRequest(id) => {
                self.flush(db_path);
                cx.report(core_collections::duplicate_request_db(db_path, &id));
                self.reload_tree(db_path);
            }
            CollectionsCmd::MoveFolder { id, parent } => {
                let into_itself = parent.as_ref().is_some_and(|p| p == &id || folder_contains(&self.trees, &id, p));
                if !into_itself {
                    cx.report(core_collections::move_collection_db(db_path, &id, parent.clone()));
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
                    cx.report(core_collections::move_request_db(db_path, &id, &target));
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
                cx.report(core_collections::clear_request_histories_db(db_path, &id));
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
                    cx.report(core_collections::update_request_db(db_path, request));
                }
                self.reload_tree(db_path);
            }
        }
    }
}
