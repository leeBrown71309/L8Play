//! One-time move of the default snapshot folder after the app was renamed from "L8 Play" to "L8Play".
//!
//! Archive paths are stored as absolute paths in the database, so moving the folder also rewrites
//! them. The move only happens when the new folder does not exist yet, so existing data is never
//! merged or overwritten.

use std::path::Path;

use rusqlite::params;

use crate::db::Database;
use crate::error::CoreResult;

/// Moves `legacy_root` to `new_root` and rebases the recorded archive paths. Returns whether a move
/// happened (`false` when there is nothing to move or the destination already exists).
pub fn migrate_legacy_root(db: &Database, legacy_root: &Path, new_root: &Path) -> CoreResult<bool> {
    if !legacy_root.is_dir() || new_root.exists() {
        return Ok(false);
    }
    if let Some(parent) = new_root.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(legacy_root, new_root)?;
    let legacy = legacy_root.to_string_lossy();
    let current = new_root.to_string_lossy();
    let rebased = db.connection()?.execute(
        "UPDATE snapshots SET archive_path = ?2 || substr(archive_path, length(?1) + 1)
         WHERE archive_path IS NOT NULL AND substr(archive_path, 1, length(?1)) = ?1",
        params![legacy, current],
    )?;
    // The old parent folder ("L8 Play") is removed only if it is now empty.
    if let Some(parent) = legacy_root.parent() {
        let _ = std::fs::remove_dir(parent);
    }
    tracing::info!(from = %legacy_root.display(), to = %new_root.display(), rebased, "snapshot folder moved");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_snapshot(db: &Database, id: &str, path: &Path) -> CoreResult<()> {
        let connection = db.connection()?;
        connection.execute("INSERT OR IGNORE INTO devices (id, name, is_current) VALUES ('d', 'pc', 1)", [])?;
        connection.execute(
            "INSERT OR IGNORE INTO games (id, platform_id, title, sort_title, slug, added_at, updated_at)
             VALUES (1, 'standalone', 'G', 'g', 'g', 'x', 'x')",
            [],
        )?;
        connection.execute(
            "INSERT INTO snapshots (id, game_id, name, kind, status, device_id, archive_path, archive_size, archive_sha256,
                                    total_files, total_bytes, is_pinned, created_at)
             VALUES (?1, 1, 'n', 'manual', 'ready', 'd', ?2, 1, 'sha', 1, 1, 0, 'x')",
            params![id, path.to_string_lossy()],
        )?;
        Ok(())
    }

    #[test]
    fn moves_the_folder_and_rebases_paths() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        let legacy = dir.path().join("L8 Play").join("Snapshots");
        let current = dir.path().join("L8Play").join("Snapshots");
        std::fs::create_dir_all(legacy.join("game"))?;
        std::fs::write(legacy.join("game").join("a.zip"), b"zip")?;
        let db = Database::open_in_memory()?;
        insert_snapshot(&db, "a", &legacy.join("game").join("a.zip"))?;
        insert_snapshot(&db, "elsewhere", &dir.path().join("custom").join("b.zip"))?;

        assert!(migrate_legacy_root(&db, &legacy, &current)?);
        assert!(current.join("game").join("a.zip").is_file());
        assert!(!dir.path().join("L8 Play").exists(), "the emptied legacy parent is removed");
        let paths: Vec<String> = {
            let connection = db.connection()?;
            let mut statement = connection.prepare("SELECT archive_path FROM snapshots ORDER BY id")?;
            statement.query_map([], |row| row.get(0))?.collect::<Result<_, _>>()?
        };
        assert_eq!(paths[0], current.join("game").join("a.zip").to_string_lossy());
        assert_eq!(paths[1], dir.path().join("custom").join("b.zip").to_string_lossy(), "other paths are untouched");
        Ok(())
    }

    #[test]
    fn never_merges_into_an_existing_destination() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        let legacy = dir.path().join("L8 Play").join("Snapshots");
        let current = dir.path().join("L8Play").join("Snapshots");
        std::fs::create_dir_all(&legacy)?;
        std::fs::create_dir_all(&current)?;
        let db = Database::open_in_memory()?;
        assert!(!migrate_legacy_root(&db, &legacy, &current)?);
        assert!(legacy.is_dir(), "nothing was moved");
        assert!(!migrate_legacy_root(&db, &dir.path().join("missing"), &dir.path().join("new"))?);
        Ok(())
    }
}
