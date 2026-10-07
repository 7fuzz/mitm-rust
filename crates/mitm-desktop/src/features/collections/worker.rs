use std::path::PathBuf;
use crossbeam_channel::{Receiver, Sender};
use mitm_core::collections::execute::ExecutionResult;
use super::CollectionsState;

type RunResult = (String, Result<ExecutionResult, String>);

pub struct CollectionsWorker {
    tx: Sender<RunResult>,
    rx: Receiver<RunResult>,
}

impl Default for CollectionsWorker {
    fn default() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self { tx, rx }
    }
}

impl CollectionsWorker {
    pub fn execute(&self, rt: &tokio::runtime::Runtime, ctx: &eframe::egui::Context, db_path: PathBuf, request_id: String) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        rt.spawn(async move {
            let result = mitm_core::collections::execute::execute_collection_request_db(&db_path, &request_id).await;
            let _ = tx.send((request_id, result));
            ctx.request_repaint();
        });
    }

    pub fn poll(&self, state: &mut CollectionsState, db_path: &PathBuf) {
        while let Ok((request_id, result)) = self.rx.try_recv() {
            state.finish_run(&request_id, result, db_path);
        }
    }
}
