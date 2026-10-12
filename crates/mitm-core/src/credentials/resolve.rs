use std::path::Path;
use super::store::{find_credential, validate_name, FieldKind};
use super::LOCKED;
use crate::utils::totp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolve {
    Value(String, FieldKind),
    Locked,
    Missing,
}

/// `$cred:dgAdmin.password` → `("dgAdmin", "password")`.
pub fn parse_ref(key: &str) -> Option<(&str, &str)> {
    let key = key.trim();
    let prefix = key.get(..6)?;
    if !prefix.eq_ignore_ascii_case("$cred:") {
        return None;
    }
    let (entry, field) = key[6..].trim().split_once('.')?;
    (validate_name(entry).is_ok() && validate_name(field).is_ok()).then_some((entry, field))
}

/// `(entry, field)` when `value` is exactly one `{{$cred:entry.field}}` ref, padded or not.
pub fn link_target(value: &str) -> Option<(&str, &str)> {
    let inner = value.trim().strip_prefix("{{")?.strip_suffix("}}")?;
    parse_ref(inner)
}

pub fn is_link(value: &str) -> bool {
    link_target(value).is_some()
}

pub fn format_ref(entry: &str, field: &str) -> String {
    format!("{{{{$cred:{entry}.{field}}}}}")
}

/// A field's value as sent: `totp` fields give the current code, not their secret.
pub fn resolve(db_path: &Path, workspace_id: &str, entry: &str, field: &str) -> Resolve {
    let credential = match find_credential(db_path, workspace_id, entry) {
        Ok(Some(c)) => c,
        Ok(None) => return Resolve::Missing,
        Err(e) if e == LOCKED => return Resolve::Locked,
        Err(_) => return Resolve::Missing,
    };
    let Some(field) = credential.field(field) else { return Resolve::Missing };
    match field.kind {
        FieldKind::Totp => match totp::parse_secret(&field.value) {
            Ok(params) => Resolve::Value(totp::code_for_send(&params, totp::now_unix()), FieldKind::Totp),
            Err(_) => Resolve::Missing,
        },
        kind => Resolve::Value(field.value.clone(), kind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::store::tests::{field, temp_db, workspace};
    use crate::credentials::{create_credential, save_credential};

    #[test]
    fn parses_refs() {
        assert_eq!(parse_ref("$cred:dgAdmin.password"), Some(("dgAdmin", "password")));
        assert_eq!(parse_ref(" $CRED:a-b.c_d "), Some(("a-b", "c_d")));
        assert_eq!(parse_ref("$cred:dgAdmin"), None);
        assert_eq!(parse_ref("$cred:a.b.c"), None);
        assert_eq!(parse_ref("cred:a.b"), None);
        assert_eq!(format_ref("dgAdmin", "password"), "{{$cred:dgAdmin.password}}");
        assert_eq!(link_target(" {{ $cred:dgAdmin.password }} "), Some(("dgAdmin", "password")));
        assert_eq!(link_target("x{{$cred:dgAdmin.password}}"), None);
        assert_eq!(link_target("{{adminPassword}}"), None);
    }

    #[test]
    fn resolves_values_codes_and_missing() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let mut cred = create_credential(&db, &ws, "dgAdmin").unwrap();
        cred.fields = vec![
            field("password", FieldKind::Secret, "hunter2"),
            field("totp", FieldKind::Totp, "MZXW6"),
            field("broken", FieldKind::Totp, "!!"),
        ];
        save_credential(&db, &cred).unwrap();

        assert_eq!(resolve(&db, &ws, "DGADMIN", "Password"), Resolve::Value("hunter2".into(), FieldKind::Secret));
        match resolve(&db, &ws, "dgAdmin", "totp") {
            Resolve::Value(code, FieldKind::Totp) => assert_eq!(code.len(), 6),
            other => panic!("{other:?}"),
        }
        assert_eq!(resolve(&db, &ws, "dgAdmin", "broken"), Resolve::Missing);
        assert_eq!(resolve(&db, &ws, "dgAdmin", "nope"), Resolve::Missing);
        assert_eq!(resolve(&db, &ws, "nobody", "password"), Resolve::Missing);
        let _ = std::fs::remove_file(&db);
    }
}
