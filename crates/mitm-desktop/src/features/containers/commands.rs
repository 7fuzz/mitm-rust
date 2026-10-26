use mitm_core::proxy::ContainerRow;
use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use super::ContainersState;

#[derive(Clone)]
pub enum Cmd {
    DeleteFlows(String),
    Unregister(String),
    DeleteUnregisteredFlows,
}

impl From<Cmd> for AppCommand {
    fn from(cmd: Cmd) -> Self {
        AppCommand::Containers(cmd)
    }
}

pub fn handle(state: &mut ContainersState, cx: &Cx, cmd: Cmd) {
    match cmd {
        Cmd::DeleteFlows(id) => delete_flows(cx, &[id]),
        Cmd::Unregister(id) => {
            cx.report(cx.core.remove_container(&id));
        }
        Cmd::DeleteUnregisteredFlows => {
            let ids: Vec<String> = state.unregistered().map(|row: &ContainerRow| row.id.clone()).collect();
            delete_flows(cx, &ids);
        }
    }
}

pub fn poll(state: &mut ContainersState, cx: &Cx, now: f64) {
    if state.due(now) {
        state.rows = cx.core.container_overview();
    }
}

fn delete_flows(cx: &Cx, ids: &[String]) {
    let mut removed = 0;
    for id in ids {
        match cx.core.delete_container_flows(id, None) {
            Ok(count) => removed += count,
            Err(error) => return cx.notify(Err(error)),
        }
    }
    cx.notify(Ok(format!("Deleted {removed} flows")));
}
