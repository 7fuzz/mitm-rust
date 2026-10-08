mod view;

use serde::{Deserialize, Serialize};

pub use view::show;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Mode {
    Digest,
    Hmac,
    Bcrypt,
}

/// The result of a bcrypt job, which runs off the UI thread because it is slow.
pub enum BcryptOutcome {
    Hash(Result<String, String>),
    Verify(Result<bool, String>),
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct HashingPrefs {
    pub mode: Mode,
    pub bcrypt_cost: u32,
}

impl Default for HashingPrefs {
    fn default() -> Self {
        Self { mode: Mode::Digest, bcrypt_cost: 12 }
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
}

impl HashingState {
    pub fn bcrypt_done(&mut self, outcome: BcryptOutcome) {
        match outcome {
            BcryptOutcome::Hash(result) => self.bcrypt_hash = Some(result),
            BcryptOutcome::Verify(result) => self.verify_result = Some(result),
        }
    }
}
