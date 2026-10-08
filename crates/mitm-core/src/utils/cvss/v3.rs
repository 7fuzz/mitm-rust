use serde::{Deserialize, Serialize};
use super::{AttackComplexity, AttackVector, CvssScore, ImpactMetric, PrivilegesRequired, Severity, SubScores};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserInteraction {
    None,
    Required,
}

impl UserInteraction {
    pub const ALL: [UserInteraction; 2] = [UserInteraction::None, UserInteraction::Required];

    pub fn code(self) -> &'static str {
        match self {
            UserInteraction::None => "N",
            UserInteraction::Required => "R",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            UserInteraction::None => "None (N)",
            UserInteraction::Required => "Required (R)",
        }
    }

    fn weight(self) -> f64 {
        match self {
            UserInteraction::None => 0.85,
            UserInteraction::Required => 0.62,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    Unchanged,
    Changed,
}

impl Scope {
    pub const ALL: [Scope; 2] = [Scope::Unchanged, Scope::Changed];

    pub fn code(self) -> &'static str {
        match self {
            Scope::Unchanged => "U",
            Scope::Changed => "C",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Scope::Unchanged => "Unchanged (U)",
            Scope::Changed => "Changed (C)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CvssV3Base {
    pub av: AttackVector,
    pub ac: AttackComplexity,
    pub pr: PrivilegesRequired,
    pub ui: UserInteraction,
    pub s: Scope,
    pub c: ImpactMetric,
    pub i: ImpactMetric,
    pub a: ImpactMetric,
}

impl Default for CvssV3Base {
    fn default() -> Self {
        Self {
            av: AttackVector::Network,
            ac: AttackComplexity::Low,
            pr: PrivilegesRequired::None,
            ui: UserInteraction::None,
            s: Scope::Unchanged,
            c: ImpactMetric::None,
            i: ImpactMetric::None,
            a: ImpactMetric::None,
        }
    }
}

fn pr_weight(pr: PrivilegesRequired, s: Scope) -> f64 {
    match (pr, s) {
        (PrivilegesRequired::None, _) => 0.85,
        (PrivilegesRequired::Low, Scope::Unchanged) => 0.62,
        (PrivilegesRequired::Low, Scope::Changed) => 0.68,
        (PrivilegesRequired::High, Scope::Unchanged) => 0.27,
        (PrivilegesRequired::High, Scope::Changed) => 0.50,
    }
}

fn roundup(input: f64) -> f64 {
    let int_input = (input * 100_000.0).round() as i64;
    if int_input % 10_000 == 0 {
        int_input as f64 / 100_000.0
    } else {
        ((int_input / 10_000) + 1) as f64 / 10.0
    }
}

impl CvssV3Base {
    pub fn calculate(&self) -> CvssScore {
        let iss = 1.0 - ((1.0 - self.c.weight()) * (1.0 - self.i.weight()) * (1.0 - self.a.weight()));
        let impact = match self.s {
            Scope::Unchanged => 6.42 * iss,
            Scope::Changed => 7.52 * (iss - 0.029) - 3.25 * (iss - 0.02).powf(15.0),
        };

        let exploitability = 8.22 * self.av.weight() * self.ac.weight() * pr_weight(self.pr, self.s) * self.ui.weight();

        let base_score = if impact <= 0.0 {
            0.0
        } else {
            match self.s {
                Scope::Unchanged => roundup((impact + exploitability).min(10.0)),
                Scope::Changed => roundup((1.08 * (impact + exploitability)).min(10.0)),
            }
        };

        CvssScore {
            base_score,
            severity: Severity::from_score(base_score),
            sub_scores: Some(SubScores {
                impact: (impact.max(0.0) * 10.0).round() / 10.0,
                exploitability: (exploitability * 10.0).round() / 10.0,
            }),
        }
    }

    pub fn to_vector(&self) -> String {
        format!(
            "CVSS:3.1/AV:{}/AC:{}/PR:{}/UI:{}/S:{}/C:{}/I:{}/A:{}",
            self.av.code(),
            self.ac.code(),
            self.pr.code(),
            self.ui.code(),
            self.s.code(),
            self.c.code(),
            self.i.code(),
            self.a.code()
        )
    }

    pub fn from_vector(raw: &str) -> Result<Self, String> {
        let s = raw.trim();
        let body = if let Some(rest) = s.strip_prefix("CVSS:3.1/") {
            rest
        } else if let Some(rest) = s.strip_prefix("CVSS:3.0/") {
            rest
        } else {
            s
        };

        let mut av = None;
        let mut ac = None;
        let mut pr = None;
        let mut ui = None;
        let mut sc = None;
        let mut c = None;
        let mut i = None;
        let mut a = None;

        for part in body.split('/') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (k, v) = part.split_once(':').ok_or_else(|| format!("Invalid metric: {part}"))?;
            match k {
                "AV" => av = Some(match v {
                    "N" => AttackVector::Network,
                    "A" => AttackVector::Adjacent,
                    "L" => AttackVector::Local,
                    "P" => AttackVector::Physical,
                    _ => return Err(format!("Unknown AV: {v}")),
                }),
                "AC" => ac = Some(match v {
                    "L" => AttackComplexity::Low,
                    "H" => AttackComplexity::High,
                    _ => return Err(format!("Unknown AC: {v}")),
                }),
                "PR" => pr = Some(match v {
                    "N" => PrivilegesRequired::None,
                    "L" => PrivilegesRequired::Low,
                    "H" => PrivilegesRequired::High,
                    _ => return Err(format!("Unknown PR: {v}")),
                }),
                "UI" => ui = Some(match v {
                    "N" => UserInteraction::None,
                    "R" => UserInteraction::Required,
                    _ => return Err(format!("Unknown UI: {v}")),
                }),
                "S" => sc = Some(match v {
                    "U" => Scope::Unchanged,
                    "C" => Scope::Changed,
                    _ => return Err(format!("Unknown S: {v}")),
                }),
                "C" => c = Some(match v {
                    "N" => ImpactMetric::None,
                    "L" => ImpactMetric::Low,
                    "H" => ImpactMetric::High,
                    _ => return Err(format!("Unknown C: {v}")),
                }),
                "I" => i = Some(match v {
                    "N" => ImpactMetric::None,
                    "L" => ImpactMetric::Low,
                    "H" => ImpactMetric::High,
                    _ => return Err(format!("Unknown I: {v}")),
                }),
                "A" => a = Some(match v {
                    "N" => ImpactMetric::None,
                    "L" => ImpactMetric::Low,
                    "H" => ImpactMetric::High,
                    _ => return Err(format!("Unknown A: {v}")),
                }),
                _ => return Err(format!("Unknown metric key: {k}")),
            }
        }

        Ok(Self {
            av: av.ok_or("Missing AV metric")?,
            ac: ac.ok_or("Missing AC metric")?,
            pr: pr.ok_or("Missing PR metric")?,
            ui: ui.ok_or("Missing UI metric")?,
            s: sc.ok_or("Missing S metric")?,
            c: c.ok_or("Missing C metric")?,
            i: i.ok_or("Missing I metric")?,
            a: a.ok_or("Missing A metric")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_known_v3_vectors() {
        let v1 = CvssV3Base::from_vector("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H").unwrap();
        let score1 = v1.calculate();
        assert_eq!(score1.base_score, 9.8);
        assert_eq!(score1.severity, Severity::Critical);
        assert_eq!(v1.to_vector(), "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H");

        let v2 = CvssV3Base::from_vector("CVSS:3.1/AV:N/AC:L/PR:L/UI:N/S:C/C:L/I:L/A:N").unwrap();
        let score2 = v2.calculate();
        assert_eq!(score2.base_score, 6.4);
        assert_eq!(score2.severity, Severity::Medium);

        let v3 = CvssV3Base::default();
        let score3 = v3.calculate();
        assert_eq!(score3.base_score, 0.0);
        assert_eq!(score3.severity, Severity::None);

        let v4 = CvssV3Base::from_vector("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:C/C:H/I:H/A:H").unwrap();
        assert_eq!(v4.calculate().base_score, 10.0);
    }
}
