use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::engine::Engine;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SshConfig {
    /// Hostname or an alias from `~/.ssh/config`.
    pub host: String,
    pub port: Option<u16>,
    pub user: String,
    pub identity_file: String,
    /// Key passphrase or SSH password. Stored in plain text; empty means ask on connect.
    #[serde(default)]
    pub secret: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub engine: Engine,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub database: String,
    /// Stored in plain text in the connections file; empty means ask on connect.
    #[serde(default)]
    pub password: String,
    pub read_only: bool,
    pub ssh: Option<SshConfig>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: String::new(),
            engine: Engine::Postgres,
            host: "127.0.0.1".into(),
            port: 5432,
            user: "postgres".into(),
            database: "postgres".into(),
            password: String::new(),
            read_only: true,
            ssh: None,
        }
    }
}

impl Profile {
    /// Replaces fields still holding the previous engine's defaults.
    pub fn switch_engine_defaults(&mut self, previous: Engine) {
        if self.port == previous.default_port() {
            self.port = self.engine.default_port();
        }
        if self.user == previous.default_user() {
            self.user = self.engine.default_user().into();
        }
        if self.database == previous.default_database() {
            self.database = self.engine.default_database().into();
        }
    }
}

pub struct ProfileStore {
    path: PathBuf,
}

impl ProfileStore {
    pub fn default_location() -> Self {
        let dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("dbviewer");
        Self { path: dir.join("connections.json") }
    }

    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<Vec<Profile>> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => serde_json::from_str(&text).with_context(|| format!("parsing {}", self.path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e).with_context(|| format!("reading {}", self.path.display())),
        }
    }

    pub fn save(&self, profiles: &[Profile]) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&self.path, serde_json::to_string_pretty(profiles)?)
            .with_context(|| format!("writing {}", self.path.display()))?;
        restrict_to_owner(&self.path)
    }
}

#[cfg(unix)]
fn restrict_to_owner(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).with_context(|| format!("chmod {}", path.display()))
}

#[cfg(not(unix))]
fn restrict_to_owner(_path: &std::path::Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_profiles_with_ssh() {
        let dir = std::env::temp_dir().join(format!("dbviewer-test-{}", std::process::id()));
        let store = ProfileStore::at(dir.join("c.json"));
        assert!(store.load().unwrap().is_empty());
        let profile = Profile {
            name: "prod".into(),
            ssh: Some(SshConfig { host: "bastion".into(), port: Some(2222), user: "me".into(), identity_file: String::new(), secret: String::new() }),
            ..Profile::default()
        };
        store.save(std::slice::from_ref(&profile)).unwrap();
        assert_eq!(store.load().unwrap(), vec![profile]);
        std::fs::remove_dir_all(dir).ok();
    }
}
