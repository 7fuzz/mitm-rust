use std::io::Read;
use flate2::read::{GzDecoder, ZlibDecoder};
use base64::Engine;

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

    if is_binary {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&decompressed);
        format!("base64:{}", encoded)
    } else {
        String::from_utf8(decompressed.clone())
            .unwrap_or_else(|_| String::from_utf8_lossy(&decompressed).to_string())
    }
}
