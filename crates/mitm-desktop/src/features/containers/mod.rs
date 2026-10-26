mod commands;
mod view;

use mitm_core::proxy::ContainerRow;

pub use commands::{handle, poll, Cmd};
pub use view::show;

const REFRESH_EVERY: f64 = 0.5;

pub struct ContainersState {
    pub rows: Vec<ContainerRow>,
    /// Set by the tab while it draws, so rows are only recomputed when someone can see them.
    visible: bool,
    dirty: bool,
    refreshed_at: f64,
}

impl Default for ContainersState {
    fn default() -> Self {
        Self { rows: Vec::new(), visible: false, dirty: true, refreshed_at: 0.0 }
    }
}

impl ContainersState {
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn due(&mut self, now: f64) -> bool {
        let due = std::mem::take(&mut self.visible) && self.dirty && now - self.refreshed_at >= REFRESH_EVERY;
        if due {
            self.dirty = false;
            self.refreshed_at = now;
        }
        due
    }

    fn unregistered(&self) -> impl Iterator<Item = &ContainerRow> {
        self.rows.iter().filter(|row| !row.registered)
    }
}
