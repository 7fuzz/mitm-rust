use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use super::RepeaterState;

#[derive(Clone)]
pub enum Cmd {
    New,
    Select(String),
    Delete(String),
}

impl From<Cmd> for AppCommand {
    fn from(cmd: Cmd) -> Self {
        AppCommand::Repeater(cmd)
    }
}

pub fn handle(state: &mut RepeaterState, cx: &Cx, cmd: Cmd) {
    let db_path = cx.db_path();
    match cmd {
        Cmd::New => state.create_blank(db_path),
        Cmd::Select(id) => state.select(id, db_path),
        Cmd::Delete(id) => state.delete(&id, db_path),
    }
}
