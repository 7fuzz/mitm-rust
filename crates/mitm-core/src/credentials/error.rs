use std::fmt;

/// Why a credentials call failed; `Locked` is the one callers act on (by asking for the master password).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialError {
    Locked,
    Failed(String),
}

impl fmt::Display for CredentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CredentialError::Locked => f.write_str(super::LOCKED),
            CredentialError::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for CredentialError {}

impl From<String> for CredentialError {
    fn from(message: String) -> Self {
        CredentialError::Failed(message)
    }
}

impl From<&str> for CredentialError {
    fn from(message: &str) -> Self {
        CredentialError::Failed(message.to_string())
    }
}

impl From<CredentialError> for String {
    fn from(error: CredentialError) -> Self {
        error.to_string()
    }
}
