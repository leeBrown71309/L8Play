//! SQLite connection management and schema migrations.
//!
//! The database holds metadata only; snapshot archives and artwork live on disk. Before any
//! pending migration runs, the existing database file is copied next to itself so a failed
//! upgrade can never lose the user's library or snapshot history.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations, SchemaVersion};

use crate::error::{CoreError, CoreResult};

const MIGRATIONS_SLICE: &[M<'_>] = &[
    M::up(include_str!("../migrations/0001_init.sql")),
    M::up(include_str!("../migrations/0002_cloud_deletions.sql")),
    M::up(include_str!("../migrations/0003_xbox_and_gog.sql")),
    M::up(include_str!("../migrations/0004_snapshot_storage.sql")),
    M::up(include_str!("../migrations/0005_archived_games.sql")),
];
const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATIONS_SLICE);

/// Single-writer SQLite handle. Phase 1 traffic is tiny, so one serialized connection is enough;
/// a reader pool can be added behind this type without touching callers.
pub struct Database {
    connection: Mutex<Connection>,
    path: Option<PathBuf>,
}

impl Database {
    /// Opens (or creates) the database file, applies pragmas and runs pending migrations.
    pub fn open(path: &Path) -> CoreResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let existed = path.exists() && std::fs::metadata(path)?.len() > 0;
        let mut connection = Connection::open(path)?;
        configure(&connection)?;
        if existed && has_pending_upgrade(&connection)? {
            backup_before_migration(&connection, path)?;
        }
        MIGRATIONS.to_latest(&mut connection)?;
        tracing::info!(db = %path.display(), "database ready");
        Ok(Self { connection: Mutex::new(connection), path: Some(path.to_path_buf()) })
    }

    /// In-memory database with the full schema, for tests.
    pub fn open_in_memory() -> CoreResult<Self> {
        let mut connection = Connection::open_in_memory()?;
        configure(&connection)?;
        MIGRATIONS.to_latest(&mut connection)?;
        Ok(Self { connection: Mutex::new(connection), path: None })
    }

    /// Exclusive access to the connection. Keep the guard short-lived.
    pub fn connection(&self) -> CoreResult<MutexGuard<'_, Connection>> {
        self.connection.lock().map_err(|_| CoreError::LockPoisoned("database connection"))
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Applied migration count (0 = empty database).
    pub fn schema_version(&self) -> CoreResult<usize> {
        let connection = self.connection()?;
        Ok(match MIGRATIONS.current_version(&connection)? {
            SchemaVersion::NoneSet => 0,
            SchemaVersion::Inside(version) | SchemaVersion::Outside(version) => version.get(),
        })
    }
}

fn configure(connection: &Connection) -> CoreResult<()> {
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
}

/// True when an already-initialised database is about to be upgraded (a fresh one has nothing to lose).
fn has_pending_upgrade(connection: &Connection) -> CoreResult<bool> {
    Ok(match MIGRATIONS.current_version(connection)? {
        SchemaVersion::NoneSet => false,
        SchemaVersion::Inside(version) | SchemaVersion::Outside(version) => version.get() < MIGRATIONS_SLICE.len(),
    })
}

/// Copies the live database with SQLite's `VACUUM INTO`, which is consistent even in WAL mode.
fn backup_before_migration(connection: &Connection, path: &Path) -> CoreResult<()> {
    let backup = path.with_extension("pre-migration.db");
    if backup.exists() {
        std::fs::remove_file(&backup)?;
    }
    connection.execute("VACUUM INTO ?1", [backup.to_string_lossy()])?;
    tracing::info!(backup = %backup.display(), "database backed up before migration");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        assert!(MIGRATIONS.validate().is_ok());
    }

    #[test]
    fn in_memory_database_has_full_schema() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        assert_eq!(db.schema_version()?, MIGRATIONS_SLICE.len());
        let platforms: i64 = db.connection()?.query_row("SELECT COUNT(*) FROM platforms", [], |row| row.get(0))?;
        assert_eq!(platforms, 5);
        Ok(())
    }

    #[test]
    fn file_database_is_created_and_reopened() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("nested").join("l8.db");
        {
            let db = Database::open(&path)?;
            assert_eq!(db.schema_version()?, MIGRATIONS_SLICE.len());
        }
        let reopened = Database::open(&path)?;
        assert_eq!(reopened.schema_version()?, MIGRATIONS_SLICE.len());
        assert!(!path.with_extension("pre-migration.db").exists(), "no backup when nothing is pending");
        Ok(())
    }

    #[test]
    fn foreign_keys_are_enforced() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let result = db.connection()?.execute(
            "INSERT INTO games (platform_id, title, sort_title, slug, added_at, updated_at) \
             VALUES ('unknown', 't', 't', 't', 'now', 'now')",
            [],
        );
        assert!(result.is_err());
        Ok(())
    }
}
