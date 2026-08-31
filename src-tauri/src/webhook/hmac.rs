use ring::hmac;
use base64::Engine;

pub fn compute_hmac_sha256(secret: &[u8], data: &[u8]) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
    let tag = hmac::sign(&key, data);
    tag.as_ref().to_vec()
}

pub fn compute_hmac_sha1(secret: &[u8], data: &[u8]) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret);
    let tag = hmac::sign(&key, data);
    tag.as_ref().to_vec()
}

pub fn compute_hmac_sha512(secret: &[u8], data: &[u8]) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA512, secret);
    let tag = hmac::sign(&key, data);
    tag.as_ref().to_vec()
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn calculate_signature(secret: &str, body: &str, provider: &str) -> (String, String) {
    match provider.to_lowercase().as_str() {
        "github" => {
            let hmac = compute_hmac_sha256(secret.as_bytes(), body.as_bytes());
            (
                "X-Hub-Signature-256".to_string(),
                format!("sha256={}", to_hex(&hmac)),
            )
        }
        "stripe" => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let payload = format!("{}.{}", now, body);
            let hmac = compute_hmac_sha256(secret.as_bytes(), payload.as_bytes());
            (
                "Stripe-Signature".to_string(),
                format!("t={},v1={}", now, to_hex(&hmac)),
            )
        }
        "shopify" => {
            let hmac = compute_hmac_sha256(secret.as_bytes(), body.as_bytes());
            let encoded = base64::engine::general_purpose::STANDARD.encode(&hmac);
            (
                "X-Shopify-Hmac-SHA256".to_string(),
                encoded,
            )
        }
        "raw_sha1" => {
            let hmac = compute_hmac_sha1(secret.as_bytes(), body.as_bytes());
            (
                "X-Signature-SHA1".to_string(),
                to_hex(&hmac),
            )
        }
        "raw_sha512" => {
            let hmac = compute_hmac_sha512(secret.as_bytes(), body.as_bytes());
            (
                "X-Signature-SHA512".to_string(),
                to_hex(&hmac),
            )
        }
        _ => {
            let hmac = compute_hmac_sha256(secret.as_bytes(), body.as_bytes());
            (
                "X-Signature-SHA256".to_string(),
                to_hex(&hmac),
            )
        }
    }
}

pub fn verify_signature(
    headers: &[(String, String)],
    body: &[u8],
    secret: &str,
) -> (String, Option<String>, Option<String>) {
    if secret.trim().is_empty() {
        return ("none".to_string(), None, None);
    }

    let get_header = |name: &str| -> Option<String> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim().to_string())
    };

    // 1. GitHub SHA-256
    if let Some(provided) = get_header("x-hub-signature-256") {
        let expected = format!("sha256={}", to_hex(&compute_hmac_sha256(secret.as_bytes(), body)));
        let status = if provided == expected || provided.trim_start_matches("sha256=") == expected.trim_start_matches("sha256=") {
            "valid"
        } else {
            "invalid"
        };
        return (status.to_string(), Some(expected), Some(provided));
    }

    // 2. GitHub SHA-1 (Legacy)
    if let Some(provided) = get_header("x-hub-signature") {
        let expected = format!("sha1={}", to_hex(&compute_hmac_sha1(secret.as_bytes(), body)));
        let status = if provided == expected || provided.trim_start_matches("sha1=") == expected.trim_start_matches("sha1=") {
            "valid"
        } else {
            "invalid"
        };
        return (status.to_string(), Some(expected), Some(provided));
    }

    // 3. Stripe Signature (t=...,v1=...)
    if let Some(provided) = get_header("stripe-signature") {
        let mut t_val = "";
        let mut v1_val = "";
        for part in provided.split(',') {
            let mut kv = part.splitn(2, '=');
            let k = kv.next().unwrap_or("").trim();
            let v = kv.next().unwrap_or("").trim();
            if k == "t" {
                t_val = v;
            } else if k == "v1" {
                v1_val = v;
            }
        }

        if !t_val.is_empty() && !v1_val.is_empty() {
            let signed_payload = format!("{}.{}", t_val, String::from_utf8_lossy(body));
            let expected_hash = to_hex(&compute_hmac_sha256(secret.as_bytes(), signed_payload.as_bytes()));
            let expected = format!("t={},v1={}", t_val, expected_hash);
            let status = if v1_val == expected_hash { "valid" } else { "invalid" };
            return (status.to_string(), Some(expected), Some(provided));
        } else {
            return ("invalid".to_string(), None, Some(provided));
        }
    }

    // 4. Shopify Base64 HMAC SHA-256
    if let Some(provided) = get_header("x-shopify-hmac-sha256") {
        let expected = base64::engine::general_purpose::STANDARD.encode(&compute_hmac_sha256(secret.as_bytes(), body));
        let status = if provided == expected { "valid" } else { "invalid" };
        return (status.to_string(), Some(expected), Some(provided));
    }

    // 5. Generic X-Signature / X-Signature-256
    if let Some(provided) = get_header("x-signature").or_else(|| get_header("x-signature-256")) {
        let expected = to_hex(&compute_hmac_sha256(secret.as_bytes(), body));
        let status = if provided == expected { "valid" } else { "invalid" };
        return (status.to_string(), Some(expected), Some(provided));
    }

    ("none".to_string(), None, None)
}
