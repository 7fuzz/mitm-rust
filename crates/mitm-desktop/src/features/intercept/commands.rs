use std::sync::Arc;
use mitm_core::db::InterceptRule;
use mitm_core::state::SourceScope;
use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use super::InterceptState;

#[derive(Clone)]
pub enum Cmd {
    Configure { enabled: bool, mode: String },
    Toggle,
    SetScope(SourceScope),
    SaveRules(Vec<InterceptRule>),
    AddRuleForHost(String),
    ForwardActive { modified: bool },
    DropActive,
    ForwardAll,
    DropAll,
}

impl From<Cmd> for AppCommand {
    fn from(cmd: Cmd) -> Self {
        AppCommand::Intercept(cmd)
    }
}

pub fn handle(state: &mut InterceptState, cx: &Cx, cmd: Cmd) {
    let core = Arc::clone(cx.core);
    match cmd {
        Cmd::Configure { enabled, mode } => {
            state.jobs.spawn(cx.rt, cx.ctx, async move { core.toggle_interceptor(enabled, mode).await });
        }
        Cmd::Toggle => {
            state.enabled = !state.enabled;
            let cmd = Cmd::Configure { enabled: state.enabled, mode: state.mode.clone() };
            handle(state, cx, cmd);
        }
        Cmd::SetScope(scope) => {
            state.jobs.spawn(cx.rt, cx.ctx, async move { core.set_intercept_source_scope(scope).await });
        }
        Cmd::SaveRules(rules) => {
            state.jobs.spawn(cx.rt, cx.ctx, async move { core.update_intercept_rules(rules).await });
        }
        Cmd::AddRuleForHost(host) => {
            state.rules.push(InterceptRule {
                id: uuid::Uuid::new_v4().to_string(),
                is_enabled: true,
                target_phase: "request".to_string(),
                match_field: "host".to_string(),
                operator: "equals".to_string(),
                match_value: host,
                order_index: state.rules.len() as i32,
                created_at_ms: chrono::Utc::now().timestamp_millis(),
                action: "intercept".to_string(),
            });
            handle(state, cx, Cmd::SaveRules(state.rules.clone()));
        }
        Cmd::ForwardActive { modified } => {
            let Some(flow_id) = state.active_id.clone() else { return };
            let edit = &state.edit;
            let result = cx.core.forward_intercepted_flow(
                &flow_id,
                modified.then(|| edit.url.clone()),
                modified.then(|| edit.method.clone()),
                modified.then(|| state.edited_headers()),
                modified.then(|| edit.body.as_bytes().to_vec()),
            );
            cx.report(result);
            state.remove_pending(&flow_id);
        }
        Cmd::DropActive => {
            let Some(flow_id) = state.active_id.clone() else { return };
            cx.report(cx.core.drop_intercepted_flow(&flow_id));
            state.remove_pending(&flow_id);
        }
        Cmd::ForwardAll => {
            cx.core.forward_all_intercepted();
            state.clear_pending();
        }
        Cmd::DropAll => {
            cx.core.drop_all_intercepted();
            state.clear_pending();
        }
    }
}

pub fn poll(state: &mut InterceptState, cx: &Cx) {
    for result in state.jobs.drain() {
        cx.report(result);
    }
}
