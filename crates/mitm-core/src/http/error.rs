use std::fmt;
use crate::credentials::CredentialError;

/// Why a Repeater or Collections send failed. `Locked` means the request needs credential values while
/// they're locked, so the app asks for the master password and retries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    Locked,
    Failed(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Locked => CredentialError::Locked.fmt(f),
            RunError::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for RunError {}

impl From<String> for RunError {
    fn from(message: String) -> Self {
        RunError::Failed(message)
    }
}

impl From<CredentialError> for RunError {
    fn from(error: CredentialError) -> Self {
        match error {
            CredentialError::Locked => RunError::Locked,
            CredentialError::Failed(message) => RunError::Failed(message),
        }
    }
}
