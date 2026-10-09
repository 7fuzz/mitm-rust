mod view;

use mitm_core::utils::cvss::{CvssScore, CvssV3Base, CvssV4Base, CvssVersion};
use serde::{Deserialize, Serialize};

pub use view::show;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct CvssPrefs {
    pub version: CvssVersion,
    pub v3: CvssV3Base,
    pub v4: CvssV4Base,
}

impl Default for CvssPrefs {
    fn default() -> Self {
        Self { version: CvssVersion::V3_1, v3: CvssV3Base::default(), v4: CvssV4Base::default() }
    }
}

#[derive(Default)]
pub struct CvssState {
    pub prefs: CvssPrefs,
    /// Holds the vector field's text only while it doesn't parse; otherwise the field shows `vector()`.
    draft: Option<String>,
    error: Option<String>,
}

impl CvssState {
    pub fn score(&self) -> CvssScore {
        match self.prefs.version {
            CvssVersion::V3_1 => self.prefs.v3.calculate(),
            CvssVersion::V4_0 => self.prefs.v4.calculate(),
        }
    }

    pub fn vector(&self) -> String {
        match self.prefs.version {
            CvssVersion::V3_1 => self.prefs.v3.to_vector(),
            CvssVersion::V4_0 => self.prefs.v4.to_vector(),
        }
    }

    pub fn set_version(&mut self, version: CvssVersion) {
        self.prefs.version = version;
        self.discard_draft();
    }

    pub(super) fn discard_draft(&mut self) {
        self.draft = None;
        self.error = None;
    }

    pub fn edit_vector(&mut self, text: String) {
        let trimmed = text.trim();
        let version = if trimmed.starts_with("CVSS:4.0/") {
            CvssVersion::V4_0
        } else if trimmed.starts_with("CVSS:3.1/") || trimmed.starts_with("CVSS:3.0/") {
            CvssVersion::V3_1
        } else {
            self.prefs.version
        };
        let parsed = match version {
            CvssVersion::V3_1 => CvssV3Base::from_vector(trimmed).map(|b| self.prefs.v3 = b),
            CvssVersion::V4_0 => CvssV4Base::from_vector(trimmed).map(|b| self.prefs.v4 = b),
        };
        match parsed {
            Ok(()) => {
                self.prefs.version = version;
                self.discard_draft();
            }
            Err(e) => {
                self.draft = Some(text);
                self.error = Some(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_edits_drive_version_and_metrics() {
        let mut state = CvssState::default();
        state.edit_vector("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H".into());
        assert_eq!(state.prefs.version, CvssVersion::V3_1);
        assert_eq!(state.score().base_score, 9.8);

        state.edit_vector("CVSS:4.0/AV:N/AC:L/AT:N/PR:N/UI:N/VC:H/VI:H/VA:H/SC:N/SI:N/SA:N".into());
        assert_eq!(state.prefs.version, CvssVersion::V4_0);
        assert_eq!(state.score().base_score, 9.3);

        state.edit_vector("CVSS:4.0/AV:N".into());
        assert!(state.error.is_some());
        assert_eq!(state.draft.as_deref(), Some("CVSS:4.0/AV:N"));
        assert_eq!(state.score().base_score, 9.3);

        state.set_version(CvssVersion::V3_1);
        assert!(state.draft.is_none());
        assert_eq!(state.vector(), "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H");
    }
}
