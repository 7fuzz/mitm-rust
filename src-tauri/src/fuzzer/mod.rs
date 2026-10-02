pub mod execute;

use std::path::PathBuf;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::repeater::{HeaderItem, ParamItem};

pub const MARKER: char = '§';
pub const MAX_REQUESTS: usize = 100_000;

/// A request with §payload§ markers in its field values.
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzConfig {
    pub template: FuzzTemplate,
    /// "sniper" | "clusterbomb"
    pub attack_type: String,
    pub payload_sets: Vec<PayloadSet>,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzPosition {
    pub index: usize,
    /// "url" | "param:Key" | "header:Key" | "body"
    pub field: String,
    pub base: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchResult {
    pub name: String,
    pub value: String,
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
    pub matches: Vec<MatchResult>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzRunMeta {
    pub id: String,
    pub name: String,
    pub attack_type: String,
    pub total: u32,
    pub created_at_ms: i64,
}

/// Replaces each §marker§ in `s` with the next value from `values`, sharing `cursor` across fields.
fn replace_markers(s: &str, values: &[String], cursor: &mut usize) -> String {
    let parts: Vec<&str> = s.split(MARKER).collect();
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i % 2 == 0 {
            out.push_str(part);
        } else {
            if let Some(v) = values.get(*cursor) {
                out.push_str(v);
            } else {
                out.push_str(part);
            }
            *cursor += 1;
        }
    }
    out
}

/// Fields that may hold markers, in the order positions are numbered.
fn marked_fields(t: &FuzzTemplate) -> Vec<(String, String)> {
    let mut fields = vec![("url".to_string(), t.url.clone())];
    for p in &t.params {
        if p.enabled {
            fields.push((format!("param:{}", p.key), p.value.clone()));
        }
    }
    for h in &t.headers {
        if h.enabled {
            fields.push((format!("header:{}", h.key), h.value.clone()));
        }
    }
    fields.push(("body".to_string(), t.body.clone().unwrap_or_default()));
    fields
}

pub fn parse_positions(t: &FuzzTemplate) -> Vec<FuzzPosition> {
    let mut positions = Vec::new();
    for (field, value) in marked_fields(t) {
        let parts: Vec<&str> = value.split(MARKER).collect();
        for (i, part) in parts.iter().enumerate() {
            if i % 2 == 1 {
                positions.push(FuzzPosition {
                    index: positions.len(),
                    field: field.clone(),
                    base: part.to_string(),
                });
            }
        }
    }
    positions
}

/// Base value of each position, in order.
fn base_values(t: &FuzzTemplate) -> Vec<String> {
    parse_positions(t).into_iter().map(|p| p.base).collect()
}

/// A template with its markers replaced by one payload per position.
pub fn apply_payloads(t: &FuzzTemplate, payloads: &[String]) -> FuzzTemplate {
    let mut cursor = 0usize;
    let url = replace_markers(&t.url, payloads, &mut cursor);
    let params = t
        .params
        .iter()
        .map(|p| {
            if p.enabled {
                ParamItem {
                    value: replace_markers(&p.value, payloads, &mut cursor),
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
                    value: replace_markers(&h.value, payloads, &mut cursor),
                    ..h.clone()
                }
            } else {
                h.clone()
            }
        })
        .collect();
    let body = t
        .body
        .as_ref()
        .map(|b| replace_markers(b, payloads, &mut cursor));
    FuzzTemplate {
        method: t.method.clone(),
        url,
        headers,
        params,
        body_type: t.body_type.clone(),
        body,
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

/// Every payload tuple to send, one inner vec of length `positions`.
pub fn generate_combos(config: &FuzzConfig) -> Result<Vec<Vec<String>>, String> {
    let bases = base_values(&config.template);
    let n = bases.len();
    if n == 0 {
        return Err("Add at least one payload position (§marker§) to the request.".to_string());
    }
    if config.payload_sets.is_empty() {
        return Err("Add at least one payload set.".to_string());
    }

    let combos: Vec<Vec<String>> = match config.attack_type.as_str() {
        "sniper" => {
            let values = expand_payload_set(&config.payload_sets[0]);
            if values.is_empty() {
                return Err("The payload set is empty.".to_string());
            }
            let mut out = Vec::with_capacity(n * values.len());
            for pos in 0..n {
                for v in &values {
                    let mut combo = bases.clone();
                    combo[pos] = v.clone();
                    out.push(combo);
                }
            }
            out
        }
        "clusterbomb" => {
            if config.payload_sets.len() < n {
                return Err(format!(
                    "Cluster bomb needs one payload set per position ({} needed, {} given).",
                    n,
                    config.payload_sets.len()
                ));
            }
            let sets: Vec<Vec<String>> = (0..n)
                .map(|i| expand_payload_set(&config.payload_sets[i]))
                .collect();
            if let Some(empty) = sets.iter().position(|s| s.is_empty()) {
                return Err(format!("Payload set {} is empty.", empty + 1));
            }
            let total: usize = sets.iter().map(|s| s.len()).product();
            if total > MAX_REQUESTS {
                return Err(format!(
                    "That is {} requests; the limit is {}. Trim the payload sets.",
                    total, MAX_REQUESTS
                ));
            }
            let mut out: Vec<Vec<String>> = vec![vec![]];
            for set in &sets {
                let mut next = Vec::with_capacity(out.len() * set.len());
                for prefix in &out {
                    for v in set {
                        let mut combo = prefix.clone();
                        combo.push(v.clone());
                        next.push(combo);
                    }
                }
                out = next;
            }
            out
        }
        other => return Err(format!("Unknown attack type '{}'.", other)),
    };

    if combos.len() > MAX_REQUESTS {
        return Err(format!(
            "That is {} requests; the limit is {}. Trim the payload sets.",
            combos.len(),
            MAX_REQUESTS
        ));
    }
    Ok(combos)
}

pub fn evaluate_matches(rules: &[MatchRule], body: &str) -> Vec<MatchResult> {
    rules
        .iter()
        .map(|rule| {
            let value = if rule.kind == "regex" {
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
            };
            MatchResult {
                name: rule.name.clone(),
                value,
            }
        })
        .collect()
}

// ---- persistence ----

pub fn save_run_db(
    db_path: &PathBuf,
    id: &str,
    name: &str,
    config: &FuzzConfig,
    results: &[FuzzResult],
) -> Result<(), String> {
    let mut conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let config_json = serde_json::to_string(config).map_err(|e| e.to_string())?;
    let now = chrono::Local::now().timestamp_millis();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT OR REPLACE INTO fuzz_runs (id, name, attack_type, config_json, total, created_at_ms)
         VALUES (?, ?, ?, ?, ?, ?)",
        params![id, name, config.attack_type, config_json, results.len() as i64, now],
    )
    .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM fuzz_results WHERE run_id = ?", params![id])
        .map_err(|e| e.to_string())?;
    for r in results {
        insert_result(&tx, id, r)?;
    }
    tx.commit().map_err(|e| e.to_string())
}

fn insert_result(conn: &Connection, run_id: &str, r: &FuzzResult) -> Result<(), String> {
    conn.execute(
        "INSERT INTO fuzz_results
            (run_id, idx, payloads_json, method, url, request_headers_json, request_body,
             status_code, response_headers_json, response_body, response_size, duration_ms, error)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
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
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_runs_db(db_path: &PathBuf) -> Result<Vec<FuzzRunMeta>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, attack_type, total, created_at_ms FROM fuzz_runs ORDER BY created_at_ms DESC")
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

pub fn get_run_config_db(db_path: &PathBuf, id: &str) -> Result<FuzzConfig, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let json: String = conn
        .query_row("SELECT config_json FROM fuzz_runs WHERE id = ?", params![id], |row| row.get(0))
        .map_err(|e| format!("Run not found: {}", e))?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

pub fn get_run_results_db(db_path: &PathBuf, id: &str) -> Result<Vec<FuzzResult>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT idx, payloads_json, method, url, request_headers_json, request_body,
                    status_code, response_headers_json, response_body, response_size, duration_ms, error
             FROM fuzz_results WHERE run_id = ? ORDER BY idx ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![id], |row| {
            let payloads_json: String = row.get(1)?;
            let req_h: String = row.get(4)?;
            let res_h: String = row.get(7)?;
            Ok(FuzzResult {
                idx: row.get::<_, i64>(0)? as u32,
                payloads: serde_json::from_str(&payloads_json).unwrap_or_default(),
                method: row.get(2)?,
                url: row.get(3)?,
                request_headers: serde_json::from_str(&req_h).unwrap_or_default(),
                request_body: row.get(5)?,
                status_code: row.get::<_, i64>(6)? as u16,
                response_headers: serde_json::from_str(&res_h).unwrap_or_default(),
                response_body: row.get(8)?,
                response_size: row.get::<_, i64>(9)? as u64,
                duration_ms: row.get::<_, i64>(10)? as u64,
                error: row.get(11)?,
                matches: Vec::new(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

pub fn delete_run_db(db_path: &PathBuf, id: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM fuzz_results WHERE run_id = ?", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM fuzz_runs WHERE id = ?", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
