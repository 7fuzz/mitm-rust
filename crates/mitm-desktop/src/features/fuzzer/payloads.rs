use serde::{Deserialize, Serialize};
use mitm_core::fuzzer::{PayloadSet, MAX_REQUESTS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AttackType {
    #[default]
    Sniper,
    Pitchfork,
    ClusterBomb,
}

impl AttackType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sniper => "sniper",
            Self::Pitchfork => "pitchfork",
            Self::ClusterBomb => "clusterbomb",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Sniper => "Sniper",
            Self::Pitchfork => "Pitchfork",
            Self::ClusterBomb => "Cluster Bomb",
        }
    }

    pub fn tooltip(&self) -> &'static str {
        match self {
            Self::Sniper => "Each variable is fuzzed in turn while the others hold their first payload.",
            Self::Pitchfork => "All variables advance together, stopping at the shortest list.",
            Self::ClusterBomb => "Every combination of the variables' payloads.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayloadKind {
    List,
    Numbers,
}

/// The editing state for one variable's payloads; `to_set` turns it into a core `PayloadSet`.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PayloadConfig {
    pub kind: PayloadKind,
    pub list_text: String,
    pub from: f64,
    pub to: f64,
    pub step: f64,
    pub pad: u32,
    pub prefix: String,
    pub suffix: String,
    pub url_encode: bool,
}

impl Default for PayloadConfig {
    fn default() -> Self {
        Self {
            kind: PayloadKind::List,
            list_text: String::new(),
            from: 0.0,
            to: 10.0,
            step: 1.0,
            pad: 0,
            prefix: String::new(),
            suffix: String::new(),
            url_encode: false,
        }
    }
}

impl PayloadConfig {
    pub(super) fn to_set(&self) -> PayloadSet {
        PayloadSet {
            kind: match self.kind {
                PayloadKind::List => "list",
                PayloadKind::Numbers => "numbers",
            }
            .to_string(),
            list: self.list_text.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect(),
            from: self.from,
            to: self.to,
            step: self.step,
            pad: self.pad,
            url_encode: self.url_encode,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
        }
    }

    /// The payload count without expanding the whole list, so a huge number range stays cheap.
    pub fn count(&self) -> usize {
        match self.kind {
            PayloadKind::List => self.list_text.lines().filter(|l| !l.trim().is_empty()).count(),
            PayloadKind::Numbers => {
                let step = if self.step == 0.0 { 1.0 } else { self.step };
                let span = self.to - self.from;
                if span != 0.0 && (span > 0.0) != (step > 0.0) {
                    0
                } else {
                    (((span / step).floor() as i64) + 1).clamp(0, MAX_REQUESTS as i64) as usize
                }
            }
        }
    }
}
