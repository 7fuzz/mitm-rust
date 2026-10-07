mod view;

use std::path::PathBuf;
use mitm_core::collections::CollectionTreeItem;
use mitm_core::workspace::Workspace;

pub use view::show;

#[derive(Default)]
pub struct CollectionsState {
    pub workspaces: Vec<Workspace>,
    pub active_workspace_id: String,
    pub trees: Vec<CollectionTreeItem>,
}

impl CollectionsState {
    pub fn load(&mut self, db_path: &PathBuf) {
        if let Ok(workspaces) = mitm_core::workspace::get_workspaces_db(db_path) {
            self.workspaces = workspaces;
        }
        if self.active_workspace_id.is_empty() {
            self.active_workspace_id = self
                .workspaces
                .first()
                .map(|w| w.id.clone())
                .unwrap_or_else(|| mitm_core::repeater::REPEATER_WORKSPACE_ID.to_string());
        }
        if let Ok(trees) = mitm_core::collections::get_collections_db(db_path, &self.active_workspace_id) {
            self.trees = trees;
        }
    }
}
