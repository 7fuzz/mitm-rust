//! Workspace credentials: named entries of text, secret and TOTP fields, encrypted at rest.

pub mod crypto;
mod error;
mod master;
mod rename;
mod resolve;
mod session;
mod store;
mod transfer;

pub use error::CredentialError;
pub use master::*;
pub use rename::*;
pub use resolve::*;
pub use store::*;
pub use transfer::*;
pub(crate) use session::data_key_or_create;

#[cfg(test)]
pub(crate) mod tests_support {
    pub(crate) use super::store::tests::temp_db;
}
