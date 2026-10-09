mod view;

use mitm_core::utils::json::Indent;
use serde::{Deserialize, Serialize};

pub use view::show;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Mode {
    Format,
    Minify,
    Escape,
    Unescape,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum IndentOpt {
    Two,
    Four,
    Tab,
}

impl IndentOpt {
    pub const ALL: [IndentOpt; 3] = [IndentOpt::Two, IndentOpt::Four, IndentOpt::Tab];

    pub fn label(&self) -> &'static str {
        match self {
            IndentOpt::Two => "2 spaces",
            IndentOpt::Four => "4 spaces",
            IndentOpt::Tab => "Tab",
        }
    }

    fn to_core(self) -> Indent {
        match self {
            IndentOpt::Two => Indent::Two,
            IndentOpt::Four => Indent::Four,
            IndentOpt::Tab => Indent::Tab,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct JsonToolsPrefs {
    pub split: f32,
    pub mode: Mode,
    pub indent: IndentOpt,
    pub sort_keys: bool,
}

impl Default for JsonToolsPrefs {
    fn default() -> Self {
        Self { split: 0.5, mode: Mode::Format, indent: IndentOpt::Two, sort_keys: false }
    }
}

#[derive(Default)]
pub struct JsonToolsState {
    pub prefs: JsonToolsPrefs,
    pub input: String,
    cache: Option<(u64, Result<String, String>)>,
}

impl JsonToolsState {
    pub(crate) fn output(&mut self) -> &Result<String, String> {
        let key = self.key();
        if self.cache.as_ref().map(|(k, _)| *k) != Some(key) {
            self.cache = Some((key, compute(&self.prefs, &self.input)));
        }
        &self.cache.as_ref().unwrap().1
    }

    fn key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.input.hash(&mut h);
        (self.prefs.mode as u8).hash(&mut h);
        (self.prefs.indent as u8).hash(&mut h);
        self.prefs.sort_keys.hash(&mut h);
        h.finish()
    }
}

fn compute(prefs: &JsonToolsPrefs, input: &str) -> Result<String, String> {
    use mitm_core::utils::json;
    if input.trim().is_empty() {
        return Ok(String::new());
    }
    match prefs.mode {
        Mode::Format => json::format(input, prefs.indent.to_core(), prefs.sort_keys),
        Mode::Minify => json::minify(input),
        Mode::Escape => Ok(json::escape(input)),
        Mode::Unescape => json::unescape(input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_mode_and_parse_errors() {
        let mut state = JsonToolsState {
            input: "{\"b\":1,\"a\":2}".to_string(),
            ..JsonToolsState::default()
        };
        state.prefs.mode = Mode::Format;
        state.prefs.sort_keys = true;
        assert_eq!(state.output().as_ref().unwrap(), "{\n  \"a\": 2,\n  \"b\": 1\n}");

        state.prefs.mode = Mode::Minify;
        assert_eq!(state.output().as_ref().unwrap(), "{\"b\":1,\"a\":2}");

        state.prefs.mode = Mode::Escape;
        assert_eq!(state.output().as_ref().unwrap(), "\"{\\\"b\\\":1,\\\"a\\\":2}\"");

        state.input = state.output().clone().unwrap();
        state.prefs.mode = Mode::Unescape;
        assert_eq!(state.output().as_ref().unwrap(), "{\"b\":1,\"a\":2}");

        state.prefs.mode = Mode::Format;
        state.input = "{invalid}".to_string();
        let err = state.output().as_ref().unwrap_err();
        assert!(err.contains("line 1, column"));
    }
}
