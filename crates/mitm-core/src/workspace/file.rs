//! The `mitm-workspace` file: a workspace's environments and collections, as written by export and read
//! by import. Field names are the documented file format (see `docs/WORKSPACE_FILE.md`), so renaming one
//! breaks existing files and needs a new `version`.

use serde::{Deserialize, Serialize};
use super::{EnvironmentVariable, VariableVariant};

pub const FORMAT: &str = "mitm-workspace";
pub const VERSION: u32 = 1;
/// The variant that extract rules write into; import adds it, empty, to every variable.
pub const AUTO_VARIANT: &str = "(auto)";
/// The variant a variable's plain `value` becomes.
pub const DEFAULT_VARIANT: &str = "default";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceFile {
    /// Lets editors validate the file against a JSON Schema; ignored by import.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub format: String,
    pub version: u32,
    pub workspace: WorkspaceInfo,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub environments: Vec<EnvironmentEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collections: Vec<CollectionEntry>,
    /// The vaults linked to the workspace, with their entries but no values, so refs to them re-link on import.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vaults: Vec<VaultEntry>,
    /// Entries from files made before vaults; import puts them in a vault named after the workspace.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credentials: Vec<CredentialEntry>,
    /// Credential values and notes, encrypted with a passphrase chosen at export.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentials_sealed: Option<SealedCredentials>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultEntry {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credentials: Vec<CredentialEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialEntry {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<crate::credentials::FieldMeta>,
}

/// `ciphertext` is AES-256-GCM over the JSON of `[{vault, name, values: {key: value}, notes}]`, keyed by argon2id of the
/// passphrase. Binary parts are base64.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SealedCredentials {
    pub kdf: String,
    pub params: crate::credentials::crypto::KdfParams,
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvironmentEntry {
    pub name: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub active: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<VariableEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariableEntry {
    pub key: String,
    /// Imported as the `default` variant (`default1`, … when that name is taken).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub value: String,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<VariantEntry>,
    /// Which variant is in use; without it the first imported one is, or `(auto)` when there are none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_variant: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariantEntry {
    pub name: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CollectionEntry {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requests: Vec<RequestEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collections: Vec<CollectionEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestEntry {
    /// Unique within the file; `before` and `after` point at it.
    #[serde(rename = "ref", default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    pub name: String,
    #[serde(default = "get")]
    pub method: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<KeyValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<KeyValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Body>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extract: Vec<ExtractEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeyValue {
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Body {
    Json { content: String },
    Raw { content: String },
    Urlencoded { fields: Vec<KeyValue> },
    Form { fields: Vec<FormField> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FormField {
    pub name: String,
    /// Text, a file path for `file`, or base64 bytes for `base64`.
    #[serde(default)]
    pub value: String,
    #[serde(rename = "type", default, skip_serializing_if = "FormFieldType::is_text")]
    pub kind: FormFieldType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FormFieldType {
    #[default]
    Text,
    File,
    Base64,
}

impl FormFieldType {
    fn is_text(&self) -> bool {
        *self == FormFieldType::Text
    }

    /// The `type` stored in a multipart field spec.
    pub fn stored(self) -> &'static str {
        match self {
            FormFieldType::Text => "text",
            FormFieldType::File => "file_path",
            FormFieldType::Base64 => "base64",
        }
    }

    pub fn from_stored(stored: &str) -> Self {
        match stored {
            "file_path" => FormFieldType::File,
            "base64" => FormFieldType::Base64,
            _ => FormFieldType::Text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtractEntry {
    pub variable: String,
    pub from: ExtractSource,
    pub expression: String,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtractSource {
    Json,
    Header,
    Regex,
    After,
    Before,
    Between,
}

impl ExtractSource {
    /// The rule `type` the collection runner understands.
    pub fn stored(self) -> &'static str {
        match self {
            ExtractSource::Json => "json",
            ExtractSource::Header => "header",
            ExtractSource::Regex => "body_regex",
            ExtractSource::After => "after_string",
            ExtractSource::Before => "before_string",
            ExtractSource::Between => "between_string",
        }
    }

    pub fn from_stored(stored: &str) -> Option<Self> {
        Some(match stored {
            "json" | "jsonpath" | "json_path" | "body_json" => ExtractSource::Json,
            "header" | "headers" => ExtractSource::Header,
            "body_regex" | "regex" => ExtractSource::Regex,
            "after_string" => ExtractSource::After,
            "before_string" => ExtractSource::Before,
            "between_string" => ExtractSource::Between,
            _ => return None,
        })
    }
}

impl WorkspaceFile {
    pub fn new(workspace: WorkspaceInfo) -> Self {
        Self {
            schema: None,
            format: FORMAT.to_string(),
            version: VERSION,
            workspace,
            environments: Vec::new(),
            collections: Vec::new(),
            vaults: Vec::new(),
            credentials: Vec::new(),
            credentials_sealed: None,
        }
    }

    /// Parses and checks the file: format and version, and that every `before`/`after` names a `ref`.
    pub fn parse(json: &str) -> Result<Self, String> {
        let file: WorkspaceFile = serde_json::from_str(json).map_err(|e| format!("Invalid workspace file: {e}"))?;
        if file.format != FORMAT {
            return Err(format!("Not a workspace file: \"format\" is \"{}\", expected \"{FORMAT}\"", file.format));
        }
        if file.version > VERSION {
            return Err(format!(
                "This file is version {}; this app reads up to version {VERSION}. Update the app to import it.",
                file.version
            ));
        }
        file.check_refs()?;
        file.check_active_variants()?;
        Ok(file)
    }

    fn check_refs(&self) -> Result<(), String> {
        let mut refs = std::collections::HashSet::new();
        for request in self.requests() {
            if let Some(reference) = &request.reference {
                if !refs.insert(reference.as_str()) {
                    return Err(format!("Two requests use the ref \"{reference}\""));
                }
            }
        }
        for request in self.requests() {
            for step in request.before.iter().chain(&request.after) {
                if !refs.contains(step.as_str()) {
                    return Err(format!("Request \"{}\" runs \"{step}\", but no request has that ref", request.name));
                }
                if request.reference.as_deref() == Some(step.as_str()) {
                    return Err(format!("Request \"{}\" can't run itself", request.name));
                }
            }
        }
        Ok(())
    }

    fn check_active_variants(&self) -> Result<(), String> {
        for env in &self.environments {
            for var in &env.variables {
                let Some(active) = &var.active_variant else { continue };
                if !var.to_variable().variants.iter().any(|v| &v.name == active) {
                    return Err(format!(
                        "Variable \"{}\" in \"{}\": activeVariant \"{active}\" isn't one of its variants",
                        var.key, env.name
                    ));
                }
            }
        }
        Ok(())
    }

    /// Every request, depth first.
    pub fn requests(&self) -> Vec<&RequestEntry> {
        fn walk<'a>(collections: &'a [CollectionEntry], out: &mut Vec<&'a RequestEntry>) {
            for collection in collections {
                out.extend(&collection.requests);
                walk(&collection.collections, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.collections, &mut out);
        out
    }
}

impl VariableEntry {
    /// `(auto)` first (empty unless the file lists one), then `value` as `default`, then the other variants.
    pub fn to_variable(&self) -> EnvironmentVariable {
        let auto = self.variants.iter().find(|v| v.name == AUTO_VARIANT).map_or_else(String::new, |v| v.value.clone());
        let mut variants = vec![VariableVariant { name: AUTO_VARIANT.to_string(), value: auto }];
        if !self.value.is_empty() {
            variants.push(VariableVariant { name: self.free_default_name(), value: self.value.clone() });
        }
        variants.extend(
            self.variants
                .iter()
                .filter(|v| v.name != AUTO_VARIANT)
                .map(|v| VariableVariant { name: v.name.clone(), value: v.value.clone() }),
        );
        let active_index = match &self.active_variant {
            Some(name) => variants.iter().position(|v| &v.name == name).unwrap_or(0),
            None => usize::from(variants.len() > 1),
        };
        EnvironmentVariable {
            key: self.key.clone(),
            value: variants[active_index].value.clone(),
            enabled: self.enabled,
            r#type: if self.secret { "secret" } else { "default" }.to_string(),
            active_index,
            variants,
        }
    }

    fn free_default_name(&self) -> String {
        let taken = |name: &str| self.variants.iter().any(|v| v.name == name);
        if !taken(DEFAULT_VARIANT) {
            return DEFAULT_VARIANT.to_string();
        }
        (1..).map(|n| format!("{DEFAULT_VARIANT}{n}")).find(|name| !taken(name)).unwrap()
    }

    /// The reverse of `to_variable`, leaving out what `options` excludes.
    pub fn from_variable(var: &EnvironmentVariable, options: ExportOptions) -> Self {
        let secret = var.r#type == "secret";
        // A credential link holds no secret itself; keeping it lets the variable re-link on import.
        let is_link = |value: &str| crate::credentials::is_link(value);
        let shown = |value: &str| {
            if secret && !options.include_secrets && !is_link(value) { String::new() } else { value.to_string() }
        };
        let mut entry = Self {
            key: var.key.clone(),
            value: String::new(),
            enabled: var.enabled,
            secret,
            variants: Vec::new(),
            active_variant: None,
        };
        if var.variants.is_empty() {
            entry.value = shown(&var.value);
            return entry;
        }
        entry.variants = var
            .variants
            .iter()
            .map(|v| VariantEntry { name: v.name.clone(), value: shown(&v.value) })
            .filter(|v| v.name != AUTO_VARIANT || is_link(&v.value) || (options.include_auto && !v.value.is_empty()))
            .collect();
        let active = var.variants.get(var.active_index).map_or(AUTO_VARIANT, |v| v.name.as_str());
        let implied = entry.variants.iter().map(|v| v.name.as_str()).find(|n| *n != AUTO_VARIANT).unwrap_or(AUTO_VARIANT);
        entry.active_variant = (active != implied).then(|| active.to_string());
        entry
    }
}

/// What export writes besides the workspace's structure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExportOptions {
    /// Values of secret variables; left blank otherwise.
    pub include_secrets: bool,
    /// `(auto)` values, which extract rules wrote during runs; left out otherwise.
    pub include_auto: bool,
}

/// A file-local id from a request name: lowercase words joined by `-`, numbered when taken.
pub fn make_ref(name: &str, taken: &mut std::collections::HashSet<String>) -> String {
    let slug = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("-");
    let base = if slug.is_empty() { "request".to_string() } else { slug };
    let mut candidate = base.clone();
    let mut n = 2;
    while !taken.insert(candidate.clone()) {
        candidate = format!("{base}-{n}");
        n += 1;
    }
    candidate
}

fn yes() -> bool {
    true
}

fn get() -> String {
    "GET".to_string()
}

fn is_true(b: &bool) -> bool {
    *b
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../../../examples/online-store.mitm.json");

    #[test]
    fn example_file_parses_and_round_trips() {
        let file = WorkspaceFile::parse(EXAMPLE).unwrap();
        assert_eq!(file.workspace.name, "Online Store");
        let again = WorkspaceFile::parse(&serde_json::to_string_pretty(&file).unwrap()).unwrap();
        assert_eq!(again, file);
    }

    #[test]
    fn rejects_unknown_fields_wrong_format_newer_versions_and_bad_refs() {
        let base = r#"{"format":"mitm-workspace","version":1,"workspace":{"name":"Shop"}"#;
        assert!(WorkspaceFile::parse(&format!("{base}}}")).is_ok());
        assert!(WorkspaceFile::parse(&format!("{base},\"envs\":[]}}")).unwrap_err().contains("envs"));
        assert!(WorkspaceFile::parse(r#"{"format":"other","version":1,"workspace":{"name":"x"}}"#).is_err());
        assert!(WorkspaceFile::parse(r#"{"format":"mitm-workspace","version":2,"workspace":{"name":"x"}}"#)
            .unwrap_err()
            .contains("version 2"));
        let dangling = format!(
            "{base},\"collections\":[{{\"name\":\"A\",\"requests\":[{{\"name\":\"R\",\"url\":\"/\",\"before\":[\"nope\"]}}]}}]}}"
        );
        assert!(WorkspaceFile::parse(&dangling).unwrap_err().contains("nope"));
    }

    fn variable(json: &str) -> EnvironmentVariable {
        serde_json::from_str::<VariableEntry>(json).unwrap().to_variable()
    }

    fn names(var: &EnvironmentVariable) -> Vec<&str> {
        var.variants.iter().map(|v| v.name.as_str()).collect()
    }

    #[test]
    fn value_becomes_default_and_auto_starts_empty() {
        let var = variable(r#"{"key":"baseUrl","value":"http://localhost:3000"}"#);
        assert_eq!(names(&var), ["(auto)", "default"]);
        assert_eq!((var.active_index, var.current_value()), (1, "http://localhost:3000"));
        assert_eq!(var.variants[0].value, "");

        let empty = variable(r#"{"key":"accessToken"}"#);
        assert_eq!((names(&empty), empty.active_index), (vec!["(auto)"], 0));
    }

    #[test]
    fn variants_keep_their_names_and_the_first_one_is_active() {
        let var = variable(r#"{"key":"email","value":"a@x","variants":[{"name":"default","value":"b@x"},{"name":"default1","value":"c@x"}]}"#);
        assert_eq!(names(&var), ["(auto)", "default2", "default", "default1"]);
        assert_eq!(var.current_value(), "a@x");

        let chosen = variable(r#"{"key":"email","variants":[{"name":"new","value":"n@x"},{"name":"old","value":"o@x"}],"activeVariant":"old"}"#);
        assert_eq!(chosen.current_value(), "o@x");

        let auto = variable(r#"{"key":"token","variants":[{"name":"(auto)","value":"t1"},{"name":"admin","value":"t2"}]}"#);
        assert_eq!(names(&auto), ["(auto)", "admin"]);
        assert_eq!((auto.variants[0].value.as_str(), auto.current_value()), ("t1", "t2"));
    }

    #[test]
    fn export_then_import_keeps_variants_and_the_active_one() {
        for json in [
            r#"{"key":"a","value":"1"}"#,
            r#"{"key":"a","value":"1","variants":[{"name":"b","value":"2"}],"activeVariant":"b"}"#,
            r#"{"key":"a","variants":[{"name":"(auto)","value":"x"},{"name":"b","value":"2"}],"activeVariant":"(auto)"}"#,
            r#"{"key":"a","enabled":false}"#,
        ] {
            let var = variable(json);
            let again = VariableEntry::from_variable(&var, ExportOptions { include_secrets: true, include_auto: true }).to_variable();
            assert_eq!(names(&again), names(&var), "{json}");
            assert_eq!(again.active_index, var.active_index, "{json}");
            assert_eq!(again.current_value(), var.current_value(), "{json}");
            assert_eq!(again.enabled, var.enabled, "{json}");
        }
    }

    #[test]
    fn secrets_are_blank_unless_included() {
        let var = variable(r#"{"key":"password","value":"hunter2","secret":true}"#);
        let hidden = VariableEntry::from_variable(&var, ExportOptions::default());
        assert!(hidden.secret && hidden.variants.iter().all(|v| v.value.is_empty()));
        assert!(VariableEntry::from_variable(&var, ExportOptions { include_secrets: true, include_auto: true }).variants.iter().any(|v| v.value == "hunter2"));
    }

    #[test]
    fn auto_values_are_left_out_unless_included() {
        let var = variable(r#"{"key":"token","variants":[{"name":"(auto)","value":"t1"},{"name":"admin","value":"t2"}],"activeVariant":"(auto)"}"#);
        let without = VariableEntry::from_variable(&var, ExportOptions::default());
        assert_eq!(without.variants.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(), ["admin"]);
        assert_eq!(without.active_variant.as_deref(), Some("(auto)"));
        let reimported = without.to_variable();
        assert_eq!((reimported.variants[0].value.as_str(), reimported.active_index), ("", 0));

        let with = VariableEntry::from_variable(&var, ExportOptions { include_auto: true, ..ExportOptions::default() });
        assert_eq!(with.variants[0].value, "t1");
    }

    #[test]
    fn unknown_active_variant_is_rejected() {
        let json = r#"{"format":"mitm-workspace","version":1,"workspace":{"name":"Shop"},
            "environments":[{"name":"Local","variables":[{"key":"a","value":"1","activeVariant":"nope"}]}]}"#;
        assert!(WorkspaceFile::parse(json).unwrap_err().contains("nope"));
    }

    #[test]
    fn refs_are_slugs_numbered_when_taken() {
        let mut taken = std::collections::HashSet::new();
        assert_eq!(make_ref("Log in!", &mut taken), "log-in");
        assert_eq!(make_ref("Log in", &mut taken), "log-in-2");
        assert_eq!(make_ref("✓", &mut taken), "request");
    }
}
