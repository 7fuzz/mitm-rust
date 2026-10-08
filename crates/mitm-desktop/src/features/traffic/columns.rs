use eframe::egui::Align;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColumnId {
    Id,
    Method,
    Source,
    Host,
    Path,
    Status,
    ContentType,
    Size,
    Duration,
    RequestAt,
    ResponseAt,
}

impl ColumnId {
    pub const ALL: [ColumnId; 11] = [
        ColumnId::Id,
        ColumnId::Method,
        ColumnId::Source,
        ColumnId::Host,
        ColumnId::Path,
        ColumnId::Status,
        ColumnId::ContentType,
        ColumnId::Size,
        ColumnId::Duration,
        ColumnId::RequestAt,
        ColumnId::ResponseAt,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ColumnId::Id => "#",
            ColumnId::Method => "Method",
            ColumnId::Source => "Source",
            ColumnId::Host => "Host",
            ColumnId::Path => "Path",
            ColumnId::Status => "Status",
            ColumnId::ContentType => "Content-Type",
            ColumnId::Size => "Size",
            ColumnId::Duration => "Duration",
            ColumnId::RequestAt => "Request Time",
            ColumnId::ResponseAt => "Response Time",
        }
    }

    pub fn menu_label(self) -> &'static str {
        match self {
            ColumnId::Id => "# (ID)",
            other => other.label(),
        }
    }

    pub fn default_width(self) -> f32 {
        match self {
            ColumnId::Id => 56.0,
            ColumnId::Method => 104.0,
            ColumnId::Source => 88.0,
            ColumnId::Host => 200.0,
            ColumnId::Path => 320.0,
            ColumnId::Status => 72.0,
            ColumnId::ContentType => 140.0,
            ColumnId::Size => 80.0,
            ColumnId::Duration => 80.0,
            ColumnId::RequestAt | ColumnId::ResponseAt => 104.0,
        }
    }

    pub fn align(self) -> Align {
        match self {
            ColumnId::Id | ColumnId::Status => Align::Center,
            ColumnId::Size | ColumnId::Duration | ColumnId::RequestAt | ColumnId::ResponseAt => Align::Max,
            _ => Align::Min,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ColumnLayout {
    pub order: Vec<ColumnId>,
    pub hidden: Vec<ColumnId>,
}

impl Default for ColumnLayout {
    fn default() -> Self {
        Self { order: ColumnId::ALL.to_vec(), hidden: Vec::new() }
    }
}

impl ColumnLayout {
    pub fn visible(&self) -> Vec<ColumnId> {
        self.order.iter().copied().filter(|c| !self.hidden.contains(c)).collect()
    }

    pub fn is_shown(&self, col: ColumnId) -> bool {
        !self.hidden.contains(&col)
    }

    pub fn set_shown(&mut self, col: ColumnId, shown: bool) {
        self.hidden.retain(|&c| c != col);
        if !shown {
            self.hidden.push(col);
        }
    }

    pub fn move_next_to(&mut self, col: ColumnId, target: ColumnId, after: bool) {
        if col == target {
            return;
        }
        self.order.retain(|&c| c != col);
        let at = self.order.iter().position(|&c| c == target).map_or(self.order.len(), |i| i + usize::from(after));
        self.order.insert(at, col);
    }

    /// Swaps `col` with its visible neighbour, skipping hidden columns.
    pub fn shift(&mut self, col: ColumnId, right: bool) {
        let visible = self.visible();
        let Some(index) = visible.iter().position(|&c| c == col) else { return };
        let neighbour = if right { visible.get(index + 1) } else { index.checked_sub(1).and_then(|i| visible.get(i)) };
        if let Some(&target) = neighbour {
            self.move_next_to(col, target, right);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ColumnId::*;

    #[test]
    fn moving_next_to_a_column_keeps_every_column_once() {
        let mut layout = ColumnLayout::default();
        layout.move_next_to(Path, Id, false);
        assert_eq!(&layout.order[..3], &[Path, Id, Method]);
        layout.move_next_to(Path, ResponseAt, true);
        assert_eq!(layout.order.last(), Some(&Path));
        assert_eq!(layout.order.len(), ColumnId::ALL.len());
    }

    #[test]
    fn shifting_skips_hidden_columns() {
        let mut layout = ColumnLayout::default();
        layout.set_shown(Source, false);
        layout.shift(Host, false);
        assert_eq!(layout.visible()[..3], [Id, Host, Method]);
        layout.shift(Id, false);
        assert_eq!(layout.visible()[0], Id);
    }
}
