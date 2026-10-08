use serde::{Deserialize, Serialize};
use super::{AttackComplexity, AttackVector, CvssScore, ImpactMetric, PrivilegesRequired, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackRequirements {
    None,
    Present,
}

impl AttackRequirements {
    pub const ALL: [AttackRequirements; 2] = [AttackRequirements::None, AttackRequirements::Present];

    pub fn code(self) -> &'static str {
        match self {
            AttackRequirements::None => "N",
            AttackRequirements::Present => "P",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AttackRequirements::None => "None (N)",
            AttackRequirements::Present => "Present (P)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserInteractionV4 {
    None,
    Passive,
    Active,
}

impl UserInteractionV4 {
    pub const ALL: [UserInteractionV4; 3] = [
        UserInteractionV4::None,
        UserInteractionV4::Passive,
        UserInteractionV4::Active,
    ];

    pub fn code(self) -> &'static str {
        match self {
            UserInteractionV4::None => "N",
            UserInteractionV4::Passive => "P",
            UserInteractionV4::Active => "A",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            UserInteractionV4::None => "None (N)",
            UserInteractionV4::Passive => "Passive (P)",
            UserInteractionV4::Active => "Active (A)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CvssV4Base {
    pub av: AttackVector,
    pub ac: AttackComplexity,
    pub at: AttackRequirements,
    pub pr: PrivilegesRequired,
    pub ui: UserInteractionV4,
    pub vc: ImpactMetric,
    pub vi: ImpactMetric,
    pub va: ImpactMetric,
    pub sc: ImpactMetric,
    pub si: ImpactMetric,
    pub sa: ImpactMetric,
}

impl Default for CvssV4Base {
    fn default() -> Self {
        Self {
            av: AttackVector::Network,
            ac: AttackComplexity::Low,
            at: AttackRequirements::None,
            pr: PrivilegesRequired::None,
            ui: UserInteractionV4::None,
            vc: ImpactMetric::None,
            vi: ImpactMetric::None,
            va: ImpactMetric::None,
            sc: ImpactMetric::None,
            si: ImpactMetric::None,
            sa: ImpactMetric::None,
        }
    }
}

const LOOKUP: [f64; 324] = [
    10.0,     9.9,     9.8,     9.5,     9.5,     9.2,     10.0,     9.6,     9.3,     8.7,     9.1,     8.1,
    9.3,     9.0,     8.9,     8.0,     8.1,     6.8,     9.8,     9.5,     9.5,     9.2,     9.0,     8.4,
    9.3,     9.2,     8.9,     8.1,     8.1,     6.5,     8.8,     8.0,     7.8,     7.0,     6.9,     4.8,
    f64::NAN,     9.2,     f64::NAN,     8.2,     f64::NAN,     7.2,     f64::NAN,     7.9,     f64::NAN,     6.9,     f64::NAN,     5.0,
    f64::NAN,     6.9,     f64::NAN,     5.5,     f64::NAN,     2.7,     9.9,     9.7,     9.5,     9.2,     9.2,     8.5,
    9.5,     9.1,     9.0,     8.3,     8.4,     7.1,     9.2,     8.1,     8.2,     7.1,     7.2,     5.3,
    9.5,     9.3,     9.2,     8.5,     8.5,     7.3,     9.2,     8.2,     8.0,     7.2,     7.0,     5.9,
    8.4,     7.0,     7.1,     5.2,     5.0,     3.0,     f64::NAN,     8.6,     f64::NAN,     7.5,     f64::NAN,     5.2,
    f64::NAN,     7.1,     f64::NAN,     5.2,     f64::NAN,     2.9,     f64::NAN,     6.3,     f64::NAN,     2.9,     f64::NAN,     1.7,
    9.8,     9.5,     9.4,     8.7,     9.1,     8.1,     9.4,     8.9,     8.6,     7.4,     7.7,     6.4,
    8.7,     7.5,     7.4,     6.3,     6.3,     4.9,     9.4,     8.9,     8.8,     7.7,     7.6,     6.7,
    8.6,     7.6,     7.4,     5.8,     5.9,     5.0,     7.2,     5.7,     5.7,     5.2,     5.2,     2.5,
    f64::NAN,     8.3,     f64::NAN,     7.0,     f64::NAN,     5.4,     f64::NAN,     6.5,     f64::NAN,     5.8,     f64::NAN,     2.6,
    f64::NAN,     5.3,     f64::NAN,     2.1,     f64::NAN,     1.3,     9.5,     9.0,     8.8,     7.6,     7.6,     7.0,
    9.0,     7.7,     7.5,     6.2,     6.1,     5.3,     7.7,     6.6,     6.8,     5.9,     5.2,     3.0,
    8.9,     7.8,     7.6,     6.7,     6.2,     5.8,     7.4,     5.9,     5.7,     5.7,     4.7,     2.3,
    6.1,     5.2,     5.7,     2.9,     2.4,     1.6,     f64::NAN,     7.1,     f64::NAN,     5.9,     f64::NAN,     3.0,
    f64::NAN,     5.8,     f64::NAN,     2.6,     f64::NAN,     1.5,     f64::NAN,     2.3,     f64::NAN,     1.3,     f64::NAN,     0.6,
    9.3,     8.7,     8.6,     7.2,     7.5,     5.8,     8.6,     7.4,     7.4,     6.1,     5.6,     3.4,
    7.0,     5.4,     5.2,     4.0,     4.0,     2.2,     8.5,     7.5,     7.4,     5.5,     6.2,     5.1,
    7.2,     5.7,     5.5,     4.1,     4.6,     1.9,     5.3,     3.6,     3.4,     1.9,     1.9,     0.8,
    f64::NAN,     6.4,     f64::NAN,     5.1,     f64::NAN,     2.0,     f64::NAN,     4.7,     f64::NAN,     2.1,     f64::NAN,     1.1,
    f64::NAN,     2.4,     f64::NAN,     0.9,     f64::NAN,     0.4,     8.8,     7.5,     7.3,     5.3,     6.0,     5.0,
    7.3,     5.5,     5.9,     4.0,     4.1,     2.0,     5.4,     4.3,     4.5,     2.2,     2.0,     1.1,
    7.5,     5.5,     5.8,     4.5,     4.0,     2.1,     6.1,     5.1,     4.8,     1.8,     2.0,     0.9,
    4.6,     1.8,     1.7,     0.7,     0.8,     0.2,     f64::NAN,     5.3,     f64::NAN,     2.4,     f64::NAN,     1.4,
    f64::NAN,     2.4,     f64::NAN,     1.2,     f64::NAN,     0.5,     f64::NAN,     1.0,     f64::NAN,     0.3,     f64::NAN,     0.1,
];


fn lookup_score(eq1: usize, eq2: usize, eq3: usize, eq4: usize, eq5: usize, eq6: usize) -> f64 {
    if eq1 > 2 || eq2 > 1 || eq3 > 2 || eq4 > 2 || eq5 > 2 || eq6 > 1 {
        return f64::NAN;
    }
    let idx = eq1 * 108 + eq2 * 54 + eq3 * 18 + eq4 * 6 + eq5 * 2 + eq6;
    LOOKUP[idx]
}

fn m_eq1(av: AttackVector, pr: PrivilegesRequired, ui: UserInteractionV4) -> usize {
    let n_av = av == AttackVector::Network;
    let n_pr = pr == PrivilegesRequired::None;
    let n_ui = ui == UserInteractionV4::None;
    if n_av && n_pr && n_ui {
        0
    } else if (n_av || n_pr || n_ui) && !(n_av && n_pr && n_ui) && av != AttackVector::Physical {
        1
    } else {
        2
    }
}

fn m_eq2(ac: AttackComplexity, at: AttackRequirements) -> usize {
    if ac == AttackComplexity::Low && at == AttackRequirements::None {
        0
    } else {
        1
    }
}

fn m_eq3(vc: ImpactMetric, vi: ImpactMetric, va: ImpactMetric) -> usize {
    let h_vc = vc == ImpactMetric::High;
    let h_vi = vi == ImpactMetric::High;
    let h_va = va == ImpactMetric::High;
    if h_vc && h_vi {
        0
    } else if !(h_vc && h_vi) && (h_vc || h_vi || h_va) {
        1
    } else {
        2
    }
}

fn m_eq4(sc: ImpactMetric, si: ImpactMetric, sa: ImpactMetric) -> usize {
    if sc == ImpactMetric::High || si == ImpactMetric::High || sa == ImpactMetric::High {
        1
    } else {
        2
    }
}

fn m_eq6(vc: ImpactMetric, vi: ImpactMetric, va: ImpactMetric) -> usize {
    if vc == ImpactMetric::High || vi == ImpactMetric::High || va == ImpactMetric::High {
        0
    } else {
        1
    }
}

fn level_av(code: u8) -> f64 {
    match code {
        b'N' => 0.0,
        b'A' => 0.1,
        b'L' => 0.2,
        b'P' => 0.3,
        _ => 0.0,
    }
}

fn level_pr(code: u8) -> f64 {
    match code {
        b'N' => 0.0,
        b'L' => 0.1,
        b'H' => 0.2,
        _ => 0.0,
    }
}

fn level_ui(code: u8) -> f64 {
    match code {
        b'N' => 0.0,
        b'P' => 0.1,
        b'A' => 0.2,
        _ => 0.0,
    }
}

fn level_ac(code: u8) -> f64 {
    match code {
        b'L' => 0.0,
        b'H' => 0.1,
        _ => 0.0,
    }
}

fn level_at(code: u8) -> f64 {
    match code {
        b'N' => 0.0,
        b'P' => 0.1,
        _ => 0.0,
    }
}

fn level_v(code: u8) -> f64 {
    match code {
        b'H' => 0.0,
        b'L' => 0.1,
        b'N' => 0.2,
        _ => 0.0,
    }
}

fn level_s(code: u8) -> f64 {
    match code {
        b'H' => 0.1,
        b'L' => 0.2,
        b'N' => 0.3,
        _ => 0.0,
    }
}

const EQ1_MAXES: [&[&[u8]]; 3] = [
    &[b"AV:N/PR:N/UI:N/"],
    &[b"AV:A/PR:N/UI:N/", b"AV:N/PR:L/UI:N/", b"AV:N/PR:N/UI:P/"],
    &[b"AV:P/PR:N/UI:N/", b"AV:A/PR:L/UI:P/"],
];

const EQ2_MAXES: [&[&[u8]]; 2] = [
    &[b"AC:L/AT:N/"],
    &[b"AC:H/AT:N/", b"AC:L/AT:P/"],
];

const EQ3_EQ6_0_0: &[&[u8]] = &[b"VC:H/VI:H/VA:H/CR:H/IR:H/AR:H/"];
const EQ3_EQ6_0_1: &[&[u8]] = &[b"VC:H/VI:H/VA:L/CR:M/IR:M/AR:H/", b"VC:H/VI:H/VA:H/CR:M/IR:M/AR:M/"];
const EQ3_EQ6_1_0: &[&[u8]] = &[b"VC:L/VI:H/VA:H/CR:H/IR:H/AR:H/", b"VC:H/VI:L/VA:H/CR:H/IR:H/AR:H/"];
const EQ3_EQ6_1_1: &[&[u8]] = &[
    b"VC:L/VI:H/VA:L/CR:H/IR:M/AR:H/",
    b"VC:L/VI:H/VA:H/CR:H/IR:M/AR:M/",
    b"VC:H/VI:L/VA:H/CR:M/IR:H/AR:M/",
    b"VC:H/VI:L/VA:L/CR:M/IR:H/AR:H/",
    b"VC:L/VI:L/VA:H/CR:H/IR:H/AR:M/",
];
const EQ3_EQ6_2_1: &[&[u8]] = &[b"VC:L/VI:L/VA:L/CR:H/IR:H/AR:H/"];

const EQ4_MAXES: [&[&[u8]]; 3] = [
    &[b"SC:H/SI:S/SA:S/"],
    &[b"SC:H/SI:H/SA:H/"],
    &[b"SC:L/SI:L/SA:L/"],
];

const MAX_SEVERITY_EQ1: [f64; 3] = [0.1, 0.4, 0.5];
const MAX_SEVERITY_EQ2: [f64; 2] = [0.1, 0.2];
const MAX_SEVERITY_EQ3EQ6: [[f64; 2]; 3] = [
    [0.7, 0.6],
    [0.8, 0.8],
    [1.0, 1.0],
];
const MAX_SEVERITY_EQ4: [f64; 3] = [0.6, 0.5, 0.4];

fn final_rounding(x: f64) -> f64 {
    let epsilon = 1e-6;
    let rounded = ((x + epsilon) * 10.0).round() / 10.0;
    (rounded * 10.0).round() / 10.0
}

impl CvssV4Base {
    pub fn calculate(&self) -> CvssScore {
        if self.vc == ImpactMetric::None
            && self.vi == ImpactMetric::None
            && self.va == ImpactMetric::None
            && self.sc == ImpactMetric::None
            && self.si == ImpactMetric::None
            && self.sa == ImpactMetric::None
        {
            return CvssScore {
                base_score: 0.0,
                severity: Severity::None,
                sub_scores: None,
            };
        }

        let eq1 = m_eq1(self.av, self.pr, self.ui);
        let eq2 = m_eq2(self.ac, self.at);
        let eq3 = m_eq3(self.vc, self.vi, self.va);
        let eq4 = m_eq4(self.sc, self.si, self.sa);
        let eq5 = 0;
        let eq6 = m_eq6(self.vc, self.vi, self.va);

        let value = lookup_score(eq1, eq2, eq3, eq4, eq5, eq6);

        let score_eq1 = lookup_score(eq1 + 1, eq2, eq3, eq4, eq5, eq6);
        let score_eq2 = lookup_score(eq1, eq2 + 1, eq3, eq4, eq5, eq6);

        let score_eq3eq6 = match (eq3, eq6) {
            (0 | 1, 1) => lookup_score(eq1, eq2, eq3 + 1, eq4, eq5, eq6),
            (1, 0) => lookup_score(eq1, eq2, eq3, eq4, eq5, eq6 + 1),
            (0, 0) => {
                let s_left = lookup_score(eq1, eq2, eq3, eq4, eq5, eq6 + 1);
                let s_right = lookup_score(eq1, eq2, eq3 + 1, eq4, eq5, eq6);
                if s_left.is_nan() {
                    s_right
                } else if s_right.is_nan() {
                    s_left
                } else {
                    s_left.max(s_right)
                }
            }
            _ => lookup_score(eq1, eq2, eq3 + 1, eq4, eq5, eq6 + 1),
        };

        let score_eq4 = lookup_score(eq1, eq2, eq3, eq4 + 1, eq5, eq6);
        let score_eq5 = lookup_score(eq1, eq2, eq3, eq4, eq5 + 1, eq6);

        let eq1_maxes = EQ1_MAXES[eq1];
        let eq2_maxes = EQ2_MAXES[eq2];
        let eq3_eq6_maxes = match (eq3, eq6) {
            (0, 0) => EQ3_EQ6_0_0,
            (0, 1) => EQ3_EQ6_0_1,
            (1, 0) => EQ3_EQ6_1_0,
            (1, 1) => EQ3_EQ6_1_1,
            _ => EQ3_EQ6_2_1,
        };
        let eq4_maxes = EQ4_MAXES[eq4];

        let mut d_av = 0.0;
        let mut d_pr = 0.0;
        let mut d_ui = 0.0;
        let mut d_ac = 0.0;
        let mut d_at = 0.0;
        let mut d_vc = 0.0;
        let mut d_vi = 0.0;
        let mut d_va = 0.0;
        let mut d_sc = 0.0;
        let mut d_si = 0.0;
        let mut d_sa = 0.0;

        let my_av = level_av(self.av.code().as_bytes()[0]);
        let my_pr = level_pr(self.pr.code().as_bytes()[0]);
        let my_ui = level_ui(self.ui.code().as_bytes()[0]);
        let my_ac = level_ac(self.ac.code().as_bytes()[0]);
        let my_at = level_at(self.at.code().as_bytes()[0]);
        let my_vc = level_v(self.vc.code().as_bytes()[0]);
        let my_vi = level_v(self.vi.code().as_bytes()[0]);
        let my_va = level_v(self.va.code().as_bytes()[0]);
        let my_sc = level_s(self.sc.code().as_bytes()[0]);
        let my_si = level_s(self.si.code().as_bytes()[0]);
        let my_sa = level_s(self.sa.code().as_bytes()[0]);

        let mut found = false;
        for eq1_m in eq1_maxes {
            let dav = my_av - level_av(eq1_m[3]);
            let dpr = my_pr - level_pr(eq1_m[8]);
            let dui = my_ui - level_ui(eq1_m[13]);
            if dav < 0.0 || dpr < 0.0 || dui < 0.0 {
                continue;
            }
            for eq2_m in eq2_maxes {
                let dac = my_ac - level_ac(eq2_m[3]);
                let dat = my_at - level_at(eq2_m[8]);
                if dac < 0.0 || dat < 0.0 {
                    continue;
                }
                for eq3_m in eq3_eq6_maxes {
                    let dvc = my_vc - level_v(eq3_m[3]);
                    let dvi = my_vi - level_v(eq3_m[8]);
                    let dva = my_va - level_v(eq3_m[13]);
                    if dvc < 0.0 || dvi < 0.0 || dva < 0.0 {
                        continue;
                    }
                    for eq4_m in eq4_maxes {
                        let dsc = my_sc - level_s(eq4_m[3]);
                        let dsi = my_si - level_s(eq4_m[8]);
                        let dsa = my_sa - level_s(eq4_m[13]);
                        if dsc < 0.0 || dsi < 0.0 || dsa < 0.0 {
                            continue;
                        }
                        d_av = dav;
                        d_pr = dpr;
                        d_ui = dui;
                        d_ac = dac;
                        d_at = dat;
                        d_vc = dvc;
                        d_vi = dvi;
                        d_va = dva;
                        d_sc = dsc;
                        d_si = dsi;
                        d_sa = dsa;
                        found = true;
                        break;
                    }
                    if found {
                        break;
                    }
                }
                if found {
                    break;
                }
            }
            if found {
                break;
            }
        }

        let cur_d_eq1 = d_av + d_pr + d_ui;
        let cur_d_eq2 = d_ac + d_at;
        let cur_d_eq3eq6 = d_vc + d_vi + d_va;
        let cur_d_eq4 = d_sc + d_si + d_sa;

        let avail_d_eq1 = value - score_eq1;
        let avail_d_eq2 = value - score_eq2;
        let avail_d_eq3eq6 = value - score_eq3eq6;
        let avail_d_eq4 = value - score_eq4;
        let avail_d_eq5 = value - score_eq5;

        let max_s_eq1 = MAX_SEVERITY_EQ1[eq1];
        let max_s_eq2 = MAX_SEVERITY_EQ2[eq2];
        let max_s_eq3eq6 = MAX_SEVERITY_EQ3EQ6[eq3][eq6];
        let max_s_eq4 = MAX_SEVERITY_EQ4[eq4];

        let mut norm_eq1 = 0.0;
        let mut norm_eq2 = 0.0;
        let mut norm_eq3eq6 = 0.0;
        let mut norm_eq4 = 0.0;
        let norm_eq5 = 0.0;
        let mut n = 0.0;

        if !avail_d_eq1.is_nan() && avail_d_eq1 >= 0.0 {
            n += 1.0;
            norm_eq1 = avail_d_eq1 * (cur_d_eq1 / max_s_eq1);
        }
        if !avail_d_eq2.is_nan() && avail_d_eq2 >= 0.0 {
            n += 1.0;
            norm_eq2 = avail_d_eq2 * (cur_d_eq2 / max_s_eq2);
        }
        if !avail_d_eq3eq6.is_nan() && avail_d_eq3eq6 >= 0.0 {
            n += 1.0;
            norm_eq3eq6 = avail_d_eq3eq6 * (cur_d_eq3eq6 / max_s_eq3eq6);
        }
        if !avail_d_eq4.is_nan() && avail_d_eq4 >= 0.0 {
            n += 1.0;
            norm_eq4 = avail_d_eq4 * (cur_d_eq4 / max_s_eq4);
        }
        if !avail_d_eq5.is_nan() && avail_d_eq5 >= 0.0 {
            n += 1.0;
        }

        let mean_d = if n == 0.0 {
            0.0
        } else {
            (norm_eq1 + norm_eq2 + norm_eq3eq6 + norm_eq4 + norm_eq5) / n
        };

        let mut final_val = value - mean_d;
        final_val = final_val.clamp(0.0, 10.0);
        let base_score = final_rounding(final_val);

        CvssScore {
            base_score,
            severity: Severity::from_score(base_score),
            sub_scores: None,
        }
    }

    pub fn to_vector(&self) -> String {
        format!(
            "CVSS:4.0/AV:{}/AC:{}/AT:{}/PR:{}/UI:{}/VC:{}/VI:{}/VA:{}/SC:{}/SI:{}/SA:{}",
            self.av.code(),
            self.ac.code(),
            self.at.code(),
            self.pr.code(),
            self.ui.code(),
            self.vc.code(),
            self.vi.code(),
            self.va.code(),
            self.sc.code(),
            self.si.code(),
            self.sa.code()
        )
    }

    pub fn from_vector(raw: &str) -> Result<Self, String> {
        let s = raw.trim();
        let body = if let Some(rest) = s.strip_prefix("CVSS:4.0/") {
            rest
        } else {
            s
        };

        let mut av = None;
        let mut ac = None;
        let mut at = None;
        let mut pr = None;
        let mut ui = None;
        let mut vc = None;
        let mut vi = None;
        let mut va = None;
        let mut sc = None;
        let mut si = None;
        let mut sa = None;

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
                "AT" => at = Some(match v {
                    "N" => AttackRequirements::None,
                    "P" => AttackRequirements::Present,
                    _ => return Err(format!("Unknown AT: {v}")),
                }),
                "PR" => pr = Some(match v {
                    "N" => PrivilegesRequired::None,
                    "L" => PrivilegesRequired::Low,
                    "H" => PrivilegesRequired::High,
                    _ => return Err(format!("Unknown PR: {v}")),
                }),
                "UI" => ui = Some(match v {
                    "N" => UserInteractionV4::None,
                    "P" => UserInteractionV4::Passive,
                    "A" => UserInteractionV4::Active,
                    _ => return Err(format!("Unknown UI: {v}")),
                }),
                "VC" => vc = Some(match v {
                    "N" => ImpactMetric::None,
                    "L" => ImpactMetric::Low,
                    "H" => ImpactMetric::High,
                    _ => return Err(format!("Unknown VC: {v}")),
                }),
                "VI" => vi = Some(match v {
                    "N" => ImpactMetric::None,
                    "L" => ImpactMetric::Low,
                    "H" => ImpactMetric::High,
                    _ => return Err(format!("Unknown VI: {v}")),
                }),
                "VA" => va = Some(match v {
                    "N" => ImpactMetric::None,
                    "L" => ImpactMetric::Low,
                    "H" => ImpactMetric::High,
                    _ => return Err(format!("Unknown VA: {v}")),
                }),
                "SC" => sc = Some(match v {
                    "N" => ImpactMetric::None,
                    "L" => ImpactMetric::Low,
                    "H" => ImpactMetric::High,
                    _ => return Err(format!("Unknown SC: {v}")),
                }),
                "SI" => si = Some(match v {
                    "N" => ImpactMetric::None,
                    "L" => ImpactMetric::Low,
                    "H" => ImpactMetric::High,
                    _ => return Err(format!("Unknown SI: {v}")),
                }),
                "SA" => sa = Some(match v {
                    "N" => ImpactMetric::None,
                    "L" => ImpactMetric::Low,
                    "H" => ImpactMetric::High,
                    _ => return Err(format!("Unknown SA: {v}")),
                }),
                _ => {}
            }
        }

        Ok(Self {
            av: av.ok_or("Missing AV metric")?,
            ac: ac.ok_or("Missing AC metric")?,
            at: at.ok_or("Missing AT metric")?,
            pr: pr.ok_or("Missing PR metric")?,
            ui: ui.ok_or("Missing UI metric")?,
            vc: vc.ok_or("Missing VC metric")?,
            vi: vi.ok_or("Missing VI metric")?,
            va: va.ok_or("Missing VA metric")?,
            sc: sc.ok_or("Missing SC metric")?,
            si: si.ok_or("Missing SI metric")?,
            sa: sa.ok_or("Missing SA metric")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_known_v4_vectors() {
        // High vuln
        let v1 = CvssV4Base::from_vector("CVSS:4.0/AV:N/AC:L/AT:N/PR:N/UI:N/VC:H/VI:H/VA:H/SC:N/SI:N/SA:N").unwrap();
        assert_eq!(v1.calculate().base_score, 9.3);
        assert_eq!(v1.calculate().severity, Severity::Critical);

        // All High
        let v2 = CvssV4Base::from_vector("CVSS:4.0/AV:N/AC:L/AT:N/PR:N/UI:N/VC:H/VI:H/VA:H/SC:H/SI:H/SA:H").unwrap();
        assert_eq!(v2.calculate().base_score, 10.0);

        // All None
        let v3 = CvssV4Base::default();
        assert_eq!(v3.calculate().base_score, 0.0);

        // Local Critical
        let v4 = CvssV4Base::from_vector("CVSS:4.0/AV:L/AC:L/AT:N/PR:H/UI:N/VC:H/VI:H/VA:H/SC:H/SI:H/SA:H").unwrap();
        assert_eq!(v4.calculate().base_score, 9.3);

        // Low severity
        let v5 = CvssV4Base::from_vector("CVSS:4.0/AV:A/AC:H/AT:P/PR:L/UI:P/VC:L/VI:L/VA:L/SC:L/SI:L/SA:L").unwrap();
        assert_eq!(v5.calculate().base_score, 1.0);
        assert_eq!(v5.calculate().severity, Severity::Low);
    }
}
