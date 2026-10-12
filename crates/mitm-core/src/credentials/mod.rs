//! Workspace credentials: named entries of text, secret and TOTP fields, encrypted at rest.

pub mod crypto;
mod resolve;
mod session;
mod store;

pub use resolve::*;
pub use store::*;

/// Error string for anything that needs credential values while the data key is locked.
pub const LOCKED: &str = "CREDENTIALS_LOCKED";

#[cfg(test)]
pub(crate) mod tests_support {
    pub(crate) use super::store::tests::temp_db;
}
