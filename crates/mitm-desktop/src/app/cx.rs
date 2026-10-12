use std::path::PathBuf;
use std::sync::Arc;
use eframe::egui;
use mitm_core::MitmCore;
use tokio::runtime::Runtime;
use crate::widgets::toast;

/// What a feature's command handler works with: the runtime for jobs, the core, and the egui context.
pub struct Cx<'a> {
    pub rt: &'a Runtime,
    pub core: &'a Arc<MitmCore>,
    pub ctx: &'a egui::Context,
}

impl Cx<'_> {
    pub fn db_path(&self) -> &PathBuf {
        &self.core.db_path
    }

    /// Shows a failure as a toast and passes a success through.
    pub fn report<T>(&self, result: Result<T, String>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                toast::push(self.ctx, Err(error));
                None
            }
        }
    }
}
