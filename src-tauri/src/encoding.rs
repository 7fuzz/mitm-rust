use std::io::Read;
use flate2::read::{GzDecoder, ZlibDecoder};
use base64::Engine;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlEncodedParamSpec {
    pub id: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub key: String,
    pub value: String,
}

pub fn build_urlencoded_payload(content_json: &str) -> Result<Vec<(String, String)>, String> {
    let params: Vec<UrlEncodedParamSpec> = serde_json::from_str(content_json)
        .map_err(|e| format!("Invalid URL-encoded payload JSON format: {}", e))?;

    let active_tuples: Vec<(String, String)> = params
        .into_iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .map(|p| (p.key.trim().to_string(), p.value))
        .collect();

    Ok(active_tuples)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartFieldSpec {
    pub id: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub key: String,
    pub value: String,
    #[serde(default = "default_text_type")]
    pub r#type: String, // 'text' | 'file_path' | 'base64'
    pub file_name: Option<String>,
    pub content_type: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_text_type() -> String {
    "text".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartBodySpec {
    pub __form_data: Vec<MultipartFieldSpec>,
}

pub fn infer_mime_type(file_name_or_path: &str) -> String {
    let lower = file_name_or_path.to_lowercase();
    if lower.ends_with(".png") {
        "image/png".to_string()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if lower.ends_with(".gif") {
        "image/gif".to_string()
    } else if lower.ends_with(".webp") {
        "image/webp".to_string()
    } else if lower.ends_with(".svg") {
        "image/svg+xml".to_string()
    } else if lower.ends_with(".pdf") {
        "application/pdf".to_string()
    } else if lower.ends_with(".json") {
        "application/json".to_string()
    } else if lower.ends_with(".txt") {
        "text/plain".to_string()
    } else if lower.ends_with(".html") || lower.ends_with(".htm") {
        "text/html".to_string()
    } else if lower.ends_with(".zip") {
        "application/zip".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

pub fn build_multipart_payload(content_json: &str) -> Result<Form, String> {
    let mut form = Form::new();

    let fields: Vec<MultipartFieldSpec> = if let Ok(spec) = serde_json::from_str::<MultipartBodySpec>(content_json) {
        spec.__form_data
    } else if let Ok(list) = serde_json::from_str::<Vec<MultipartFieldSpec>>(content_json) {
        list
    } else {
        return Err("Invalid multipart form-data payload JSON format".to_string());
    };

    for field in fields {
        if !field.enabled || field.key.trim().is_empty() {
            continue;
        }

        let key = field.key.trim().to_string();
        let field_type = field.r#type.to_lowercase();

        match field_type.as_str() {
            "file_path" => {
                let path = std::path::Path::new(&field.value);
                if path.exists() && path.is_file() {
                    let data = std::fs::read(path)
                        .map_err(|e| format!("Failed to read file '{}': {}", field.value, e))?;

                    let file_name = field.file_name.unwrap_or_else(|| {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("file")
                            .to_string()
                    });

                    let mime = field.content_type.unwrap_or_else(|| infer_mime_type(&file_name));

                    let part = match Part::bytes(data.clone()).file_name(file_name.clone()).mime_str(&mime) {
                        Ok(p) => p,
                        Err(_) => Part::bytes(data).file_name(file_name),
                    };
                    form = form.part(key, part);
                } else {
                    return Err(format!("Multipart file path not found: {}", field.value));
                }
            }
            "base64" => {
                let raw_val = field.value.trim();
                let base64_str = if let Some(pos) = raw_val.find(";base64,") {
                    &raw_val[pos + 8..]
                } else {
                    raw_val
                };

                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(base64_str)
                    .map_err(|e| format!("Invalid Base64 payload in field '{}': {}", key, e))?;

                let file_name = field.file_name.unwrap_or_else(|| "file".to_string());
                let mime = field.content_type.unwrap_or_else(|| infer_mime_type(&file_name));

                let part = match Part::bytes(decoded.clone()).file_name(file_name.clone()).mime_str(&mime) {
                    Ok(p) => p,
                    Err(_) => Part::bytes(decoded).file_name(file_name),
                };
                form = form.part(key, part);
            }
            _ => {
                form = form.text(key, field.value);
            }
        }
    }

    Ok(form)
}

/// Decompresses raw body bytes based on Content-Encoding (gzip, zstd, brotli, deflate)
pub fn decompress_body(body: &[u8], encoding: &str) -> Vec<u8> {
    if encoding.trim().is_empty() {
        return body.to_vec();
    }

    let encodings: Vec<&str> = encoding.split(',').map(|s| s.trim()).collect();
    let mut current_body = body.to_vec();

    for enc in encodings.iter().rev() {
        let enc = enc.to_lowercase();
        match enc.as_str() {
            "gzip" | "x-gzip" => {
                let mut decoder = GzDecoder::new(&current_body[..]);
                let mut decoded = Vec::new();
                if decoder.read_to_end(&mut decoded).is_ok() {
                    current_body = decoded;
                }
            }
            "deflate" => {
                let mut decoder = ZlibDecoder::new(&current_body[..]);
                let mut decoded = Vec::new();
                if decoder.read_to_end(&mut decoded).is_ok() {
                    current_body = decoded;
                }
            }
            "br" => {
                let mut decoded = Vec::new();
                if brotli::Decompressor::new(&current_body[..], 4096).read_to_end(&mut decoded).is_ok() {
                    current_body = decoded;
                }
            }
            "zstd" => {
                if let Ok(decoded) = zstd::decode_all(&current_body[..]) {
                    current_body = decoded;
                }
            }
            _ => {}
        }
    }

    current_body
}

/// Formats response body bytes into clean UTF-8 text or base64 data string
pub fn format_body_for_ui(body: &[u8], content_type: &str, content_encoding: &str) -> String {
    let decompressed = decompress_body(body, content_encoding);

    let is_binary = !content_type.is_empty() && (
        content_type.contains("image/") ||
        content_type.contains("video/") ||
        content_type.contains("audio/") ||
        content_type.contains("application/octet-stream") ||
        content_type.contains("application/pdf") ||
        content_type.contains("application/zip") ||
        content_type.contains("application/gzip") ||
        content_type.contains("font/")
    );

    // Multipart uploads mix text fields with raw file bytes; a lossy UTF-8 decode would corrupt the files.
    let is_binary = is_binary
        || (content_type.contains("multipart/") && std::str::from_utf8(&decompressed).is_err());

    if is_binary {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&decompressed);
        format!("base64:{}", encoded)
    } else {
        String::from_utf8(decompressed.clone())
            .unwrap_or_else(|_| String::from_utf8_lossy(&decompressed).to_string())
    }
}
