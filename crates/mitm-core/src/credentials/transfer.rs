//! Vaults in workspace files: vault names, entry names and field keys always, values only sealed by a passphrase.

use std::collections::BTreeMap;
use base64::Engine;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;
use super::crypto::{self, DataKey, KdfParams};
use super::store::{self, list_credentials, load_credential, Field};
use super::vault::list_vaults;
use crate::workspace::file::{CredentialEntry, SealedCredentials, VaultEntry};
use crate::db::Db;
use super::CredentialError;

const AAD: &[u8] = b"mitm-workspace-credentials";
const WRONG_PASSPHRASE: &str = "Wrong passphrase for the credentials in this file";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedEntry {
    /// Empty in files from before vaults.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub vault: String,
    pub name: String,
    #[serde(default)]
    pub values: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

/// The vaults linked to the workspace for a file, plus their values sealed with `passphrase` when one is given.
/// Extracted fields (tokens) carry their values only with `include_auto`.
pub fn export_credentials(
    db: &Db,
    workspace_id: &str,
    passphrase: Option<&str>,
    include_auto: bool,
) -> Result<(Vec<VaultEntry>, Option<SealedCredentials>), String> {
    let vaults: Vec<_> = list_vaults(db)?.into_iter().filter(|v| v.is_linked(workspace_id)).collect();
    let summaries: Vec<_> = list_credentials(db)?.into_iter().filter(|c| vaults.iter().any(|v| v.id == c.vault_id)).collect();
    let entries = vaults
        .iter()
        .map(|vault| VaultEntry {
            name: vault.name.clone(),
            credentials: summaries
                .iter()
                .filter(|c| c.vault_id == vault.id)
                .map(|c| CredentialEntry { name: c.name.clone(), fields: c.fields.clone() })
                .collect(),
        })
        .collect();
    let Some(passphrase) = passphrase.filter(|_| !summaries.is_empty()) else { return Ok((entries, None)) };

    let mut sealed_entries = Vec::new();
    for summary in &summaries {
        let credential = load_credential(db, &summary.id)?;
        let values = credential
            .fields
            .iter()
            .filter(|f| include_auto || !f.extracted)
            .map(|f| (f.key.clone(), f.value.clone()))
            .collect();
        sealed_entries.push(SealedEntry { vault: credential.vault, name: credential.name, values, notes: credential.notes });
    }
    Ok((entries, Some(seal(passphrase, &sealed_entries)?)))
}

pub fn seal(passphrase: &str, entries: &[SealedEntry]) -> Result<SealedCredentials, CredentialError> {
    let params = KdfParams::default();
    let salt = crypto::random_bytes::<{ crypto::SALT_LEN }>();
    let key = crypto::derive_key(passphrase, &salt, params)?;
    let plain = Zeroizing::new(serde_json::to_vec(entries).map_err(|e| e.to_string())?);
    let sealed = crypto::seal(&key, AAD, &plain);
    Ok(SealedCredentials {
        kdf: crypto::KDF.to_string(),
        params,
        salt: b64().encode(salt),
        nonce: b64().encode(sealed.nonce),
        ciphertext: b64().encode(sealed.ciphertext),
    })
}

pub fn open_sealed(block: &SealedCredentials, passphrase: &str) -> Result<Vec<SealedEntry>, CredentialError> {
    if block.kdf != crypto::KDF {
        return Err(format!("Unsupported credential encryption \"{}\"", block.kdf).into());
    }
    let decode = |s: &str| b64().decode(s).map_err(|_| "Corrupt credentials block".to_string());
    let key = crypto::derive_key(passphrase, &decode(&block.salt)?, block.params)?;
    let plain = crypto::open(&key, AAD, &decode(&block.nonce)?, &decode(&block.ciphertext)?)
        .map_err(|_| WRONG_PASSPHRASE.to_string())?;
    Ok(serde_json::from_slice(&plain).map_err(|e| e.to_string())?)
}

/// Writes a file vault's entries into `vault_id`. An entry that already exists only gains the fields it lacks.
/// `sealed` values count when their vault is `file_vault` (empty for files from before vaults).
/// Returns how many entries were created or extended.
pub(crate) fn import_credentials(
    conn: &Connection,
    key: &DataKey,
    vault_id: &str,
    file_vault: &str,
    entries: &[CredentialEntry],
    sealed: &[SealedEntry],
) -> Result<usize, String> {
    let mut touched = 0;
    for entry in entries {
        let values = sealed.iter().find(|s| s.vault.eq_ignore_ascii_case(file_vault) && s.name.eq_ignore_ascii_case(&entry.name));
        let value_of = |key: &str| {
            values
                .and_then(|v| v.values.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)))
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        let existing = store::find_credential_conn(conn, key, vault_id, &entry.name)?;
        let is_new = existing.is_none();
        let mut credential = match existing {
            Some(c) => c,
            None => store::new_credential(conn, vault_id, &entry.name)?,
        };
        let before = credential.fields.len();
        for meta in &entry.fields {
            if credential.field(&meta.key).is_none() {
                credential.fields.push(Field { key: meta.key.clone(), kind: meta.kind, extracted: meta.extracted, value: value_of(&meta.key) });
            }
        }
        if is_new {
            credential.notes = values.map(|v| v.notes.clone()).unwrap_or_default();
            store::insert_credential_conn(conn, key, &credential)?;
        } else if credential.fields.len() > before {
            store::update_credential_conn(conn, key, &credential)?;
        } else {
            continue;
        }
        touched += 1;
    }
    Ok(touched)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_block_opens_only_with_its_passphrase() {
        let entries = vec![SealedEntry {
            vault: "DG".into(),
            name: "dgAdmin".into(),
            values: [("password".to_string(), "hunter2".to_string())].into(),
            notes: "n".into(),
        }];
        let block = seal("pass phrase", &entries).unwrap();
        assert!(!block.ciphertext.contains("hunter2"));
        assert_eq!(open_sealed(&block, "pass phrase").unwrap(), entries);
        assert_eq!(open_sealed(&block, "wrong").unwrap_err(), CredentialError::from(WRONG_PASSPHRASE));
    }
}
