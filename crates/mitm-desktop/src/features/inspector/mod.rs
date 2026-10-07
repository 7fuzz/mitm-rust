mod view;

pub use view::show;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Request,
    Response,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Headers,
    Formatted,
    Raw,
    Hex,
}

pub struct InspectorState {
    pub phase: Phase,
    pub mode: ViewMode,
    pub header_search: String,
}

impl Default for InspectorState {
    fn default() -> Self {
        Self {
            phase: Phase::Request,
            mode: ViewMode::Headers,
            header_search: String::new(),
        }
    }
}
