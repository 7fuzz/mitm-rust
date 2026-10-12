use std::sync::Arc;
use mitm_core::state::SourceScope;
use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use super::RewriteState;

/// After a failed save, wait this long before trying again.
const RETRY_DELAY_SECS: f64 = 5.0;

#[derive(Clone)]
pub enum Cmd {
    SetEnabled(bool),
    SetScope(SourceScope),
    ClearHistory,
}

impl From<Cmd> for AppCommand {
    fn from(cmd: Cmd) -> Self {
        AppCommand::Rewrite(cmd)
    }
}

pub fn handle(state: &mut RewriteState, cx: &Cx, cmd: Cmd) {
    match cmd {
        Cmd::SetEnabled(enabled) => {
            cx.report(cx.core.set_rewrite_enabled(enabled));
        }
        Cmd::SetScope(scope) => {
            let core = Arc::clone(cx.core);
            state.jobs.spawn(cx.rt, cx.ctx, async move { core.set_rewrite_source_scope(scope).await });
        }
        Cmd::ClearHistory => {
            if cx.report(cx.core.clear_rewrite_history()).is_some() {
                state.clear_log();
            }
        }
    }
}

/// Reports scope changes that failed and saves edited rules once edits pause.
pub fn poll(state: &mut RewriteState, cx: &Cx, now: f64) {
    for result in state.jobs.drain() {
        cx.report(result);
    }
    if let Some(rules) = state.take_save(now, false) {
        if cx.report(cx.rt.block_on(cx.core.save_rewrite_rules(rules))).is_none() {
            state.dirty_since = Some(now + RETRY_DELAY_SECS);
        }
    }
}
