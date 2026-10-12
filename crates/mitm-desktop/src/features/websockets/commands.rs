use std::sync::Arc;
use mitm_core::ws::types::WebSocketConn;
use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use crate::app::jobs::Jobs;
use super::WebSocketsState;

#[derive(Clone)]
pub enum Cmd {
    Select(i64),
    Connect { url: String, headers: Vec<(String, String)> },
    SendFrame { connection_id: i64, msg_type: String, payload: String, to_server: bool },
    Disconnect(i64),
    ClearSession(i64),
    DeleteSession(i64),
    SetMitm(bool),
}

impl From<Cmd> for AppCommand {
    fn from(cmd: Cmd) -> Self {
        AppCommand::WebSockets(cmd)
    }
}

#[derive(Default)]
pub struct WsJobs {
    connects: Jobs<Result<WebSocketConn, String>>,
    actions: Jobs<Result<(), String>>,
    /// The value asked for, and whether the core took it.
    mitm: Jobs<(bool, Result<(), String>)>,
}

pub fn handle(state: &mut WebSocketsState, cx: &Cx, cmd: Cmd) {
    let core = Arc::clone(cx.core);
    match cmd {
        Cmd::Select(id) => state.select(id, cx.db_path()),
        Cmd::Connect { url, headers } => {
            let headers = (!headers.is_empty()).then_some(headers);
            state.jobs.connects.spawn(cx.rt, cx.ctx, async move { core.connect_ws_client(url, headers).await });
        }
        Cmd::SendFrame { connection_id, msg_type, payload, to_server } => {
            state.jobs.actions.spawn(cx.rt, cx.ctx, async move {
                core.send_ws_frame(connection_id, msg_type, payload, to_server).await
            });
        }
        Cmd::Disconnect(id) => {
            state.jobs.actions.spawn(cx.rt, cx.ctx, async move { core.disconnect_ws(id).await });
        }
        Cmd::ClearSession(id) => {
            if cx.report(cx.core.clear_ws_session(id)).is_some() {
                state.cleared(id);
            }
        }
        Cmd::DeleteSession(id) => {
            if cx.report(cx.core.delete_ws_connection(id)).is_some() {
                state.removed(id);
            }
        }
        Cmd::SetMitm(enabled) => {
            state.jobs.mitm.spawn(cx.rt, cx.ctx, async move { (enabled, core.set_ws_mitm_enabled(enabled).await) });
        }
    }
}

pub fn poll(state: &mut WebSocketsState, cx: &Cx) {
    let connects: Vec<_> = state.jobs.connects.drain().collect();
    for result in connects {
        if let Some(conn) = cx.report(result) {
            state.connected(conn, cx.db_path());
        }
    }
    for result in state.jobs.actions.drain() {
        cx.report(result);
    }
    for (enabled, result) in state.jobs.mitm.drain() {
        if cx.report(result).is_none() {
            state.mitm_enabled = !enabled;
        }
    }
}
