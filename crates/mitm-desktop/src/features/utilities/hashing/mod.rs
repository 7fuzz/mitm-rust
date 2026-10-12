mod commands;
mod view;

use serde::{Deserialize, Serialize};

pub use commands::{handle, poll, Cmd};
pub use view::show;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Mode {
    Digest,
    Hmac,
    Bcrypt,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct HashingPrefs {
    pub mode: Mode,
    pub bcrypt_cost: u32,
    pub upper: bool,
}

impl Default for HashingPrefs {
    fn default() -> Self {
        Self { mode: Mode::Digest, bcrypt_cost: 12, upper: false }
    }
}

#[derive(Default)]
pub struct HashingState {
    pub prefs: HashingPrefs,
    pub input: String,
    pub hmac_key: String,
    pub bcrypt_password: String,
    pub bcrypt_hash: Option<Result<String, String>>,
    pub verify_password: String,
    pub verify_hash: String,
    pub verify_result: Option<Result<bool, String>>,
    jobs: crate::app::jobs::Jobs<commands::BcryptOutcome>,
}

