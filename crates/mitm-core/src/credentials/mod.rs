//! Credentials: named entries of text, secret and TOTP fields, encrypted at rest, kept in vaults that workspaces link to.

pub mod crypto;
mod error;
mod master;
mod rename;
mod resolve;
mod session;
mod store;
mod transfer;
mod vault;

pub use error::CredentialError;
pub use master::*;
pub use rename::*;
pub use resolve::*;
pub use store::*;
pub use transfer::*;
pub(crate) use rename::rewrite_legacy_refs;
pub(crate) use vault::ensure_linked_vault;
pub use vault::{
    create_vault, delete_vault, link_or_create_vault, link_vault, list_vaults, migrate_into_vaults, rename_vault, unlink_vault,
    vault_name_from, Vault,
};
pub(crate) use session::data_key_or_create;

#[cfg(test)]
pub(crate) mod tests_support {
    pub(crate) use super::store::tests::{temp_db, vault};
}
