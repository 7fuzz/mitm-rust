use std::sync::Arc;

use base64::Engine;
use dashmap::DashMap;
use hyper::header::PROXY_AUTHORIZATION;
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::db::Db;

const TOKEN_PREFERENCE: &str = "container_proxy_token";
const CONTAINERS_PREFERENCE: &str = "containers";

/// Which container a flow belongs to; empty for traffic that sent no proxy credentials.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContainerTag {
    pub label: String,
    pub run_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub name: String,
    pub run_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyAuth {
    Anonymous,
    Container(ContainerTag),
    Rejected,
}

/// Containers the fork registered, and the shared token its proxy credentials carry as password.
#[derive(Clone)]
pub struct Containers {
    entries: Arc<DashMap<String, ContainerInfo>>,
    token: Arc<str>,
}

impl Containers {
    pub fn load(db: &Db) -> Self {
        let token = crate::db::get_preference(db, TOKEN_PREFERENCE)
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| {
                let token = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
                let _ = crate::db::set_preference(db, TOKEN_PREFERENCE, &token);
                token
            });
        let saved: std::collections::BTreeMap<String, ContainerInfo> = crate::db::get_preference(db, CONTAINERS_PREFERENCE)
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        Self { entries: Arc::new(saved.into_iter().collect()), token: token.into() }
    }

    pub fn save(&self, db: &Db) -> Result<(), String> {
        let all: std::collections::BTreeMap<String, ContainerInfo> = self.list().into_iter().collect();
        let json = serde_json::to_string(&all).map_err(|e| e.to_string())?;
        crate::db::set_preference(db, CONTAINERS_PREFERENCE, &json)
    }

    pub fn list(&self) -> Vec<(String, ContainerInfo)> {
        let mut all: Vec<_> = self.entries.iter().map(|e| (e.key().clone(), e.value().clone())).collect();
        all.sort_by(|a, b| a.0.cmp(&b.0));
        all
    }

    pub fn info(&self, id: &str) -> Option<ContainerInfo> {
        self.entries.get(id).map(|e| e.value().clone())
    }

    pub fn remove(&self, id: &str) -> bool {
        self.entries.remove(id).is_some()
    }

    /// Begins a new run for the container, registering it under its id when unknown.
    pub fn start_run(&self, id: &str) -> i64 {
        let mut info = self.entries.entry(id.to_string()).or_insert_with(|| ContainerInfo { name: id.to_string(), run_id: 0 });
        info.run_id += 1;
        info.run_id
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// Returns the previous label when this renamed a container, so stored flows can follow.
    pub fn register(&self, id: &str, name: &str) -> Option<String> {
        let mut renamed_from = None;
        self.entries
            .entry(id.to_string())
            .and_modify(|info| {
                if info.name != name {
                    renamed_from = Some(std::mem::replace(&mut info.name, name.to_string()));
                }
            })
            .or_insert_with(|| ContainerInfo { name: name.to_string(), run_id: 0 });
        renamed_from
    }

    /// Registered name when there is one, else the raw id the client sent.
    pub fn tag(&self, id: &str) -> ContainerTag {
        match self.entries.get(id) {
            Some(info) => ContainerTag { label: info.name.clone(), run_id: info.run_id },
            None => ContainerTag { label: id.to_string(), run_id: 0 },
        }
    }

    /// Never a 407 challenge: a browser that answers one through a prompt would share the credentials across containers.
    pub fn authenticate(&self, headers: &HeaderMap) -> ProxyAuth {
        let Some(value) = headers.get(PROXY_AUTHORIZATION) else {
            return ProxyAuth::Anonymous;
        };
        match value.to_str().ok().and_then(parse_basic) {
            Some((id, password)) if !id.is_empty() && same_secret(&password, &self.token) => ProxyAuth::Container(self.tag(&id)),
            _ => ProxyAuth::Rejected,
        }
    }
}

fn parse_basic(value: &str) -> Option<(String, String)> {
    let (scheme, encoded) = value.trim().split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("basic") {
        return None;
    }
    let decoded = base64::engine::general_purpose::STANDARD.decode(encoded.trim()).ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    let (user, password) = decoded.split_once(':')?;
    Some((user.to_string(), password.to_string()))
}

fn same_secret(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn containers() -> Containers {
        Containers { entries: Arc::new(DashMap::new()), token: "secret".into() }
    }

    fn headers(value: Option<&str>) -> HeaderMap {
        let mut map = HeaderMap::new();
        if let Some(value) = value {
            map.insert(PROXY_AUTHORIZATION, value.parse().unwrap());
        }
        map
    }

    fn basic(user: &str, password: &str) -> String {
        format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}")))
    }

    #[test]
    fn no_header_is_anonymous() {
        assert_eq!(containers().authenticate(&headers(None)), ProxyAuth::Anonymous);
    }

    #[test]
    fn right_token_labels_with_registered_name_or_raw_id() {
        let containers = containers();
        let tag = |label: &str, run_id| ProxyAuth::Container(ContainerTag { label: label.into(), run_id });
        assert_eq!(containers.authenticate(&headers(Some(&basic("7", "secret")))), tag("7", 0));
        let _ = containers.register("7", "App A · admin");
        assert_eq!(containers.authenticate(&headers(Some(&basic("7", "secret")))), tag("App A · admin", 0));
    }

    #[test]
    fn wrong_token_or_malformed_header_is_rejected() {
        let containers = containers();
        for value in [basic("7", "nope"), basic("", "secret"), "Basic !!!".to_string(), "Bearer secret".to_string(), "Basic".to_string()] {
            assert_eq!(containers.authenticate(&headers(Some(&value))), ProxyAuth::Rejected, "{value}");
        }
    }

    #[test]
    fn registry_survives_a_reload_and_runs_count_up_per_container() {
        let dir = std::env::temp_dir().join(format!("mitm-containers-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = crate::db::Db::new(dir.join("test.db"));
        crate::db::init_database(&db, None).unwrap();
        let containers = Containers::load(&db);
        assert_eq!(containers.register("7", "App A"), None);
        assert_eq!(containers.register("7", "App A"), None);
        assert_eq!(containers.register("7", "App A · admin"), Some("App A".to_string()));
        assert_eq!(containers.start_run("7"), 1);
        assert_eq!(containers.start_run("7"), 2);
        assert_eq!(containers.start_run("9"), 1);
        containers.save(&db).unwrap();

        let reloaded = Containers::load(&db);
        assert_eq!(reloaded.info("7"), Some(ContainerInfo { name: "App A · admin".into(), run_id: 2 }));
        assert_eq!(reloaded.tag("9"), ContainerTag { label: "9".into(), run_id: 1 });
        assert!(reloaded.remove("9") && !reloaded.remove("9"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn token_is_generated_once_and_kept() {
        let dir = std::env::temp_dir().join(format!("mitm-containers-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = crate::db::Db::new(dir.join("test.db"));
        crate::db::init_database(&db, None).unwrap();
        let first = Containers::load(&db).token().to_string();
        assert_eq!(first.len(), 64);
        assert_eq!(Containers::load(&db).token(), first);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
