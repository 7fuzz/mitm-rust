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
    /// Stored in plain text in the app database; empty means ask on connect.
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
