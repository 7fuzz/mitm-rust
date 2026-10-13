use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::db::Db;
use super::model::*;
use super::store::*;

pub fn resolve_dynamic_variable(key: &str) -> Option<String> {
    let lower = key.trim().to_lowercase();
    let norm = lower.trim_start_matches('$');

    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let millis = duration.as_millis();

    match norm {
        "timestamp" => Some(secs.to_string()),
        "timestampms" | "timestamp_ms" => Some(millis.to_string()),
        "isotimestamp" | "iso_timestamp" => {
            let offset_dt = time::OffsetDateTime::from_unix_timestamp(secs as i64)
                .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
            Some(
                offset_dt
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| format!("{}Z", secs)),
            )
        }
        "currentdate" | "current_date" | "date" => {
            let offset_dt = time::OffsetDateTime::from_unix_timestamp(secs as i64)
                .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
            Some(format!(
                "{:04}-{:02}-{:02}",
                offset_dt.year(),
                offset_dt.month() as u8,
                offset_dt.day()
            ))
        }
        "currenttime" | "current_time" | "time" => {
            let offset_dt = time::OffsetDateTime::from_unix_timestamp(secs as i64)
                .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
            Some(format!(
                "{:02}:{:02}:{:02}",
                offset_dt.hour(),
                offset_dt.minute(),
                offset_dt.second()
            ))
        }
        "guid" | "uuid" | "uuidv4" | "uuid_v4" | "randomuuid" | "random_uuid" | "randomguid" | "random_guid" | "randomuuidv4" | "random_uuid_v4" => {
            Some(Uuid::new_v4().to_string())
        }
        "uuidv7" | "uuid_v7" | "guidv7" | "guid_v7" | "randomuuidv7" | "random_uuid_v7" => {
            Some(Uuid::now_v7().to_string())
        }
        "randomint" | "random_int" | "randominteger" | "random_integer" => {
            let u = Uuid::new_v4().as_u128();
            let val = (u % 1000) + 1;
            Some(val.to_string())
        }
        "randomdigit" | "random_digit" => {
            let u = Uuid::new_v4().as_u128();
            let val = u % 10;
            Some(val.to_string())
        }
        "randomalphanumeric" | "random_alphanumeric" => {
            let u = Uuid::new_v4().simple().to_string();
            Some(u[..8].to_string())
        }
        "randomhex" | "random_hex" => {
            let u = Uuid::new_v4().simple().to_string();
            Some(u[..16].to_string())
        }
        "randomprice" | "random_price" => {
            let u = Uuid::new_v4().as_u128();
            let whole = (u % 100) + 1;
            let cents = (u / 100) % 100;
            Some(format!("{}.{:02}", whole, cents))
        }
        "randomport" | "random_port" => {
            let u = Uuid::new_v4().as_u128();
            let port = 1024 + (u % 64000);
            Some(port.to_string())
        }
        "randomfirstname" | "random_first_name" => {
            let names = ["Alex", "Jordan", "Taylor", "Morgan", "Casey", "Riley", "Sam", "Chris", "Jamie", "Pat"];
            let u = Uuid::new_v4().as_u128() as usize;
            Some(names[u % names.len()].to_string())
        }
        "randomlastname" | "random_last_name" => {
            let names = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Miller", "Davis", "Wilson", "Moore", "Taylor"];
            let u = Uuid::new_v4().as_u128() as usize;
            Some(names[u % names.len()].to_string())
        }
        "randomfullname" | "random_full_name" | "randomname" | "random_name" => {
            let firsts = ["Alex", "Jordan", "Taylor", "Morgan", "Casey", "Riley", "Sam", "Chris", "Jamie", "Pat"];
            let lasts = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Miller", "Davis", "Wilson", "Moore", "Taylor"];
            let u = Uuid::new_v4().as_u128() as usize;
            Some(format!("{} {}", firsts[u % firsts.len()], lasts[(u / 10) % lasts.len()]))
        }
        "randomemail" | "random_email" => {
            let u = Uuid::new_v4().simple().to_string();
            Some(format!("user_{}@example.com", &u[..8]))
        }
        "randomphonenumber" | "random_phone_number" | "randomphone" | "random_phone" => {
            let u = Uuid::new_v4().as_u128();
            let num = (u % 900000000) + 100000000;
            Some(format!("+6281{}", num))
        }
        "randomusername" | "random_user_name" => {
            let u = Uuid::new_v4().simple().to_string();
            Some(format!("user_{}", &u[..6]))
        }
        "randompassword" | "random_password" => {
            let u = Uuid::new_v4().simple().to_string();
            Some(format!("Pass!{}", &u[..10]))
        }
        "randomcity" | "random_city" => {
            let cities = ["Jakarta", "Bandung", "Surabaya", "Medan", "Semarang", "Yogyakarta", "Denpasar", "Makassar"];
            let u = Uuid::new_v4().as_u128() as usize;
            Some(cities[u % cities.len()].to_string())
        }
        "randomcountry" | "random_country" => {
            let countries = ["Indonesia", "Singapore", "Malaysia", "Japan", "United States", "Germany", "Australia"];
            let u = Uuid::new_v4().as_u128() as usize;
            Some(countries[u % countries.len()].to_string())
        }
        "randomcountrycode" | "random_country_code" => {
            let codes = ["ID", "SG", "MY", "JP", "US", "DE", "AU"];
            let u = Uuid::new_v4().as_u128() as usize;
            Some(codes[u % codes.len()].to_string())
        }
        "randomipv4" | "random_ipv4" | "randomip" | "random_ip" => {
            let u = Uuid::new_v4().as_u128();
            Some(format!("{}.{}.{}.{}", (u >> 24) & 0xFF, (u >> 16) & 0xFF, (u >> 8) & 0xFF, u & 0xFF))
        }
        "randomboolean" | "random_boolean" | "randombool" | "random_bool" => {
            let u = Uuid::new_v4().as_u128();
            if u.is_multiple_of(2) {
                Some("true".to_string())
            } else {
                Some("false".to_string())
            }
        }
        _ => None,
    }
}

/// `$totp:adminSecret` → `adminSecret`.
fn totp_ref_name(key: &str) -> Option<&str> {
    let key = key.trim();
    let prefix = key.get(..6)?;
    prefix.eq_ignore_ascii_case("$totp:").then(|| key[6..].trim())
}

fn totp_code_from_secret(secret: &str) -> Option<String> {
    let params = crate::utils::totp::parse_secret(secret).ok()?;
    Some(crate::utils::totp::code_for_send(&params, crate::utils::totp::now_unix()))
}

fn env_var_value<'a>(env: &'a Environment, name: &str) -> Option<&'a str> {
    let name = name.trim();
    env.variables
        .iter()
        .find(|v| v.enabled && v.key.trim() == name)
        .or_else(|| env.variables.iter().find(|v| v.enabled && v.key.trim().eq_ignore_ascii_case(name)))
        .map(|v| v.current_value())
}

/// The TOTP parameters behind a `$totp:<var>` reference, for showing a live code.
pub fn totp_params_in_env(env: Option<&Environment>, key: &str) -> Option<crate::utils::totp::TotpParams> {
    let name = totp_ref_name(key)?;
    crate::utils::totp::parse_secret(env_var_value(env?, name)?).ok()
}

/// Where `{{$cred:vault/entry.field}}` refs are looked up: the vaults linked to `workspace_id`.
#[derive(Debug, Clone, Copy)]
pub struct CredentialScope<'a> {
    pub db: &'a Db,
    pub workspace_id: &'a str,
}

/// What an interpolation substituted from credentials, so the logged copy of a request can be masked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Interpolation {
    pub secrets: Vec<String>,
    /// A `$cred:` ref stayed unresolved because credentials are locked.
    pub locked: bool,
}

pub const MASK: &str = "••••";

impl Interpolation {
    /// Replaces every secret this interpolation substituted, raw or URL-encoded, so it can be logged.
    pub fn mask(&self, text: &str) -> String {
        if self.secrets.is_empty() {
            return text.to_string();
        }
        let mut needles: Vec<String> = self
            .secrets
            .iter()
            .flat_map(|s| {
                let form: String = url::form_urlencoded::byte_serialize(s.as_bytes()).collect();
                [s.clone(), urlencoding::encode(s).into_owned(), form]
            })
            .filter(|n| !n.is_empty())
            .collect();
        needles.sort_by_key(|n| std::cmp::Reverse(n.len()));
        needles.dedup();
        needles.iter().fold(text.to_string(), |out, needle| out.replace(needle.as_str(), MASK))
    }
}

pub fn interpolate_variables_with_env(env: &Environment, text: &str) -> String {
    interpolate(Some(env), None, text, &mut Interpolation::default())
}

/// Fills `{{vars}}` from `env`, credentials from `creds` and dynamic variables, up to 15 levels deep.
pub fn interpolate(
    env: Option<&Environment>,
    creds: Option<CredentialScope>,
    text: &str,
    record: &mut Interpolation,
) -> String {
    if !text.contains("{{") || !text.contains("}}") {
        return text.to_string();
    }

    let mut var_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut lower_var_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for var in env.map(|e| e.variables.as_slice()).unwrap_or_default() {
        if var.enabled && !var.key.trim().is_empty() {
            let k = var.key.trim().to_string();
            let v = var.current_value().to_string();
            lower_var_map.insert(k.to_lowercase(), v.clone());
            var_map.insert(k, v);
        }
    }

    let re = match regex::Regex::new(r"\{\{\s*([^\r\n{}]+?)\s*\}\}") {
        Ok(r) => r,
        Err(_) => return text.to_string(),
    };

    let mut result = text.to_string();
    const MAX_DEPTH: usize = 15;

    for _ in 0..MAX_DEPTH {
        if !result.contains("{{") || !result.contains("}}") {
            break;
        }

        let mut changed = false;
        let next_result = re
            .replace_all(&result, |caps: &regex::Captures| {
                let key = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                let unresolved = || caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
                if let Some(val) = var_map.get(key) {
                    changed = true;
                    val.to_string()
                } else if let Some(val) = lower_var_map.get(&key.to_lowercase()) {
                    changed = true;
                    val.to_string()
                } else if let Some(name) = totp_ref_name(key) {
                    let secret = var_map.get(name).or_else(|| lower_var_map.get(&name.to_lowercase()));
                    match secret.and_then(|s| totp_code_from_secret(s)) {
                        Some(code) => {
                            changed = true;
                            code
                        }
                        None => unresolved(),
                    }
                } else if let Some(cred) = crate::credentials::parse_ref(key) {
                    let Some(scope) = creds else { return unresolved() };
                    match crate::credentials::resolve(scope.db, scope.workspace_id, cred) {
                        crate::credentials::Resolve::Value(value, kind) => {
                            if kind == crate::credentials::FieldKind::Secret && !record.secrets.contains(&value) {
                                record.secrets.push(value.clone());
                            }
                            changed = true;
                            value
                        }
                        crate::credentials::Resolve::Locked => {
                            record.locked = true;
                            unresolved()
                        }
                        crate::credentials::Resolve::Missing => unresolved(),
                    }
                } else if let Some(val) = resolve_dynamic_variable(key) {
                    changed = true;
                    val
                } else {
                    unresolved()
                }
            })
            .to_string();

        if !changed || next_result == result {
            break;
        }
        result = next_result;
    }

    result
}

pub fn interpolate_dynamic_variables(text: &str) -> String {
    if !text.contains("{{") || !text.contains("}}") {
        return text.to_string();
    }

    let re = match regex::Regex::new(r"\{\{\s*([^\r\n{}]+?)\s*\}\}") {
        Ok(r) => r,
        Err(_) => return text.to_string(),
    };

    re.replace_all(text, |caps: &regex::Captures| {
        let key = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        if let Some(val) = resolve_dynamic_variable(key) {
            val
        } else {
            caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
        }
    })
    .to_string()
}

pub fn interpolate_variables(db: &Db, workspace_id: &str, text: &str) -> String {
    if !text.contains("{{") || !text.contains("}}") {
        return text.to_string();
    }
    let env = active_environment_db(db, workspace_id);
    let scope = CredentialScope { db, workspace_id };
    interpolate(env.as_ref(), Some(scope), text, &mut Interpolation::default())
}

pub fn active_environment_db(db: &Db, workspace_id: &str) -> Option<Environment> {
    get_workspace_environments_db(db, workspace_id)
        .unwrap_or_default()
        .into_iter()
        .find(|e| e.is_active)
}

pub fn find_unresolved_variables(text: &str) -> Vec<Unresolved> {
    if !text.contains("{{") || !text.contains("}}") {
        return Vec::new();
    }
    let re = match regex::Regex::new(r"\{\{\s*([^\r\n{}]+?)\s*\}\}") {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut seen = std::collections::HashSet::new();
    let mut unresolved = Vec::new();
    for caps in re.captures_iter(text) {
        let key = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        if !key.is_empty() && seen.insert(key.to_string()) {
            unresolved.push(Unresolved { name: key.to_string() });
        }
    }
    unresolved
}

pub fn resolve_variables_with_env(
    env: Option<&Environment>,
    text: &str,
) -> (String, Vec<Unresolved>) {
    let resolved = match env {
        Some(e) => interpolate_variables_with_env(e, text),
        None => interpolate_dynamic_variables(text),
    };
    let unresolved = find_unresolved_variables(&resolved);
    (resolved, unresolved)
}

pub fn variable_value_in_env(env: Option<&Environment>, name: &str) -> Option<String> {
    let trimmed = name.trim();
    if let Some(value) = env.and_then(|e| env_var_value(e, trimmed)) {
        return Some(value.to_string());
    }
    if let Some(var) = totp_ref_name(trimmed) {
        return env.and_then(|e| env_var_value(e, var)).and_then(totp_code_from_secret);
    }
    resolve_dynamic_variable(trimmed)
}
