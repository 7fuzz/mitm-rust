use ring::hmac;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TotpAlgo {
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotpParams {
    pub secret: Vec<u8>,
    pub digits: u32,
    pub period: u64,
    pub algo: TotpAlgo,
}

impl TotpParams {
    pub fn new(secret: Vec<u8>) -> Self {
        Self { secret, digits: 6, period: 30, algo: TotpAlgo::Sha1 }
    }
}

pub fn code_at(params: &TotpParams, unix_secs: u64) -> String {
    let counter = unix_secs / params.period.max(1);
    let alg = match params.algo {
        TotpAlgo::Sha1 => hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
        TotpAlgo::Sha256 => hmac::HMAC_SHA256,
        TotpAlgo::Sha512 => hmac::HMAC_SHA512,
    };
    let key = hmac::Key::new(alg, &params.secret);
    let tag = hmac::sign(&key, &counter.to_be_bytes());
    let mac = tag.as_ref();
    let offset = (mac[mac.len() - 1] & 0x0f) as usize;
    let binary = u32::from_be_bytes([mac[offset] & 0x7f, mac[offset + 1], mac[offset + 2], mac[offset + 3]]);
    let digits = params.digits.clamp(1, 9);
    format!("{:0width$}", binary % 10u32.pow(digits), width = digits as usize)
}

pub fn remaining(period: u64, unix_secs: u64) -> u64 {
    let period = period.max(1);
    period - unix_secs % period
}

/// Below this many seconds left the next window's code is sent instead; servers accept ±1 step, so it
/// stays valid while the request is in flight.
pub const SEND_MARGIN_SECS: u64 = 2;

pub fn code_for_send(params: &TotpParams, unix_secs: u64) -> String {
    if remaining(params.period, unix_secs) < SEND_MARGIN_SECS {
        code_at(params, unix_secs + params.period)
    } else {
        code_at(params, unix_secs)
    }
}

pub fn decode_base32(input: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '=' && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if cleaned.is_empty() {
        return Err("empty secret".into());
    }
    data_encoding::BASE32_NOPAD
        .decode(cleaned.as_bytes())
        .map_err(|_| "invalid base32 secret".to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtpAuth {
    pub params: TotpParams,
    pub label: String,
    pub issuer: Option<String>,
}

pub fn parse_otpauth(uri: &str) -> Result<OtpAuth, String> {
    let url = url::Url::parse(uri.trim()).map_err(|_| "invalid otpauth URI".to_string())?;
    if !url.scheme().eq_ignore_ascii_case("otpauth") {
        return Err("not an otpauth URI".into());
    }
    match url.host_str().map(|h| h.to_ascii_lowercase()) {
        Some(h) if h == "totp" => {}
        Some(h) if h == "hotp" => return Err("HOTP is not supported".into()),
        _ => return Err("otpauth URI must be totp".into()),
    }

    let path = url.path().trim_start_matches('/');
    let path = urlencoding::decode(path).map(|p| p.into_owned()).unwrap_or_else(|_| path.to_string());
    let (label_issuer, label) = match path.split_once(':') {
        Some((issuer, account)) => (Some(issuer.trim().to_string()), account.trim().to_string()),
        None => (None, path.trim().to_string()),
    };

    let mut secret = None;
    let mut issuer = None;
    let mut params = TotpParams::new(Vec::new());
    for (key, value) in url.query_pairs() {
        match key.to_ascii_lowercase().as_str() {
            "secret" => secret = Some(decode_base32(&value)?),
            "issuer" => issuer = Some(value.trim().to_string()),
            "digits" => {
                params.digits = value.parse().ok().filter(|d| (6..=9).contains(d)).ok_or("invalid digits")?;
            }
            "period" => params.period = value.parse().ok().filter(|p| *p > 0).ok_or("invalid period")?,
            "algorithm" => {
                params.algo = match value.to_ascii_uppercase().as_str() {
                    "SHA1" => TotpAlgo::Sha1,
                    "SHA256" => TotpAlgo::Sha256,
                    "SHA512" => TotpAlgo::Sha512,
                    _ => return Err("unsupported algorithm".into()),
                }
            }
            _ => {}
        }
    }
    params.secret = secret.ok_or("otpauth URI has no secret")?;
    let issuer = issuer.filter(|i| !i.is_empty()).or(label_issuer.filter(|i| !i.is_empty()));
    Ok(OtpAuth { params, label, issuer })
}

pub fn parse_secret(input: &str) -> Result<TotpParams, String> {
    let input = input.trim();
    if input.len() >= 10 && input[..10].eq_ignore_ascii_case("otpauth://") {
        return parse_otpauth(input).map(|o| o.params);
    }
    decode_base32(input).map(TotpParams::new)
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rfc_params(algo: TotpAlgo) -> TotpParams {
        let seed: &[u8] = match algo {
            TotpAlgo::Sha1 => b"12345678901234567890",
            TotpAlgo::Sha256 => b"12345678901234567890123456789012",
            TotpAlgo::Sha512 => b"1234567890123456789012345678901234567890123456789012345678901234",
        };
        TotpParams { secret: seed.to_vec(), digits: 8, period: 30, algo }
    }

    #[test]
    fn matches_rfc_6238_vectors() {
        let cases: [(u64, &str, &str, &str); 6] = [
            (59, "94287082", "46119246", "90693936"),
            (1111111109, "07081804", "68084774", "25091201"),
            (1111111111, "14050471", "67062674", "99943326"),
            (1234567890, "89005924", "91819424", "93441116"),
            (2000000000, "69279037", "90698825", "38618901"),
            (20000000000, "65353130", "77737706", "47863826"),
        ];
        for (t, sha1, sha256, sha512) in cases {
            assert_eq!(code_at(&rfc_params(TotpAlgo::Sha1), t), sha1, "sha1 t={t}");
            assert_eq!(code_at(&rfc_params(TotpAlgo::Sha256), t), sha256, "sha256 t={t}");
            assert_eq!(code_at(&rfc_params(TotpAlgo::Sha512), t), sha512, "sha512 t={t}");
        }
    }

    #[test]
    fn six_digit_codes_keep_leading_zeros() {
        let mut p = rfc_params(TotpAlgo::Sha1);
        p.digits = 6;
        assert_eq!(code_at(&p, 1111111109), "081804");
    }

    #[test]
    fn remaining_counts_down_to_the_window_edge() {
        assert_eq!(remaining(30, 0), 30);
        assert_eq!(remaining(30, 29), 1);
        assert_eq!(remaining(30, 30), 30);
    }

    #[test]
    fn code_for_send_uses_next_window_near_the_edge() {
        let p = rfc_params(TotpAlgo::Sha1);
        assert_eq!(code_for_send(&p, 1111111110), code_at(&p, 1111111110));
        assert_eq!(code_for_send(&p, 1111111139), code_at(&p, 1111111140));
        assert_eq!(code_for_send(&p, 1111111138), code_at(&p, 1111111138));
    }

    #[test]
    fn base32_accepts_lowercase_spaces_and_missing_padding() {
        let expected = b"12345678901234567890".to_vec();
        assert_eq!(decode_base32("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ").unwrap(), expected);
        assert_eq!(decode_base32("gezd gnbv gy3t qojq gezd gnbv gy3t qojq").unwrap(), expected);
        assert_eq!(decode_base32("MZXW6===").unwrap(), b"foo");
        assert_eq!(decode_base32("mzxw6").unwrap(), b"foo");
    }

    #[test]
    fn otpauth_reads_label_issuer_and_defaults() {
        let o = parse_otpauth("otpauth://totp/ACME%20Co:john.doe%40email.com?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ").unwrap();
        assert_eq!(o.label, "john.doe@email.com");
        assert_eq!(o.issuer.as_deref(), Some("ACME Co"));
        assert_eq!(o.params, TotpParams::new(b"12345678901234567890".to_vec()));
    }

    #[test]
    fn otpauth_issuer_param_wins_over_label_prefix() {
        let o = parse_otpauth("otpauth://totp/Old:alice?secret=MZXW6&issuer=New&digits=8&period=60&algorithm=SHA256")
            .unwrap();
        assert_eq!(o.issuer.as_deref(), Some("New"));
        assert_eq!(o.label, "alice");
        assert_eq!(o.params.digits, 8);
        assert_eq!(o.params.period, 60);
        assert_eq!(o.params.algo, TotpAlgo::Sha256);
    }

    #[test]
    fn otpauth_without_issuer_keeps_plain_label() {
        let o = parse_otpauth("otpauth://totp/alice?secret=MZXW6").unwrap();
        assert_eq!(o.label, "alice");
        assert_eq!(o.issuer, None);
    }

    #[test]
    fn otpauth_rejects_hotp_and_missing_secret() {
        assert!(parse_otpauth("otpauth://hotp/x?secret=MZXW6&counter=1").is_err());
        assert!(parse_otpauth("otpauth://totp/x?digits=6").is_err());
        assert!(parse_otpauth("otpauth://totp/x?secret=MZXW6&algorithm=MD5").is_err());
    }

    #[test]
    fn parse_secret_accepts_base32_or_uri() {
        assert_eq!(parse_secret(" mzxw6 ").unwrap().secret, b"foo");
        let p = parse_secret("otpauth://totp/x?secret=MZXW6&digits=8").unwrap();
        assert_eq!((p.secret.as_slice(), p.digits), (b"foo".as_slice(), 8));
    }

    #[test]
    fn base32_rejects_invalid_input() {
        assert!(decode_base32("GEZD1!").is_err());
        assert!(decode_base32("   ").is_err());
    }
}
