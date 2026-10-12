mod api;
pub mod execute;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use regex::{Captures, Regex};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::repeater::{HeaderItem, ParamItem};

pub const MAX_REQUESTS: usize = 100_000;

/// Matches a `{{name}}` fuzz placeholder; the capture group is the variable name.
pub fn marker_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{\{\s*([A-Za-z0-9_.\-]+)\s*\}\}").unwrap())
}

/// A request with `{{name}}` placeholders in its field values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzTemplate {
    pub method: String,
    pub url: String,
    pub headers: Vec<HeaderItem>,
    pub params: Vec<ParamItem>,
    #[serde(default)]
    pub body_type: String,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadSet {
    /// "list" | "numbers"
    pub kind: String,
    #[serde(default)]
    pub list: Vec<String>,
    #[serde(default)]
    pub from: f64,
    #[serde(default)]
    pub to: f64,
    #[serde(default = "one")]
    pub step: f64,
    #[serde(default)]
    pub pad: u32,
    #[serde(default)]
    pub url_encode: bool,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
}

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchRule {
    pub name: String,
    /// "contains" | "regex"
    pub kind: String,
    pub pattern: String,
}

/// A named placeholder (`{{name}}`) paired with the payloads it draws from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzVariable {
    pub name: String,
    pub set: PayloadSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzConfig {
    pub template: FuzzTemplate,
    /// "sniper" | "pitchfork" | "clusterbomb"
    pub attack_type: String,
    #[serde(default)]
    pub variables: Vec<FuzzVariable>,
    #[serde(default)]
    pub match_rules: Vec<MatchRule>,
    #[serde(default = "default_concurrency")]
    pub concurrency: u32,
    #[serde(default)]
    pub delay_ms: u64,
}

fn default_concurrency() -> u32 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzResult {
    pub idx: u32,
    pub payloads: Vec<String>,
    pub method: String,
    pub url: String,
    pub request_headers: Vec<HeaderItem>,
    pub request_body: Option<String>,
    pub status_code: u16,
    pub response_headers: Vec<HeaderItem>,
    pub response_body: Option<String>,
    pub response_size: u64,
    pub duration_ms: u64,
    pub error: Option<String>,
    /// One value per match rule, in rule order.
    pub matches: Vec<String>,
}

/// What a results table needs from a result, without its bodies and headers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzRow {
    pub idx: u32,
    pub payloads: Vec<String>,
    pub status_code: u16,
    pub response_size: u64,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub matches: Vec<String>,
}

impl FuzzResult {
    pub fn row(&self) -> FuzzRow {
        FuzzRow {
            idx: self.idx,
            payloads: self.payloads.clone(),
            status_code: self.status_code,
            response_size: self.response_size,
            duration_ms: self.duration_ms,
            error: self.error.clone(),
            matches: self.matches.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FuzzStart {
    pub run_id: i64,
    /// Variable names in the order each row's payloads are listed.
    pub names: Vec<String>,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzRunMeta {
    pub id: i64,
    pub name: String,
    pub attack_type: String,
    pub total: u32,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedFuzzRun {
    pub id: i64,
    pub name: String,
    pub attack_type: String,
    pub total: u32,
    pub config: FuzzConfig,
    pub rows: Vec<FuzzRow>,
}

/// Replaces every `{{name}}` in `s` with its value from `values`; unknown names are left as-is.
fn replace_markers(s: &str, values: &HashMap<String, String>) -> String {
    marker_re()
        .replace_all(s, |caps: &Captures| {
            values
                .get(&caps[1])
                .cloned()
                .unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned()
}

/// Form bodies are stored as JSON, so payloads are substituted inside its strings to keep quotes and
/// backslashes in a payload from breaking the JSON.
fn replace_body_markers(body_type: &str, body: &str, values: &HashMap<String, String>) -> String {
    let is_form = matches!(body_type, "form" | "form-data" | "multipart" | "urlencoded" | "x-www-form-urlencoded");
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(mut json) if is_form => {
            replace_json_markers(&mut json, values);
            json.to_string()
        }
        _ => replace_markers(body, values),
    }
}

fn replace_json_markers(value: &mut serde_json::Value, values: &HashMap<String, String>) {
    match value {
        serde_json::Value::String(s) => *s = replace_markers(s, values),
        serde_json::Value::Array(items) => items.iter_mut().for_each(|v| replace_json_markers(v, values)),
        serde_json::Value::Object(map) => map.values_mut().for_each(|v| replace_json_markers(v, values)),
        _ => {}
    }
}

/// Fields that may hold placeholders, in the order variables are discovered.
fn marked_fields(t: &FuzzTemplate) -> Vec<String> {
    let mut fields = vec![t.url.clone()];
    for p in &t.params {
        if p.enabled {
            fields.push(p.value.clone());
        }
    }
    for h in &t.headers {
        if h.enabled {
            fields.push(h.value.clone());
        }
    }
    fields.push(t.body.clone().unwrap_or_default());
    fields
}

/// Unique variable names, in first-appearance order (url, params, headers, body).
pub fn parse_variables(t: &FuzzTemplate) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for value in marked_fields(t) {
        for caps in marker_re().captures_iter(&value) {
            let name = caps[1].to_string();
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

/// A template with every `{{name}}` replaced by the assigned value.
pub fn apply_payloads(t: &FuzzTemplate, values: &HashMap<String, String>) -> FuzzTemplate {
    let params = t
        .params
        .iter()
        .map(|p| {
            if p.enabled {
                ParamItem {
                    value: replace_markers(&p.value, values),
                    ..p.clone()
                }
            } else {
                p.clone()
            }
        })
        .collect();
    let headers = t
        .headers
        .iter()
        .map(|h| {
            if h.enabled {
                HeaderItem {
                    value: replace_markers(&h.value, values),
                    ..h.clone()
                }
            } else {
                h.clone()
            }
        })
        .collect();
    FuzzTemplate {
        method: t.method.clone(),
        url: replace_markers(&t.url, values),
        headers,
        params,
        body_type: t.body_type.clone(),
        body: t.body.as_ref().map(|b| replace_body_markers(&t.body_type, b, values)),
    }
}

fn expand_payload_set(set: &PayloadSet) -> Vec<String> {
    let raw: Vec<String> = if set.kind == "numbers" {
        let mut out = Vec::new();
        let step = if set.step == 0.0 { 1.0 } else { set.step };
        let mut v = set.from;
        let ascending = step > 0.0;
        let mut guard = 0;
        while (ascending && v <= set.to) || (!ascending && v >= set.to) {
            let s = if v.fract() == 0.0 {
                format!("{}", v as i64)
            } else {
                format!("{}", v)
            };
            let s = if set.pad > 0 {
                format!("{:0>width$}", s, width = set.pad as usize)
            } else {
                s
            };
            out.push(s);
            v += step;
            guard += 1;
            if guard > MAX_REQUESTS {
                break;
            }
        }
        out
    } else {
        set.list.iter().map(|s| s.to_string()).collect()
    };

    raw.into_iter()
        .map(|v| {
            let v = if set.url_encode {
                urlencoding::encode(&v).into_owned()
            } else {
                v
            };
            format!("{}{}{}", set.prefix, v, set.suffix)
        })
        .collect()
}

/// The expanded payload list for each variable, in `names` order.
fn variable_lists(config: &FuzzConfig, names: &[String]) -> Vec<Vec<String>> {
    names
        .iter()
        .map(|name| {
            config
                .variables
                .iter()
                .find(|v| &v.name == name)
                .map(|v| expand_payload_set(&v.set))
                .unwrap_or_default()
        })
        .collect()
}

fn too_many(total: usize) -> String {
    format!(
        "That is {} requests; the limit is {}. Trim the payload lists.",
        total, MAX_REQUESTS
    )
}

/// The ordered variable names and the per-request value rows (one value per name).
pub fn generate_combos(config: &FuzzConfig) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let names = parse_variables(&config.template);
    let n = names.len();
    if n == 0 {
        return Err("Add at least one {{variable}} to the request.".to_string());
    }

    let lists = variable_lists(config, &names);
    if let Some(i) = lists.iter().position(|l| l.is_empty()) {
        return Err(format!("Variable {{{{{}}}}} has no payloads.", names[i]));
    }

    let rows: Vec<Vec<String>> = match config.attack_type.as_str() {
        // Each variable fuzzed in turn; the others hold their first payload.
        "sniper" => {
            let baseline: Vec<String> = lists.iter().map(|l| l[0].clone()).collect();
            let total: usize = lists.iter().map(|l| l.len()).sum();
            if total > MAX_REQUESTS {
                return Err(too_many(total));
            }
            let mut out = Vec::with_capacity(total);
            for (pos, list) in lists.iter().enumerate() {
                for v in list {
                    let mut row = baseline.clone();
                    row[pos] = v.clone();
                    out.push(row);
                }
            }
            out
        }
        // All variables advance together; stops at the shortest list.
        "pitchfork" => {
            let len = lists.iter().map(|l| l.len()).min().unwrap_or(0);
            (0..len)
                .map(|i| lists.iter().map(|l| l[i].clone()).collect())
                .collect()
        }
        // Every combination of the variables' payloads.
        "clusterbomb" => {
            let total: usize = lists.iter().map(|l| l.len()).product();
            if total > MAX_REQUESTS {
                return Err(too_many(total));
            }
            let mut out: Vec<Vec<String>> = vec![vec![]];
            for list in &lists {
                let mut next = Vec::with_capacity(out.len() * list.len());
                for prefix in &out {
                    for v in list {
                        let mut row = prefix.clone();
                        row.push(v.clone());
                        next.push(row);
                    }
                }
                out = next;
            }
            out
        }
        other => return Err(format!("Unknown attack type '{}'.", other)),
    };

    if rows.len() > MAX_REQUESTS {
        return Err(too_many(rows.len()));
    }
    Ok((names, rows))
}

pub fn evaluate_matches(rules: &[MatchRule], body: &str) -> Vec<String> {
    rules
        .iter()
        .map(|rule| {
            if rule.pattern.is_empty() {
                return if rule.kind == "regex" { String::new() } else { "0".to_string() };
            }
            if rule.kind == "regex" {
                match regex::Regex::new(&rule.pattern) {
                    Ok(re) => re
                        .captures(body)
                        .and_then(|c| c.get(1).or_else(|| c.get(0)))
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default(),
                    Err(_) => "bad regex".to_string(),
                }
            } else {
                body.matches(&rule.pattern).count().to_string()
            }
        })
        .collect()
}

/// Inserts the run as unsaved, so its results can be written while it runs.
pub fn begin_run_db(db_path: &PathBuf, id: i64, config: &FuzzConfig, total: usize) -> Result<(), String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let config_json = serde_json::to_string(config).map_err(|e| e.to_string())?;
    let now = chrono::Local::now().timestamp_millis();
    conn.execute(
        "INSERT OR REPLACE INTO fuzz_runs (id, name, attack_type, config_json, total, created_at_ms, saved)
         VALUES (?, '', ?, ?, ?, ?, 0)",
        params![id, config.attack_type, config_json, total as i64, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn insert_results(conn: &mut Connection, run_id: i64, results: &[FuzzResult]) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    {
        let mut stmt = tx
            .prepare_cached(
                "INSERT INTO fuzz_results
                    (run_id, idx, payloads_json, method, url, request_headers_json, request_body, status_code,
                     response_headers_json, response_body, response_size, duration_ms, error, matches_json)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .map_err(|e| e.to_string())?;
        for r in results {
            stmt.execute(params![
                run_id,
                r.idx,
                serde_json::to_string(&r.payloads).unwrap_or_else(|_| "[]".into()),
                r.method,
                r.url,
                serde_json::to_string(&r.request_headers).unwrap_or_else(|_| "[]".into()),
                r.request_body,
                r.status_code,
                serde_json::to_string(&r.response_headers).unwrap_or_else(|_| "[]".into()),
                r.response_body,
                r.response_size as i64,
                r.duration_ms as i64,
                r.error,
                serde_json::to_string(&r.matches).unwrap_or_else(|_| "[]".into()),
            ])
            .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn discard_unsaved_runs_db(db_path: &PathBuf) -> Result<(), String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "DELETE FROM fuzz_results WHERE run_id IN (SELECT id FROM fuzz_runs WHERE saved = 0);
         DELETE FROM fuzz_runs WHERE saved = 0;",
    )
    .map_err(|e| e.to_string())
}

pub fn list_runs_db(db_path: &PathBuf) -> Result<Vec<FuzzRunMeta>, String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, attack_type, total, created_at_ms FROM fuzz_runs WHERE saved = 1 ORDER BY created_at_ms DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(FuzzRunMeta {
                id: row.get(0)?,
                name: row.get(1)?,
                attack_type: row.get(2)?,
                total: row.get::<_, i64>(3)? as u32,
                created_at_ms: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

pub fn get_run_config_db(db_path: &PathBuf, id: i64) -> Result<FuzzConfig, String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let json: String = conn
        .query_row("SELECT config_json FROM fuzz_runs WHERE id = ?", params![id], |row| row.get(0))
        .map_err(|e| format!("Run not found: {}", e))?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

pub fn get_run_rows_db(db_path: &PathBuf, id: i64) -> Result<Vec<FuzzRow>, String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT idx, payloads_json, status_code, response_size, duration_ms, error, matches_json
             FROM fuzz_results WHERE run_id = ? ORDER BY idx ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![id], |row| {
            Ok(FuzzRow {
                idx: row.get::<_, i64>(0)? as u32,
                payloads: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                status_code: row.get::<_, i64>(2)? as u16,
                response_size: row.get::<_, i64>(3)? as u64,
                duration_ms: row.get::<_, i64>(4)? as u64,
                error: row.get(5)?,
                matches: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

pub fn get_result_db(db_path: &PathBuf, run_id: i64, idx: u32) -> Result<Option<FuzzResult>, String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let result = conn.query_row(
        "SELECT idx, payloads_json, method, url, request_headers_json, request_body, status_code,
                response_headers_json, response_body, response_size, duration_ms, error, matches_json
         FROM fuzz_results WHERE run_id = ? AND idx = ?",
        params![run_id, idx],
        |row| {
            Ok(FuzzResult {
                idx: row.get::<_, i64>(0)? as u32,
                payloads: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                method: row.get(2)?,
                url: row.get(3)?,
                request_headers: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                request_body: row.get(5)?,
                status_code: row.get::<_, i64>(6)? as u16,
                response_headers: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                response_body: row.get(8)?,
                response_size: row.get::<_, i64>(9)? as u64,
                duration_ms: row.get::<_, i64>(10)? as u64,
                error: row.get(11)?,
                matches: serde_json::from_str(&row.get::<_, String>(12)?).unwrap_or_default(),
            })
        },
    );
    match result {
        Ok(result) => Ok(Some(result)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Indexes of the results whose response body contains `query`, ignoring ASCII case.
pub fn search_responses_db(db_path: &PathBuf, run_id: i64, query: &str) -> Result<Vec<u32>, String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT idx FROM fuzz_results WHERE run_id = ? AND instr(lower(response_body), lower(?)) > 0 ORDER BY idx")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![run_id, query], |row| row.get::<_, i64>(0).map(|i| i as u32))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

pub fn save_run_db(db_path: &PathBuf, id: i64, name: &str) -> Result<(), String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let updated = conn
        .execute("UPDATE fuzz_runs SET name = ?, saved = 1 WHERE id = ?", params![name, id])
        .map_err(|e| e.to_string())?;
    if updated == 0 {
        return Err(format!("Run {} not found", id));
    }
    Ok(())
}

pub fn is_run_saved_db(db_path: &PathBuf, id: i64) -> Result<bool, String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let saved: i64 = conn
        .query_row("SELECT saved FROM fuzz_runs WHERE id = ?", params![id], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    Ok(saved == 1)
}

pub fn load_run_db(db_path: &PathBuf, id: i64) -> Result<LoadedFuzzRun, String> {
    let conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let (name, attack_type, total, config_json): (String, String, i64, String) = conn
        .query_row(
            "SELECT name, attack_type, total, config_json FROM fuzz_runs WHERE id = ?",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|e| format!("Run not found: {}", e))?;
    let config: FuzzConfig = serde_json::from_str(&config_json).map_err(|e| e.to_string())?;
    let rows = get_run_rows_db(db_path, id)?;
    Ok(LoadedFuzzRun {
        id,
        name,
        attack_type,
        total: total as u32,
        config,
        rows,
    })
}

pub fn delete_run_db(db_path: &PathBuf, id: i64) -> Result<(), String> {
    let mut conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM fuzz_results WHERE run_id = ?", params![id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM fuzz_runs WHERE id = ?", params![id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn clear_fuzz_runs(db_path: &PathBuf) -> Result<(), String> {
    let mut conn = crate::db::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM fuzz_results", []).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM fuzz_runs", []).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpl(url: &str, body: &str) -> FuzzTemplate {
        FuzzTemplate {
            method: "GET".into(),
            url: url.into(),
            headers: vec![],
            params: vec![],
            body_type: "raw".into(),
            body: Some(body.into()),
        }
    }

    #[test]
    fn form_body_payloads_stay_valid_json() {
        let mut t = tmpl("http://x", r#"[{"key":"q","value":"{{p}}","enabled":true}]"#);
        t.body_type = "urlencoded".into();
        let values = HashMap::from([("p".to_string(), r#"a"b\c"#.to_string())]);
        let body = apply_payloads(&t, &values).body.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed[0]["value"], r#"a"b\c"#);
    }

    fn list(name: &str, values: &[&str]) -> FuzzVariable {
        FuzzVariable {
            name: name.into(),
            set: PayloadSet {
                kind: "list".into(),
                list: values.iter().map(|s| s.to_string()).collect(),
                from: 0.0,
                to: 0.0,
                step: 1.0,
                pad: 0,
                url_encode: false,
                prefix: String::new(),
                suffix: String::new(),
            },
        }
    }

    fn cfg(template: FuzzTemplate, attack: &str, vars: Vec<FuzzVariable>) -> FuzzConfig {
        FuzzConfig {
            template,
            attack_type: attack.into(),
            variables: vars,
            match_rules: vec![],
            concurrency: 10,
            delay_ms: 0,
        }
    }

    #[test]
    fn dedupes_repeated_variable() {
        let t = tmpl("/u/{{id}}/{{id}}", "role={{role}}");
        assert_eq!(parse_variables(&t), vec!["id", "role"]);
    }

    #[test]
    fn repeated_variable_shares_value() {
        let t = tmpl("/u/{{id}}/{{id}}", "");
        let mut map = HashMap::new();
        map.insert("id".to_string(), "7".to_string());
        assert_eq!(apply_payloads(&t, &map).url, "/u/7/7");
    }

    #[test]
    fn sniper_holds_others_at_first() {
        let t = tmpl("/{{a}}/{{b}}", "");
        let (names, rows) = generate_combos(&cfg(t, "sniper", vec![list("a", &["1", "2"]), list("b", &["9"])])).unwrap();
        assert_eq!(names, vec!["a", "b"]);
        // a: [1,2] with b held at 9, then b: [9] with a held at 1
        assert_eq!(rows, vec![vec!["1", "9"], vec!["2", "9"], vec!["1", "9"]]);
    }

    #[test]
    fn pitchfork_lockstep_min_length() {
        let t = tmpl("/{{a}}/{{b}}", "");
        let (_, rows) = generate_combos(&cfg(t, "pitchfork", vec![list("a", &["1", "2", "3"]), list("b", &["x", "y"])])).unwrap();
        assert_eq!(rows, vec![vec!["1", "x"], vec!["2", "y"]]);
    }

    #[test]
    fn clusterbomb_product() {
        let t = tmpl("/{{a}}/{{b}}", "");
        let (_, rows) = generate_combos(&cfg(t, "clusterbomb", vec![list("a", &["1", "2"]), list("b", &["x", "y"])])).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0], vec!["1", "x"]);
        assert_eq!(rows[3], vec!["2", "y"]);
    }

    #[test]
    fn errors_without_variables() {
        assert!(generate_combos(&cfg(tmpl("/static", ""), "sniper", vec![])).is_err());
    }

    #[test]
    fn errors_on_empty_list() {
        let t = tmpl("/{{a}}", "");
        assert!(generate_combos(&cfg(t, "sniper", vec![list("a", &[])])).is_err());
    }

    #[test]
    fn evaluate_matches_handles_empty_pattern_and_kinds() {
        let rules = vec![
            MatchRule { name: "c".into(), kind: "contains".into(), pattern: "foo".into() },
            MatchRule { name: "empty_c".into(), kind: "contains".into(), pattern: "".into() },
            MatchRule { name: "r".into(), kind: "regex".into(), pattern: r"id=(\d+)".into() },
            MatchRule { name: "empty_r".into(), kind: "regex".into(), pattern: "".into() },
            MatchRule { name: "bad_r".into(), kind: "regex".into(), pattern: r"(\d+".into() },
        ];
        let res = evaluate_matches(&rules, "foo foo bar id=42");
        assert_eq!(res, vec!["2", "0", "42", "", "bad regex"]);
    }

    #[test]
    fn run_persistence_lifecycle() {
        let dir = std::env::temp_dir().join(format!("mitm-fuzz-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test.db");
        crate::db::init_database(&db_path, None).unwrap();

        let config = cfg(tmpl("/{{a}}", ""), "sniper", vec![list("a", &["1", "2"])]);
        let run_id = 12345;
        begin_run_db(&db_path, run_id, &config, 2).unwrap();

        assert!(!is_run_saved_db(&db_path, run_id).unwrap());
        let runs = list_runs_db(&db_path).unwrap();
        assert!(runs.is_empty());

        let res = FuzzResult {
            idx: 0,
            payloads: vec!["1".into()],
            method: "GET".into(),
            url: "http://example.com/1".into(),
            request_headers: vec![],
            request_body: None,
            status_code: 200,
            response_headers: vec![],
            response_body: Some("hello".into()),
            response_size: 5,
            duration_ms: 10,
            error: None,
            matches: vec![],
        };
        let mut conn = crate::db::open(&db_path).unwrap();
        insert_results(&mut conn, run_id, &[res]).unwrap();
        drop(conn);

        save_run_db(&db_path, run_id, "My Saved Run").unwrap();
        assert!(is_run_saved_db(&db_path, run_id).unwrap());

        let runs = list_runs_db(&db_path).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, run_id);
        assert_eq!(runs[0].name, "My Saved Run");
        assert_eq!(runs[0].attack_type, "sniper");
        assert_eq!(runs[0].total, 2);

        let loaded = load_run_db(&db_path, run_id).unwrap();
        assert_eq!(loaded.id, run_id);
        assert_eq!(loaded.name, "My Saved Run");
        assert_eq!(loaded.rows.len(), 1);
        assert_eq!(loaded.rows[0].idx, 0);
        assert_eq!(loaded.rows[0].status_code, 200);

        delete_run_db(&db_path, run_id).unwrap();
        assert!(list_runs_db(&db_path).unwrap().is_empty());
        assert!(load_run_db(&db_path, run_id).is_err());
        assert!(get_result_db(&db_path, run_id, 0).unwrap().is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
