use std::fmt;
use rusqlite::ErrorCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitError {
    /// Another process holds a lock on the database; retrying later can succeed.
    Locked(String),
    Migration(String),
    Io(String),
}

impl InitError {
    pub fn migration(message: String) -> Self {
        // Migrations report errors as strings, so a busy database only shows up in the text.
        if message.contains("database is locked") || message.contains("database table is locked") {
            InitError::Locked(message)
        } else {
            InitError::Migration(message)
        }
    }

    pub fn message(&self) -> &str {
        match self {
            InitError::Locked(m) | InitError::Migration(m) | InitError::Io(m) => m,
        }
    }
}

impl From<rusqlite::Error> for InitError {
    fn from(err: rusqlite::Error) -> Self {
        match err.sqlite_error_code() {
            Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => InitError::Locked(err.to_string()),
            _ => InitError::Io(err.to_string()),
        }
    }
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self {
            InitError::Locked(_) => "Database is locked",
            InitError::Migration(_) => "Database migration failed",
            InitError::Io(_) => "Database could not be opened",
        };
        write!(f, "{}: {}", kind, self.message())
    }
}

impl std::error::Error for InitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busy_migration_errors_are_classed_as_locked() {
        assert!(matches!(InitError::migration("database is locked".into()), InitError::Locked(_)));
        assert!(matches!(InitError::migration("no such table: x".into()), InitError::Migration(_)));
    }

    #[test]
    fn unopenable_file_is_an_io_error() {
        let dir = std::env::temp_dir().join(format!("mitm-init-error-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        // A directory can't be opened as a database file.
        let err = super::super::init_database(&dir, None).unwrap_err();
        assert!(matches!(err, InitError::Io(_)), "{:?}", err);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
