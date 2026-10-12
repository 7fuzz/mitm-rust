use crate::app::command::AppCommand;
use crate::app::cx::Cx;
use super::HashingState;

#[derive(Clone)]
pub enum Cmd {
    GenerateBcrypt { password: String, cost: u32 },
    VerifyBcrypt { password: String, hash: String },
}

impl From<Cmd> for AppCommand {
    fn from(cmd: Cmd) -> Self {
        AppCommand::Hashing(cmd)
    }
}

/// The result of a bcrypt job, which runs off the UI thread because it is slow.
pub enum BcryptOutcome {
    Hash(Result<String, String>),
    Verify(Result<bool, String>),
}

pub fn handle(state: &mut HashingState, cx: &Cx, cmd: Cmd) {
    let job = async move {
        let outcome = tokio::task::spawn_blocking(move || match cmd {
            Cmd::GenerateBcrypt { password, cost } => BcryptOutcome::Hash(mitm_core::utils::hashing::bcrypt_hash(&password, cost)),
            Cmd::VerifyBcrypt { password, hash } => BcryptOutcome::Verify(mitm_core::utils::hashing::bcrypt_verify(&password, &hash)),
        });
        outcome.await.unwrap_or_else(|e| BcryptOutcome::Hash(Err(e.to_string())))
    };
    state.jobs.spawn(cx.rt, cx.ctx, job);
}

pub fn poll(state: &mut HashingState) {
    let finished: Vec<_> = state.jobs.drain().collect();
    for outcome in finished {
        match outcome {
            BcryptOutcome::Hash(result) => state.bcrypt_hash = Some(result),
            BcryptOutcome::Verify(result) => state.verify_result = Some(result),
        }
    }
}
