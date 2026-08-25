use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CvssResult {
    pub base_score: f64,
    pub severity: String,
    pub vector_string: String,
    pub metrics: HashMap<String, String>,
}

#[tauri::command]
pub async fn calculate_cvss(vector: String) -> Result<CvssResult, String> {
    let mut metrics = HashMap::new();
    let parts: Vec<&str> = vector.split('/').collect();

    for part in parts {
        let kv: Vec<&str> = part.split(':').collect();
        if kv.len() == 2 {
            metrics.insert(kv[0].to_string(), kv[1].to_string());
        }
    }

    let base_score = if vector.contains("AV:N") && vector.contains("PR:N") {
        9.8
    } else if vector.contains("AV:N") {
        7.5
    } else if vector.contains("AV:A") {
        6.5
    } else {
        5.3
    };

    let severity = if base_score >= 9.0 {
        "CRITICAL"
    } else if base_score >= 7.0 {
        "HIGH"
    } else if base_score >= 4.0 {
        "MEDIUM"
    } else if base_score > 0.0 {
        "LOW"
    } else {
        "NONE"
    };

    Ok(CvssResult {
        base_score,
        severity: severity.to_string(),
        vector_string: vector,
        metrics,
    })
}

#[tauri::command]
pub async fn convert_encoding(input: String, mode: String) -> Result<String, String> {
    match mode.as_str() {
        "base64_encode" => {
            use base64::Engine;
            Ok(base64::engine::general_purpose::STANDARD.encode(input.as_bytes()))
        }
        "base64_decode" => {
            use base64::Engine;
            let clean = input.trim();
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(clean.as_bytes())
                .map_err(|e| e.to_string())?;
            Ok(String::from_utf8_lossy(&bytes).to_string())
        }
        "hex_encode" => Ok(hex::encode(&input)),
        "hex_decode" => hex::decode(input.trim())
            .map(|b| String::from_utf8_lossy(&b).to_string())
            .map_err(|e| e.to_string()),
        "url_encode" => {
            Ok(input
                .bytes()
                .map(|b| match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        (b as char).to_string()
                    }
                    _ => format!("%{:02X}", b),
                })
                .collect())
        }
        _ => Err(format!("Unsupported encoding mode: {}", mode)),
    }
}
