use md5::{Digest, Md5};
use ring::{digest, hmac};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HmacAlgo {
    Sha1,
    Sha256,
    Sha512,
}

pub fn md5_hex(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn sha1_hex(data: &[u8]) -> String {
    hex::encode(digest::digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, data))
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(digest::digest(&digest::SHA256, data))
}

pub fn sha512_hex(data: &[u8]) -> String {
    hex::encode(digest::digest(&digest::SHA512, data))
}

pub fn hmac_hex(algo: HmacAlgo, key: &[u8], data: &[u8]) -> String {
    let alg = match algo {
        HmacAlgo::Sha1 => hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
        HmacAlgo::Sha256 => hmac::HMAC_SHA256,
        HmacAlgo::Sha512 => hmac::HMAC_SHA512,
    };
    let key = hmac::Key::new(alg, key);
    hex::encode(hmac::sign(&key, data).as_ref())
}

pub fn bcrypt_hash(password: &str, cost: u32) -> Result<String, String> {
    bcrypt::hash(password, cost).map_err(|e| e.to_string())
}

pub fn bcrypt_verify(password: &str, hash: &str) -> Result<bool, String> {
    bcrypt::verify(password, hash).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digests_match_known_vectors() {
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(
            sha512_hex(b"abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn hmac_matches_known_vector() {
        assert_eq!(
            hmac_hex(HmacAlgo::Sha256, b"key", b"The quick brown fox jumps over the lazy dog"),
            "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
        );
    }

    #[test]
    fn bcrypt_round_trips_and_rejects_wrong_password() {
        let hash = bcrypt_hash("hunter2", 4).unwrap();
        assert!(bcrypt_verify("hunter2", &hash).unwrap());
        assert!(!bcrypt_verify("wrong", &hash).unwrap());
        assert!(bcrypt_verify("x", "not-a-bcrypt-hash").is_err());
    }
}
