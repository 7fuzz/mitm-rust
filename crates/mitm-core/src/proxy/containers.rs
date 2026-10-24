use std::sync::Arc;

use base64::Engine;
use dashmap::DashMap;
use hyper::header::PROXY_AUTHORIZATION;
use hyper::HeaderMap;

use crate::db::Db;

const TOKEN_PREFERENCE: &str = "container_proxy_token";

/// Which container a flow belongs to; empty for traffic that sent no proxy credentials.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContainerTag {
    pub label: String,
    pub run_id: i64,
}

#[derive(Debug, Clone)]
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
        Self { entries: Arc::new(DashMap::new()), token: token.into() }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn register(&self, id: &str, name: &str) {
        self.entries
            .entry(id.to_string())
            .and_modify(|info| info.name = name.to_string())
            .or_insert_with(|| ContainerInfo { name: name.to_string(), run_id: 0 });
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
        containers.register("7", "App A · admin");
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
