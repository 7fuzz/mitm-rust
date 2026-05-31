use std::collections::HashMap;
use tauri::AppHandle;
use time::{Duration, OffsetDateTime};
use time::macros::format_description;
use crate::{db, proxy};
use rusqlite::OptionalExtension;
use hyper::Request;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use http_body_util::{Full, BodyExt};
use bytes::Bytes;
use flate2::read::GzDecoder;
use flate2::read::ZlibDecoder;
use brotli::Decompressor;
use std::io::Read;
use zstd;

fn build_variable_map(conn: &rusqlite::Connection, active_env_id: Option<&str>) -> Result<HashMap<String, String>, String> {
    let mut vars = HashMap::new();
    let env_id = match active_env_id {
        Some(id) => id,
        None => return Ok(vars),
    };

    let mut stmt = conn.prepare("SELECT id, name, active_index FROM variables WHERE environment_id = ?").map_err(|e| e.to_string())?;
    let variable_rows = stmt.query_map([env_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i32>(2)?))
    }).map_err(|e| e.to_string())?;

    for var_res in variable_rows {
        let (var_id, name, active_index) = var_res.map_err(|e| e.to_string())?;
        let selected_value = get_selected_variable_value(conn, &var_id, active_index)?;
        vars.insert(name, selected_value);
    }

    Ok(vars)
}

fn get_selected_variable_value(conn: &rusqlite::Connection, variable_id: &str, active_index: i32) -> Result<String, String> {
    let mut stmt = conn.prepare("SELECT value FROM variable_values WHERE variable_id = ? ORDER BY rowid").map_err(|e| e.to_string())?;
    let values: Vec<String> = stmt.query_map([variable_id], |row| row.get(0)).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok()).collect();

    if values.is_empty() {
        return Ok(String::new());
    }

    let index = if active_index < 0 { 0 } else { active_index as usize };
    if let Some(value) = values.get(index) {
        Ok(value.clone())
    } else {
        Ok(values[0].clone())
    }
}

fn interpolate_variables(input: &str, vars: &HashMap<String, String>) -> String {
    let mut output = String::new();
    let mut remainder = input;

    while let Some(start) = remainder.find("{{") {
        output.push_str(&remainder[..start]);
        let after_start = &remainder[start + 2..];
        if let Some(end_rel) = after_start.find("}}") {
            let key = &after_start[..end_rel];
            let replacement = vars.get(key).cloned().unwrap_or_else(|| format!("{{{{{}}}}}", key));
            output.push_str(&replacement);
            remainder = &after_start[end_rel + 2..];
        } else {
            break;
        }
    }

    output.push_str(remainder);
    output
}

fn interpolate_dates(input: &str) -> String {
    let mut output = String::new();
    let mut remainder = input;

    while let Some(start) = remainder.find("[[") {
        output.push_str(&remainder[..start]);
        let after_start = &remainder[start + 2..];
        if let Some(end_rel) = after_start.find("]]") {
            let token = &after_start[..end_rel];
            let replacement = render_dynamic_date(token);
            output.push_str(&replacement);
            remainder = &after_start[end_rel + 2..];
        } else {
            break;
        }
    }

    output.push_str(remainder);
    output
}

fn render_dynamic_date(token: &str) -> String {
    let raw = token.split('/').next().unwrap_or(token).trim();
    if raw.is_empty() {
        return format!("[[{}]]", token);
    }

    let (base, offset) = parse_date_token(raw);
    let local_date = OffsetDateTime::now_utc().date();
    let base_date = match base {
        "today" => local_date,
        "yesterday" => local_date - Duration::days(1),
        "tomorrow" => local_date + Duration::days(1),
        _ => return format!("[[{}]]", token),
    };

    let final_date = if offset != 0 {
        base_date + Duration::days(offset as i64)
    } else {
        base_date
    };

    final_date.format(&format_description!("[year]-[month]-[day]")).unwrap_or_default()
}

fn parse_date_token(raw: &str) -> (&str, i32) {
    if let Some(pos) = raw.find('+') {
        let base = &raw[..pos];
        let offset = raw[pos + 1..].parse::<i32>().unwrap_or(0);
        (base, offset)
    } else if let Some(pos) = raw.rfind('-') {
        let base = &raw[..pos];
        let offset = raw[pos + 1..].parse::<i32>().unwrap_or(0);
        (base, -offset)
    } else {
        (raw, 0)
    }
}

fn decompress_body(body: &[u8], encoding: &str) -> Option<Vec<u8>> {
    let encodings: Vec<&str> = encoding.split(',').map(|s| s.trim()).collect();
    let mut current_body = body.to_vec();
    let mut decompressed = false;

    for enc in encodings.iter().rev() {
        let enc = enc.to_lowercase();
        match enc.as_str() {
            "gzip" | "x-gzip" => {
                let mut decoder = GzDecoder::new(&current_body[..]);
                let mut decoded = Vec::new();
                if decoder.read_to_end(&mut decoded).is_ok() {
                    current_body = decoded;
                    decompressed = true;
                } else {
                    return None;
                }
            }
            "deflate" => {
                let mut decoder = ZlibDecoder::new(&current_body[..]);
                let mut decoded = Vec::new();
                if decoder.read_to_end(&mut decoded).is_ok() {
                    current_body = decoded;
                    decompressed = true;
                } else {
                    return None;
                }
            }
            "br" => {
                let mut decoded = Vec::new();
                if Decompressor::new(&current_body[..], 4096).read_to_end(&mut decoded).is_ok() {
                    current_body = decoded;
                    decompressed = true;
                } else {
                    return None;
                }
            }
            "zstd" => {
                if let Ok(decoded) = zstd::decode_all(&current_body[..]) {
                    current_body = decoded;
                    decompressed = true;
                } else {
                    return None;
                }
            }
            "identity" | "" => {}
            _ => {
                return if decompressed { Some(current_body) } else { None };
            }
        }
    }

    if decompressed { Some(current_body) } else { None }
}

fn interpolate_text(input: &str, vars: &HashMap<String, String>) -> String {
    let with_vars = interpolate_variables(input, vars);
    interpolate_dates(&with_vars)
}

#[tauri::command]
pub async fn execute_repeater_request(app_handle: AppHandle, id: String) -> Result<proxy::Traffic, String> {
    let db_path = db::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;

    let active_env_id: Option<String> = conn.prepare("SELECT id FROM environments WHERE is_active = 1 LIMIT 1")
        .map_err(|e| e.to_string())?
        .query_row([], |row| row.get(0))
        .optional()
        .map_err(|e| e.to_string())?;

    let vars = build_variable_map(&conn, active_env_id.as_deref())?;

    let (method, url, headers, body) = {
        let mut stmt = conn.prepare("SELECT method, url, headers, body FROM repeater_requests WHERE id = ?").map_err(|e| e.to_string())?;
        stmt.query_row([id.clone()], |row| {
            let headers_str: String = row.get(2)?;
            let headers: Vec<(String, String)> = serde_json::from_str(&headers_str).unwrap_or_default();
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, headers, row.get::<_, String>(3)?))
        }).map_err(|e| e.to_string())?
    };

    let method = interpolate_text(&method, &vars);
    let url = interpolate_text(&url, &vars);
    let headers: Vec<(String, String)> = headers.into_iter()
        .map(|(k, v)| (interpolate_text(&k, &vars), interpolate_text(&v, &vars)))
        .collect();
    let body = interpolate_text(&body, &vars);

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_or_http()
        .enable_http1()
        .build();
    let client = Client::builder(TokioExecutor::new()).build(https);

    let mut builder = Request::builder()
        .method(method.as_str())
        .uri(&url);

    for (k, v) in headers.iter() {
        builder = builder.header(k, v);
    }

    let req = builder.body(Full::new(Bytes::from(body.clone()))).map_err(|e| e.to_string())?;
    let res = client.request(req).await.map_err(|e| e.to_string())?;

    let status = res.status().as_u16();
    let mut res_headers = Vec::new();
    for (name, value) in res.headers().iter() {
        res_headers.push((name.to_string(), value.to_str().unwrap_or("").to_string()));
    }

    let collected_body = res.into_body().collect().await.map_err(|e| e.to_string())?.to_bytes();
    let res_encoding = res_headers.iter()
        .find(|(k, _)| k.to_lowercase() == "content-encoding")
        .map(|(_, v)| v.clone())
        .unwrap_or_default();

    let decompressed_body = decompress_body(&collected_body, &res_encoding);
    let response_body_bytes = decompressed_body.as_deref().unwrap_or(&collected_body);
    if decompressed_body.is_some() {
        res_headers.retain(|(k, _)| k.to_lowercase() != "content-encoding");
    }
    let res_body_str = String::from_utf8_lossy(response_body_bytes).to_string();

    let traffic = proxy::Traffic {
        id: format!("{}_res", id),
        method: method.clone(),
        url: url.clone(),
        host: String::new(),
        status_code: status,
        request_headers: headers.clone(),
        response_headers: res_headers.clone(),
        request_body: body.clone(),
        response_body: res_body_str.clone(),
        phase: "response".to_string(),
        is_intercepted: false,
        intercepted_at: None,
    };

    let res_headers_json = serde_json::to_string(&res_headers).unwrap_or_default();
    let req_headers_json = serde_json::to_string(&headers).unwrap_or_default();
    {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE repeater_requests SET response_status = ?, response_headers = ?, response_body = ?, hit_count = hit_count + 1 WHERE id = ?",
            rusqlite::params![status, res_headers_json, res_body_str, id],
        ).map_err(|e| e.to_string())?;

        let history_id = uuid::Uuid::new_v4().to_string();
        let _ = conn.execute(
            "INSERT INTO repeater_history (id, repeater_id, method, url, request, response, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                history_id, id, method, url,
                serde_json::json!({ "headers": req_headers_json, "body": body }).to_string(),
                serde_json::json!({ "status": status, "headers": res_headers_json, "body": res_body_str }).to_string(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
            ],
        );
    }

    Ok(traffic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_vars_and_dates() {
        let mut vars = HashMap::new();
        vars.insert("apiUrl".to_string(), "https://example.com".to_string());
        vars.insert("token".to_string(), "abc123".to_string());

        let input = "{{apiUrl}}/users?token={{token}}&date=[[today+1/vu+3]]";
        let result = interpolate_text(input, &vars);

        assert!(result.starts_with("https://example.com/users?token=abc123&date="));
        assert!(!result.contains("[["));
        assert!(!result.contains("{{"));
    }
}
