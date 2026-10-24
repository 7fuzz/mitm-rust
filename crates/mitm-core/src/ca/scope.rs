use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const SCOPE_FILE: &str = "scope.json";

/// What the CA may issue certificates for. A domain-locked CA carries an X.509 name-constraints
/// extension, so a stolen key can't mint certificates for other sites.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", content = "domains", rename_all = "lowercase")]
pub enum CaScope {
    #[default]
    Global,
    Domains(Vec<String>),
}

impl CaScope {
    /// Domain-locked scope from user input; `*.example.com` and `example.com` both permit the name and its subdomains.
    pub fn domains<S: AsRef<str>>(entries: &[S]) -> Result<Self, String> {
        let mut domains: Vec<String> = Vec::new();
        for entry in entries {
            let domain = entry.as_ref().trim().trim_start_matches("*.").trim_matches('.').to_lowercase();
            if domain.is_empty() {
                continue;
            }
            let valid = domain.contains('.') || domain == "localhost";
            if !valid || domain.chars().any(|c| !(c.is_ascii_alphanumeric() || c == '.' || c == '-')) {
                return Err(format!("'{}' is not a domain", entry.as_ref().trim()));
            }
            if !domains.contains(&domain) {
                domains.push(domain);
            }
        }
        if domains.is_empty() {
            return Err("A domain-locked CA needs at least one domain".to_string());
        }
        Ok(Self::Domains(domains))
    }

    pub fn permits(&self, host: &str) -> bool {
        match self {
            Self::Global => true,
            Self::Domains(domains) => {
                let host = host.trim_end_matches('.').to_lowercase();
                domains.iter().any(|d| host == *d || host.ends_with(&format!(".{d}")))
            }
        }
    }

    pub fn domain_list(&self) -> &[String] {
        match self {
            Self::Global => &[],
            Self::Domains(domains) => domains,
        }
    }

    pub fn load(ca_dir: &Path) -> Self {
        fs::read_to_string(ca_dir.join(SCOPE_FILE))
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, ca_dir: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(ca_dir.join(SCOPE_FILE), json).map_err(|e| format!("Failed to save CA scope: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains_are_normalised_and_checked() {
        let scope = CaScope::domains(&["*.Test.Example.com", " test.example.com ", "localhost", ""]).unwrap();
        assert_eq!(scope.domain_list(), ["test.example.com", "localhost"]);
        assert!(CaScope::domains::<&str>(&[]).is_err());
        assert!(CaScope::domains(&["nodots"]).is_err());
        assert!(CaScope::domains(&["bad domain.com"]).is_err());
    }

    #[test]
    fn permits_the_name_and_its_subdomains_only() {
        let scope = CaScope::domains(&["example.com"]).unwrap();
        assert!(scope.permits("example.com"));
        assert!(scope.permits("API.Example.com."));
        assert!(!scope.permits("notexample.com"));
        assert!(!scope.permits("example.com.evil.test"));
        assert!(CaScope::Global.permits("anything.test"));
    }

    #[test]
    fn scope_round_trips_through_json() {
        let scope = CaScope::domains(&["example.com"]).unwrap();
        let json = serde_json::to_string(&scope).unwrap();
        assert_eq!(json, r#"{"scope":"domains","domains":["example.com"]}"#);
        assert_eq!(serde_json::from_str::<CaScope>(&json).unwrap(), scope);
        assert_eq!(serde_json::to_string(&CaScope::Global).unwrap(), r#"{"scope":"global"}"#);
    }
}
