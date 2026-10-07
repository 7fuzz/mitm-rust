use std::path::PathBuf;
use crossbeam_channel::{Receiver, Sender};
use mitm_core::repeater::{RepeaterExecutionResult, RepeaterTab};
use super::RepeaterState;

type RunResult = (String, Result<RepeaterExecutionResult, String>);

pub struct RepeaterWorker {
    tx: Sender<RunResult>,
    rx: Receiver<RunResult>,
}

impl Default for RepeaterWorker {
    fn default() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self { tx, rx }
    }
}

impl RepeaterWorker {
    pub fn send(&self, rt: &tokio::runtime::Runtime, ctx: &eframe::egui::Context, db_path: PathBuf, tab: RepeaterTab) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        rt.spawn(async move {
            let result = mitm_core::repeater::execute::execute_repeater_tab(&db_path, &tab).await;
            let _ = tx.send((tab.id, result));
            ctx.request_repaint();
        });
    }

    pub fn poll(&self, state: &mut RepeaterState, db_path: &PathBuf) {
        while let Ok((tab_id, result)) = self.rx.try_recv() {
            state.finish_run(&tab_id, result, db_path);
        }
    }
}
