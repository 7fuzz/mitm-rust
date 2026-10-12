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

pub fn parse_secret(input: &str) -> Result<TotpParams, String> {
    decode_base32(input.trim()).map(TotpParams::new)
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
    fn base32_accepts_lowercase_spaces_and_missing_padding() {
        let expected = b"12345678901234567890".to_vec();
        assert_eq!(decode_base32("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ").unwrap(), expected);
        assert_eq!(decode_base32("gezd gnbv gy3t qojq gezd gnbv gy3t qojq").unwrap(), expected);
        assert_eq!(decode_base32("MZXW6===").unwrap(), b"foo");
        assert_eq!(decode_base32("mzxw6").unwrap(), b"foo");
    }

    #[test]
    fn base32_rejects_invalid_input() {
        assert!(decode_base32("GEZD1!").is_err());
        assert!(decode_base32("   ").is_err());
    }
}
