mod view;

use serde::{Deserialize, Serialize};

pub use view::show;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Provider {
    GitHub,
    Stripe,
    Shopify,
    Sha1,
    Sha256,
    Sha512,
}

impl Provider {
    pub const ALL: [Provider; 6] = [
        Provider::GitHub,
        Provider::Stripe,
        Provider::Shopify,
        Provider::Sha1,
        Provider::Sha256,
        Provider::Sha512,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Provider::GitHub => "GitHub",
            Provider::Stripe => "Stripe",
            Provider::Shopify => "Shopify",
            Provider::Sha1 => "SHA-1 hex",
            Provider::Sha256 => "SHA-256 hex",
            Provider::Sha512 => "SHA-512 hex",
        }
    }

    fn core_name(&self) -> &'static str {
        match self {
            Provider::GitHub => "github",
            Provider::Stripe => "stripe",
            Provider::Shopify => "shopify",
            Provider::Sha1 => "raw_sha1",
            Provider::Sha256 => "sha256",
            Provider::Sha512 => "raw_sha512",
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct HmacToolPrefs {
    pub provider: Provider,
}

impl Default for HmacToolPrefs {
    fn default() -> Self {
        Self { provider: Provider::GitHub }
    }
}

#[derive(Default)]
pub struct HmacToolState {
    pub prefs: HmacToolPrefs,
    pub secret: String,
    pub payload: String,
    cache: Option<(u64, (String, String))>,
}

impl HmacToolState {
    /// Opens the tool prefilled from a webhook delivery ("Recompute with…").
    pub fn prefill(&mut self, secret: String, payload: String) {
        self.secret = secret;
        self.payload = payload;
        self.cache = None;
    }

    /// The signature header name and value, recomputed only when inputs change
    /// (so Stripe's timestamp stays stable instead of ticking every frame).
    pub(crate) fn signature(&mut self) -> &(String, String) {
        let key = self.key();
        if self.cache.as_ref().map(|(k, _)| *k) != Some(key) {
            let (name, value) =
                mitm_core::webhook::calculate_signature(&self.secret, &self.payload, self.prefs.provider.core_name());
            self.cache = Some((key, (name, value)));
        }
        &self.cache.as_ref().unwrap().1
    }

    fn key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (self.prefs.provider as u8).hash(&mut h);
        self.secret.hash(&mut h);
        self.payload.hash(&mut h);
        h.finish()
    }
}
