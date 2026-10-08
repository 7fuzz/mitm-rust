pub mod v3;
pub mod v4;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CvssVersion {
    V3_1,
    V4_0,
}

impl CvssVersion {
    pub const ALL: [CvssVersion; 2] = [CvssVersion::V3_1, CvssVersion::V4_0];

    pub fn label(self) -> &'static str {
        match self {
            CvssVersion::V3_1 => "CVSS 3.1",
            CvssVersion::V4_0 => "CVSS 4.0",
        }
    }
}

pub use v3::{CvssV3Base, Scope, UserInteraction};
pub use v4::{AttackRequirements, CvssV4Base, UserInteractionV4};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackVector {
    Network,
    Adjacent,
    Local,
    Physical,
}

impl AttackVector {
    pub const ALL: [AttackVector; 4] = [
        AttackVector::Network,
        AttackVector::Adjacent,
        AttackVector::Local,
        AttackVector::Physical,
    ];

    pub fn code(self) -> &'static str {
        match self {
            AttackVector::Network => "N",
            AttackVector::Adjacent => "A",
            AttackVector::Local => "L",
            AttackVector::Physical => "P",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AttackVector::Network => "Network (N)",
            AttackVector::Adjacent => "Adjacent (A)",
            AttackVector::Local => "Local (L)",
            AttackVector::Physical => "Physical (P)",
        }
    }

    pub fn weight(self) -> f64 {
        match self {
            AttackVector::Network => 0.85,
            AttackVector::Adjacent => 0.62,
            AttackVector::Local => 0.55,
            AttackVector::Physical => 0.20,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackComplexity {
    Low,
    High,
}

impl AttackComplexity {
    pub const ALL: [AttackComplexity; 2] = [AttackComplexity::Low, AttackComplexity::High];

    pub fn code(self) -> &'static str {
        match self {
            AttackComplexity::Low => "L",
            AttackComplexity::High => "H",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AttackComplexity::Low => "Low (L)",
            AttackComplexity::High => "High (H)",
        }
    }

    pub fn weight(self) -> f64 {
        match self {
            AttackComplexity::Low => 0.77,
            AttackComplexity::High => 0.44,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivilegesRequired {
    None,
    Low,
    High,
}

impl PrivilegesRequired {
    pub const ALL: [PrivilegesRequired; 3] = [
        PrivilegesRequired::None,
        PrivilegesRequired::Low,
        PrivilegesRequired::High,
    ];

    pub fn code(self) -> &'static str {
        match self {
            PrivilegesRequired::None => "N",
            PrivilegesRequired::Low => "L",
            PrivilegesRequired::High => "H",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PrivilegesRequired::None => "None (N)",
            PrivilegesRequired::Low => "Low (L)",
            PrivilegesRequired::High => "High (H)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImpactMetric {
    None,
    Low,
    High,
}

impl ImpactMetric {
    pub const ALL: [ImpactMetric; 3] = [ImpactMetric::None, ImpactMetric::Low, ImpactMetric::High];

    pub fn code(self) -> &'static str {
        match self {
            ImpactMetric::None => "N",
            ImpactMetric::Low => "L",
            ImpactMetric::High => "H",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ImpactMetric::None => "None (N)",
            ImpactMetric::Low => "Low (L)",
            ImpactMetric::High => "High (H)",
        }
    }

    pub fn weight(self) -> f64 {
        match self {
            ImpactMetric::None => 0.0,
            ImpactMetric::Low => 0.22,
            ImpactMetric::High => 0.56,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn from_score(score: f64) -> Self {
        if score == 0.0 {
            Severity::None
        } else if score <= 3.9 {
            Severity::Low
        } else if score <= 6.9 {
            Severity::Medium
        } else if score <= 8.9 {
            Severity::High
        } else {
            Severity::Critical
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Severity::None => "None",
            Severity::Low => "Low",
            Severity::Medium => "Medium",
            Severity::High => "High",
            Severity::Critical => "Critical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CvssScore {
    pub base_score: f64,
    pub severity: Severity,
    /// Only CVSS 3.x defines sub-scores.
    pub sub_scores: Option<SubScores>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubScores {
    pub impact: f64,
    pub exploitability: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sampled from Red Hat's `cvss` Python library; the full 3.1 and 4.0 spaces were checked against it once.
    const REFERENCE: &str = include_str!("reference_vectors.txt");

    #[test]
    fn matches_reference_scores() {
        for line in REFERENCE.lines() {
            let (vector, expected) = line.split_once(' ').unwrap();
            let expected: f64 = expected.parse().unwrap();
            let score = if vector.starts_with("CVSS:4.0/") {
                CvssV4Base::from_vector(vector).unwrap().calculate()
            } else {
                CvssV3Base::from_vector(vector).unwrap().calculate()
            };
            assert_eq!(score.base_score, expected, "{vector}");
        }
    }
}
