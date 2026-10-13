//! What the database file holds, and giving free pages back to the disk.

use super::Db;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableStorage {
    pub name: String,
    pub rows: u64,
    /// The table's pages plus its indexes'.
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StorageStats {
    pub file_bytes: u64,
    pub wal_bytes: u64,
    /// Pages left by deleted rows; SQLite reuses them but only VACUUM returns them to the disk.
    pub free_bytes: u64,
    /// Largest first.
    pub tables: Vec<TableStorage>,
}

impl StorageStats {
    pub fn used_bytes(&self) -> u64 {
        self.file_bytes.saturating_sub(self.free_bytes)
    }

    pub fn should_vacuum(&self) -> bool {
        self.free_bytes >= 8 * 1024 * 1024 && self.free_bytes * 5 >= self.file_bytes
    }
}

pub fn storage_stats(db: &Db) -> Result<StorageStats, String> {
    let conn = db.conn().map_err(|e| e.to_string())?;
    let pragma = |name: &str| conn.query_row(&format!("PRAGMA {name}"), [], |r| r.get::<_, i64>(0)).map_err(|e| e.to_string());
    let page_size = pragma("page_size")? as u64;
    let file_bytes = pragma("page_count")? as u64 * page_size;
    let free_bytes = pragma("freelist_count")? as u64 * page_size;
    let wal_bytes = std::fs::metadata(wal_path(db)).map_or(0, |m| m.len());

    let sizes: Vec<(String, u64)> = {
        let mut stmt = conn
            .prepare(
                "SELECT COALESCE(m.tbl_name, s.name), SUM(s.pgsize) FROM dbstat s
                 LEFT JOIN sqlite_schema m ON m.name = s.name
                 GROUP BY 1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as u64))).map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect()
    };
    let mut tables: Vec<TableStorage> = Vec::new();
    for (name, bytes) in sizes {
        if name.starts_with("sqlite_") {
            continue;
        }
        let rows = conn
            .query_row(&format!("SELECT COUNT(*) FROM \"{}\"", name.replace('"', "\"\"")), [], |r| r.get::<_, i64>(0))
            .map_or(0, |n| n as u64);
        tables.push(TableStorage { name, rows, bytes });
    }
    tables.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    Ok(StorageStats { file_bytes, wal_bytes, free_bytes, tables })
}

/// Rewrites the file without its free pages and empties the WAL. Returns the file size before and after.
pub fn vacuum(db: &Db) -> Result<(u64, u64), String> {
    let size = || std::fs::metadata(db.path()).map_or(0, |m| m.len()) + std::fs::metadata(wal_path(db)).map_or(0, |m| m.len());
    let before = size();
    let conn = db.conn().map_err(|e| e.to_string())?;
    conn.execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);").map_err(|e| e.to_string())?;
    Ok((before, size()))
}

fn wal_path(db: &Db) -> std::path::PathBuf {
    let mut path = db.path().as_os_str().to_owned();
    path.push("-wal");
    path.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_show_freed_pages_and_vacuum_returns_them() {
        let db = Db::new(std::env::temp_dir().join(format!("mitm-storage-{}.db", uuid::Uuid::new_v4())));
        crate::db::init_database(&db, None).unwrap();
        let conn = db.conn().unwrap();
        conn.execute_batch("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB); CREATE INDEX idx_blobs_data ON blobs(id, data);")
            .unwrap();
        for _ in 0..200 {
            conn.execute("INSERT INTO blobs (data) VALUES (zeroblob(50000))", []).unwrap();
        }
        let full = storage_stats(&db).unwrap();
        let blobs = full.tables.iter().find(|t| t.name == "blobs").unwrap();
        assert_eq!(blobs.rows, 200);
        assert!(blobs.bytes > 2 * 10_000_000, "the index counts toward its table: {}", blobs.bytes);
        assert!(full.tables.iter().all(|t| !t.name.starts_with("idx_")));

        conn.execute("DELETE FROM blobs", []).unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
        let emptied = storage_stats(&db).unwrap();
        assert!(emptied.should_vacuum(), "{emptied:?}");

        let (before, after) = vacuum(&db).unwrap();
        assert!(after * 4 < before, "{before} -> {after}");
        assert!(!storage_stats(&db).unwrap().should_vacuum());
        let _ = std::fs::remove_file(db.path());
    }
}
