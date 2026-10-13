use super::store::{find_credential, validate_name, FieldKind};
use super::CredentialError;
use crate::utils::totp;
use crate::db::Db;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolve {
    Value(String, FieldKind),
    Locked,
    Missing,
}

/// A `$cred:vault/entry.field` ref's parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CredRef<'a> {
    pub vault: &'a str,
    pub entry: &'a str,
    pub field: &'a str,
}

/// `$cred:DG/dgAdmin.password` → `DG`, `dgAdmin`, `password`.
pub fn parse_ref(key: &str) -> Option<CredRef<'_>> {
    let key = key.trim();
    let prefix = key.get(..6)?;
    if !prefix.eq_ignore_ascii_case("$cred:") {
        return None;
    }
    let (vault, rest) = key[6..].trim().split_once('/')?;
    let (entry, field) = rest.split_once('.')?;
    let valid = [vault, entry, field].iter().all(|part| validate_name(part).is_ok());
    valid.then_some(CredRef { vault, entry, field })
}

/// The ref when `value` is exactly one `{{$cred:vault/entry.field}}`, padded or not.
pub fn link_target(value: &str) -> Option<CredRef<'_>> {
    let inner = value.trim().strip_prefix("{{")?.strip_suffix("}}")?;
    parse_ref(inner)
}

pub fn is_link(value: &str) -> bool {
    link_target(value).is_some()
}

pub fn format_ref(vault: &str, entry: &str, field: &str) -> String {
    format!("{{{{$cred:{vault}/{entry}.{field}}}}}")
}

/// A field's value as sent: `totp` fields give the current code, not their secret.
/// Only vaults linked to the workspace are searched.
pub fn resolve(db: &Db, workspace_id: &str, cred: CredRef) -> Resolve {
    let credential = match find_credential(db, workspace_id, cred.vault, cred.entry) {
        Ok(Some(c)) => c,
        Ok(None) => return Resolve::Missing,
        Err(CredentialError::Locked) => return Resolve::Locked,
        Err(_) => return Resolve::Missing,
    };
    let Some(field) = credential.field(cred.field) else { return Resolve::Missing };
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
    use crate::credentials::store::tests::{field, temp_db, vault, workspace};
    use crate::credentials::{create_credential, save_credential};

    fn cred<'a>(vault: &'a str, entry: &'a str, field: &'a str) -> CredRef<'a> {
        CredRef { vault, entry, field }
    }

    #[test]
    fn parses_refs() {
        assert_eq!(parse_ref("$cred:DG/dgAdmin.password"), Some(cred("DG", "dgAdmin", "password")));
        assert_eq!(parse_ref(" $CRED:v_1/a-b.c_d "), Some(cred("v_1", "a-b", "c_d")));
        assert_eq!(parse_ref("$cred:dgAdmin.password"), None);
        assert_eq!(parse_ref("$cred:DG/dgAdmin"), None);
        assert_eq!(parse_ref("$cred:DG/a.b.c"), None);
        assert_eq!(parse_ref("$cred:a/b/c.d"), None);
        assert_eq!(parse_ref("cred:DG/a.b"), None);
        assert_eq!(format_ref("DG", "dgAdmin", "password"), "{{$cred:DG/dgAdmin.password}}");
        assert_eq!(link_target(" {{ $cred:DG/dgAdmin.password }} "), Some(cred("DG", "dgAdmin", "password")));
        assert_eq!(link_target("x{{$cred:DG/dgAdmin.password}}"), None);
        assert_eq!(link_target("{{adminPassword}}"), None);
    }

    #[test]
    fn resolves_values_codes_and_missing() {
        let db = temp_db();
        let ws = workspace(&db, "W");
        let dg = vault(&db, &ws, "DG");
        let mut entry = create_credential(&db, &dg, "dgAdmin").unwrap();
        entry.fields = vec![
            field("password", FieldKind::Secret, "hunter2"),
            field("totp", FieldKind::Totp, "MZXW6"),
            field("broken", FieldKind::Totp, "!!"),
        ];
        save_credential(&db, &entry).unwrap();

        assert_eq!(resolve(&db, &ws, cred("dg", "DGADMIN", "Password")), Resolve::Value("hunter2".into(), FieldKind::Secret));
        match resolve(&db, &ws, cred("DG", "dgAdmin", "totp")) {
            Resolve::Value(code, FieldKind::Totp) => assert_eq!(code.len(), 6),
            other => panic!("{other:?}"),
        }
        assert_eq!(resolve(&db, &ws, cred("DG", "dgAdmin", "broken")), Resolve::Missing);
        assert_eq!(resolve(&db, &ws, cred("DG", "dgAdmin", "nope")), Resolve::Missing);
        assert_eq!(resolve(&db, &ws, cred("DG", "nobody", "password")), Resolve::Missing);
        assert_eq!(resolve(&db, &ws, cred("Other", "dgAdmin", "password")), Resolve::Missing);
        let unlinked = workspace(&db, "U");
        assert_eq!(resolve(&db, &unlinked, cred("DG", "dgAdmin", "password")), Resolve::Missing);
        let _ = std::fs::remove_file(&db);
    }
}
