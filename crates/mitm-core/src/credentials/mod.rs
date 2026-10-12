//! Workspace credentials: named entries of text, secret and TOTP fields, encrypted at rest.

pub mod crypto;
mod master;
mod rename;
mod resolve;
mod session;
mod store;
mod transfer;

pub use master::*;
pub use rename::*;
pub use resolve::*;
pub use store::*;
pub use transfer::*;
pub(crate) use session::data_key_or_create;

/// Error string for anything that needs credential values while the data key is locked.
pub const LOCKED: &str = "CREDENTIALS_LOCKED";

#[cfg(test)]
pub(crate) mod tests_support {
    pub(crate) use super::store::tests::temp_db;
}
