use eframe::egui::Align;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
