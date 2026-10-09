mod view;

use mitm_core::utils::encoding::{self as enc, HexFormat, JwtParts};
use serde::{Deserialize, Serialize};

pub use view::show;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Algorithm {
    Base64,
    Base64Url,
    Url,
    Hex,
    Html,
    Unicode,
    Jwt,
}

impl Algorithm {
    pub const ALL: [Algorithm; 7] = [
        Algorithm::Base64,
        Algorithm::Base64Url,
        Algorithm::Url,
        Algorithm::Hex,
        Algorithm::Html,
        Algorithm::Unicode,
        Algorithm::Jwt,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Algorithm::Base64 => "Base64",
            Algorithm::Base64Url => "Base64 URL",
            Algorithm::Url => "URL",
            Algorithm::Hex => "Hex",
            Algorithm::Html => "HTML entities",
            Algorithm::Unicode => "Unicode escape",
            Algorithm::Jwt => "JWT decode",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum HexFmt {
    Plain,
    Spaced,
    Colon,
    ZeroX,
    BackslashX,
}

impl HexFmt {
    pub const ALL: [HexFmt; 5] = [HexFmt::Plain, HexFmt::Spaced, HexFmt::Colon, HexFmt::ZeroX, HexFmt::BackslashX];

    pub fn label(&self) -> &'static str {
        match self {
            HexFmt::Plain => "4d49",
            HexFmt::Spaced => "4d 49",
            HexFmt::Colon => "4d:49",
            HexFmt::ZeroX => "0x4d 0x49",
            HexFmt::BackslashX => "\\x4d\\x49",
        }
    }

    fn to_core(self) -> HexFormat {
        match self {
            HexFmt::Plain => HexFormat::Plain,
            HexFmt::Spaced => HexFormat::Spaced,
            HexFmt::Colon => HexFormat::Colon,
            HexFmt::ZeroX => HexFormat::ZeroX,
            HexFmt::BackslashX => HexFormat::BackslashX,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct EncodingPrefs {
    pub split: f32,
    pub algorithm: Algorithm,
    pub decode: bool,
    pub hex_format: HexFmt,
    pub hex_upper: bool,
    pub url_full: bool,
}

impl Default for EncodingPrefs {
    fn default() -> Self {
        Self {
            split: 0.5,
            algorithm: Algorithm::Base64,
            decode: false,
            hex_format: HexFmt::Spaced,
            hex_upper: false,
            url_full: false,
        }
    }
}

#[derive(Clone)]
pub(crate) enum Computed {
    Text(Result<String, String>),
    Jwt(Result<JwtParts, String>),
}

#[derive(Default)]
pub struct EncodingState {
    pub prefs: EncodingPrefs,
    pub input: String,
    cache: Option<(u64, Computed)>,
}

impl EncodingState {
    /// Fills the input and switches to decode, used by "Decode in Encoding tool".
    pub fn load_for_decode(&mut self, text: String) {
        self.input = text;
        self.prefs.decode = true;
        self.cache = None;
    }

    pub(crate) fn output(&mut self) -> &Computed {
        let key = self.key();
        if self.cache.as_ref().map(|(k, _)| *k) != Some(key) {
            let computed = compute(&self.prefs, &self.input);
            self.cache = Some((key, computed));
        }
        &self.cache.as_ref().unwrap().1
    }

    fn key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.input.hash(&mut h);
        (self.prefs.algorithm as u8).hash(&mut h);
        self.prefs.decode.hash(&mut h);
        (self.prefs.hex_format as u8).hash(&mut h);
        self.prefs.hex_upper.hash(&mut h);
        self.prefs.url_full.hash(&mut h);
        h.finish()
    }
}

fn compute(prefs: &EncodingPrefs, input: &str) -> Computed {
    if let Algorithm::Jwt = prefs.algorithm {
        return Computed::Jwt(enc::jwt_decode(input));
    }
    let result = if prefs.decode {
        match prefs.algorithm {
            Algorithm::Base64 => enc::base64_decode(input, false),
            Algorithm::Base64Url => enc::base64_decode(input, true),
            Algorithm::Url => enc::url_decode(input),
            Algorithm::Hex => enc::hex_decode(input),
            Algorithm::Html => Ok(enc::html_decode(input)),
            Algorithm::Unicode => enc::unicode_unescape(input),
            Algorithm::Jwt => unreachable!(),
        }
    } else {
        let text = match prefs.algorithm {
            Algorithm::Base64 => enc::base64_encode(input, false),
            Algorithm::Base64Url => enc::base64_encode(input, true),
            Algorithm::Url => enc::url_encode(input, prefs.url_full),
            Algorithm::Hex => enc::hex_encode(input, prefs.hex_format.to_core(), prefs.hex_upper),
            Algorithm::Html => enc::html_encode(input),
            Algorithm::Unicode => enc::unicode_escape(input),
            Algorithm::Jwt => unreachable!(),
        };
        Ok(text)
    };
    Computed::Text(result)
}
