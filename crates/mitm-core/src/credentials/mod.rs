//! Workspace credentials: named entries of text, secret and TOTP fields, encrypted at rest.

pub mod crypto;
mod session;
mod store;

pub use store::*;

/// Error string for anything that needs credential values while the data key is locked.
pub const LOCKED: &str = "CREDENTIALS_LOCKED";
