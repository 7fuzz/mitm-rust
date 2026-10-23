//! Connections are credential entries in the `databases` vault, so their passwords are encrypted with the rest of
//! the credentials. Saved queries are plain rows.
use dbviewer_core::{Engine, Profile, SavedQuery, SshConfig};
use mitm_core::credentials::{self, Credential, Field, FieldKind};
use mitm_core::db::dbviewer_db::{self, StoredQuery};
use mitm_core::db::Db;

pub const VAULT: &str = "databases";
const IMPORTED_PREF: &str = "dbviewer_connections_imported";

fn text(key: &str, value: impl ToString) -> Field {
    Field { key: key.into(), kind: FieldKind::Text, extracted: false, value: value.to_string() }
}

fn secret(key: &str, value: &str) -> Field {
    Field { key: key.into(), kind: FieldKind::Secret, extracted: false, value: value.into() }
}

fn engine_name(engine: Engine) -> String {
    serde_json::to_value(engine).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

fn fields_for(profile: &Profile) -> Vec<Field> {
    let mut fields = vec![
        text("engine", engine_name(profile.engine)),
        text("host", &profile.host),
        text("port", profile.port),
        text("user", &profile.user),
        text("database", &profile.database),
        secret("password", &profile.password),
        text("read_only", profile.read_only),
    ];
    if let Some(ssh) = &profile.ssh {
        fields.push(text("ssh_host", &ssh.host));
        fields.push(text("ssh_port", ssh.port.map(|p| p.to_string()).unwrap_or_default()));
        fields.push(text("ssh_user", &ssh.user));
        fields.push(text("ssh_identity_file", &ssh.identity_file));
        fields.push(secret("ssh_secret", &ssh.secret));
    }
    fields
}

fn profile_from(credential: &Credential) -> Option<Profile> {
    let get = |key: &str| credential.field(key).map(|f| f.value.as_str());
    let host = get("host").filter(|h| !h.is_empty())?;
    let engine: Engine = get("engine").and_then(|e| serde_json::from_value(e.into()).ok()).unwrap_or_default();
    let ssh = get("ssh_host").filter(|h| !h.is_empty()).map(|host| SshConfig {
        host: host.into(),
        port: get("ssh_port").and_then(|p| p.parse().ok()),
        user: get("ssh_user").unwrap_or_default().into(),
        identity_file: get("ssh_identity_file").unwrap_or_default().into(),
        secret: get("ssh_secret").unwrap_or_default().into(),
    });
    Some(Profile {
        name: credential.name.clone(),
        engine,
        host: host.into(),
        port: get("port").and_then(|p| p.parse().ok()).unwrap_or_else(|| engine.default_port()),
        user: get("user").unwrap_or_default().into(),
        database: get("database").unwrap_or_default().into(),
        password: get("password").unwrap_or_default().into(),
        read_only: get("read_only") != Some("false"),
        ssh,
    })
}

fn vault_id(db: &Db) -> Result<String, String> {
    let vaults = credentials::list_vaults(db).map_err(|e| e.to_string())?;
    match vaults.into_iter().find(|v| v.name.eq_ignore_ascii_case(VAULT)) {
        Some(vault) => Ok(vault.id),
        None => credentials::create_vault(db, VAULT).map(|v| v.id).map_err(|e| e.to_string()),
    }
}

fn existing(db: &Db, vault_id: &str, name: &str) -> Result<Option<Credential>, String> {
    credentials::find_in_vault(db, vault_id, name).map_err(|e| e.to_string())
}

/// Needs the credentials unlocked. The first call moves the standalone viewer's connections.json into the vault.
pub fn load_profiles(db: &Db) -> Result<Vec<Profile>, String> {
    if mitm_core::db::get_preference(db, IMPORTED_PREF).is_none() {
        import_legacy(db)?;
    }
    let vaults = credentials::list_vaults(db).map_err(|e| e.to_string())?;
    let Some(vault) = vaults.into_iter().find(|v| v.name.eq_ignore_ascii_case(VAULT)) else { return Ok(Vec::new()) };
    let summaries = credentials::list_credentials(db).map_err(|e| e.to_string())?;
    let mut profiles = Vec::new();
    for summary in summaries.iter().filter(|s| s.vault_id == vault.id) {
        let credential = credentials::load_credential(db, &summary.id).map_err(|e| e.to_string())?;
        profiles.extend(profile_from(&credential));
    }
    Ok(profiles)
}

fn import_legacy(db: &Db) -> Result<(), String> {
    for value in dbviewer_db::legacy_connections(db) {
        let Ok(mut profile) = serde_json::from_value::<Profile>(value) else { continue };
        let base = credentials::sanitize_name(&profile.name);
        if base.is_empty() {
            continue;
        }
        let vault = vault_id(db)?;
        profile.name = (1..).map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") }).find(|name| matches!(existing(db, &vault, name), Ok(None))).expect("unbounded");
        save_profile(db, None, &profile)?;
    }
    mitm_core::db::set_preference(db, IMPORTED_PREF, "1")
}

/// Writes the connection as an entry named after it. `previous` is the name it had when opened for editing, so a
/// rename keeps the entry (and any refs to it) instead of leaving a copy behind.
pub fn save_profile(db: &Db, previous: Option<&str>, profile: &Profile) -> Result<(), String> {
    let vault = vault_id(db)?;
    let mut credential = match previous.map(|p| existing(db, &vault, p)).transpose()?.flatten() {
        Some(credential) if credential.name != profile.name => {
            credentials::rename_credential(db, &credential.id, &profile.name).map_err(|e| e.to_string())?;
            credentials::load_credential(db, &credential.id).map_err(|e| e.to_string())?
        }
        Some(credential) => credential,
        None => match existing(db, &vault, &profile.name)? {
            Some(_) => return Err(format!("A connection named \"{}\" already exists", profile.name)),
            None => credentials::create_credential(db, &vault, &profile.name).map_err(|e| e.to_string())?,
        },
    };
    let ours = fields_for(profile);
    credential.fields.retain(|f| !ours.iter().any(|o| o.key.eq_ignore_ascii_case(&f.key)) && !f.key.starts_with("ssh_"));
    credential.fields.splice(0..0, ours);
    credentials::save_credential(db, &credential).map_err(|e| e.to_string())
}

pub fn delete_profile(db: &Db, name: &str) -> Result<(), String> {
    let vault = vault_id(db)?;
    match existing(db, &vault, name)? {
        Some(credential) => credentials::delete_credential(db, &credential.id).map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

pub fn load_queries(db: &Db) -> Result<Vec<SavedQuery>, String> {
    let rows = dbviewer_db::load_queries(db)?;
    Ok(rows.iter().filter_map(|data| serde_json::from_str(data).ok()).collect())
}

pub fn save_queries(db: &Db, queries: &[SavedQuery]) -> Result<(), String> {
    let rows = queries
        .iter()
        .map(|q| Ok(StoredQuery { connection: q.connection.clone(), name: q.name.clone(), data: serde_json::to_string(q).map_err(|e| e.to_string())? }))
        .collect::<Result<Vec<_>, String>>()?;
    dbviewer_db::save_queries(db, &rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (Db, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("mitm-dbviewer-store-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::new(dir.join("test.db"));
        mitm_core::db::init_database(&db, None).unwrap();
        (db, dir)
    }

    fn profile(name: &str) -> Profile {
        Profile {
            name: name.into(),
            engine: Engine::MySql,
            password: "pw".into(),
            ssh: Some(SshConfig { host: "bastion".into(), port: Some(2222), user: "me".into(), identity_file: "k".into(), secret: "s".into() }),
            ..Profile::default()
        }
    }

    #[test]
    fn profiles_round_trip_through_credentials_and_rename_in_place() {
        let (db, dir) = temp_db();
        save_profile(&db, None, &profile("prod")).unwrap();
        assert_eq!(load_profiles(&db).unwrap(), vec![profile("prod")]);
        assert!(save_profile(&db, None, &profile("PROD")).is_err());

        let mut renamed = profile("prod-2");
        renamed.read_only = false;
        renamed.ssh = None;
        save_profile(&db, Some("prod"), &renamed).unwrap();
        assert_eq!(load_profiles(&db).unwrap(), vec![renamed]);

        delete_profile(&db, "prod-2").unwrap();
        assert!(load_profiles(&db).unwrap().is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_names_credentials_cannot_hold() {
        let (db, dir) = temp_db();
        assert!(save_profile(&db, None, &profile("my db")).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
