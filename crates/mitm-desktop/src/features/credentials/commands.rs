use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use super::CredentialsState;

#[derive(Clone)]
pub enum Cmd {
    Lock,
    CopyTotp(String),
}

impl From<Cmd> for AppCommand {
    fn from(cmd: Cmd) -> Self {
        AppCommand::Credentials(cmd)
    }
}

pub fn handle(state: &mut CredentialsState, cx: &Cx, cmd: Cmd) {
    match cmd {
        Cmd::Lock => state.lock(cx.db_path()),
        Cmd::CopyTotp(id) => {
            if let Some(code) = state.current_code(&id) {
                crate::util::clipboard::copy_secret(cx.ctx, &code);
            }
        }
    }
}
