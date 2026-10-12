use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};
use zeroize::Zeroizing;

pub const KEY_LEN: usize = 32;

pub type DataKey = Zeroizing<[u8; KEY_LEN]>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealed {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut out = [0u8; N];
    SystemRandom::new().fill(&mut out).expect("system RNG");
    out
}

pub fn generate_key() -> DataKey {
    Zeroizing::new(random_bytes::<KEY_LEN>())
}

pub fn key_from_slice(bytes: &[u8]) -> Result<DataKey, String> {
    let array: [u8; KEY_LEN] = bytes.try_into().map_err(|_| "data key has the wrong length".to_string())?;
    Ok(Zeroizing::new(array))
}

fn aead_key(key: &[u8; KEY_LEN]) -> LessSafeKey {
    LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).expect("32-byte key"))
}

/// AES-256-GCM with a fresh random nonce; `aad` binds the ciphertext to its row.
pub fn seal(key: &[u8; KEY_LEN], aad: &[u8], plaintext: &[u8]) -> Sealed {
    let nonce = random_bytes::<NONCE_LEN>();
    let mut buffer = plaintext.to_vec();
    aead_key(key)
        .seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce), Aad::from(aad), &mut buffer)
        .expect("AES-GCM seal");
    Sealed { nonce: nonce.to_vec(), ciphertext: buffer }
}

pub fn open(key: &[u8; KEY_LEN], aad: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| "bad nonce".to_string())?;
    let mut buffer = Zeroizing::new(ciphertext.to_vec());
    let len = aead_key(key)
        .open_in_place(nonce, Aad::from(aad), &mut buffer)
        .map_err(|_| "decryption failed".to_string())?
        .len();
    buffer.truncate(len);
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_then_open_round_trips() {
        let key = generate_key();
        let sealed = seal(&key, b"row-1", b"hunter2");
        assert_ne!(sealed.ciphertext, b"hunter2");
        assert_eq!(open(&key, b"row-1", &sealed.nonce, &sealed.ciphertext).unwrap().as_slice(), b"hunter2");
    }

    #[test]
    fn every_seal_uses_a_new_nonce() {
        let key = generate_key();
        let a = seal(&key, b"", b"same");
        let b = seal(&key, b"", b"same");
        assert_ne!(a.nonce, b.nonce);
        assert_ne!(a.ciphertext, b.ciphertext);
    }

    #[test]
    fn tampering_wrong_key_or_wrong_row_fails() {
        let key = generate_key();
        let sealed = seal(&key, b"row-1", b"hunter2");
        let mut tampered = sealed.ciphertext.clone();
        tampered[0] ^= 1;
        assert!(open(&key, b"row-1", &sealed.nonce, &tampered).is_err());
        assert!(open(&generate_key(), b"row-1", &sealed.nonce, &sealed.ciphertext).is_err());
        assert!(open(&key, b"row-2", &sealed.nonce, &sealed.ciphertext).is_err());
    }
}
