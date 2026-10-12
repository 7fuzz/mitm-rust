use std::sync::Arc;
use mitm_core::fuzzer::{FuzzConfig, FuzzResult, FuzzStart};
use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use crate::app::jobs::Jobs;
use super::FuzzerState;

#[derive(Clone)]
pub enum Cmd {
    Start(FuzzConfig),
    Stop,
    SelectResult { run_id: i64, idx: u32 },
    CopyCurl { run_id: i64, idx: u32 },
    SaveRun { run_id: i64, name: String },
    LoadRun(i64),
    DeleteRun(i64),
    ReloadSavedRuns,
}

impl From<Cmd> for AppCommand {
    fn from(cmd: Cmd) -> Self {
        AppCommand::Fuzzer(cmd)
    }
}

#[derive(Default)]
pub struct FuzzJobs {
    start: Jobs<Result<FuzzStart, String>>,
    /// (run id, query, matching result indices) for searches over stored responses.
    pub(super) search: Jobs<(i64, String, Vec<u32>)>,
}

pub fn handle(state: &mut FuzzerState, cx: &Cx, cmd: Cmd) {
    let db_path = cx.db_path();
    match cmd {
        Cmd::Start(_) if state.starting => {}
        Cmd::Start(config) => {
            state.starting = true;
            let core = Arc::clone(cx.core);
            state.jobs.start.spawn(cx.rt, cx.ctx, async move { core.start_fuzz(config).await });
        }
        Cmd::Stop => cx.core.stop_fuzz(),
        Cmd::SelectResult { run_id, idx } => state.select_result(run_id, idx, db_path),
        Cmd::CopyCurl { run_id, idx } => {
            if let Some(result) = state.result(run_id, idx, db_path) {
                cx.ctx.copy_text(crate::util::format::Exchange::from_fuzz_result(&result).curl());
            }
        }
        Cmd::SaveRun { run_id, name } => match cx.core.save_fuzz_run(run_id, &name) {
            Ok(()) => {
                if state.run_id == Some(run_id) {
                    state.is_saved = true;
                    state.run_name = Some(name);
                }
                state.load_saved_runs(db_path);
            }
            Err(e) => state.error = Some(e),
        },
        Cmd::LoadRun(id) => match cx.core.load_fuzz_run(id) {
            Ok(loaded) => state.load_run(loaded),
            Err(e) => state.error = Some(e),
        },
        Cmd::DeleteRun(id) => {
            if cx.report(cx.core.delete_fuzz_run(id)).is_some() && state.run_id == Some(id) {
                state.is_saved = false;
                state.run_name = None;
            }
            state.load_saved_runs(db_path);
        }
        Cmd::ReloadSavedRuns => state.load_saved_runs(db_path),
    }
}

pub fn poll(state: &mut FuzzerState, cx: &Cx) {
    let starts: Vec<_> = state.jobs.start.drain().collect();
    for start in starts {
        state.starting = false;
        state.started(start);
    }
    let searches: Vec<_> = state.jobs.search.drain().collect();
    for (run_id, query, matches) in searches {
        state.set_response_search_results(run_id, &query, matches);
    }
    state.sync_response_search(cx);
}

impl FuzzerState {
    /// The full stored result, reusing the selected one when it is the same row.
    pub fn result(&self, run_id: i64, idx: u32, db_path: &std::path::PathBuf) -> Option<FuzzResult> {
        let selected = self.selected_result.as_ref().filter(|r| r.idx == idx && self.run_id == Some(run_id));
        match selected {
            Some(result) => Some(result.clone()),
            None => mitm_core::fuzzer::get_result_db(db_path, run_id, idx).ok().flatten(),
        }
    }
}
