//! Persistence of snapshots (`snapshots`, `snapshot_files`, `devices`).

use std::path::Path;

use rusqlite::{OptionalExtension, Row, Transaction, params};

use crate::clock::now_rfc3339;
use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::snapshots::archive::ArchiveManifest;
use crate::snapshots::model::{
    CloudState, GameSaveStatus, SnapshotKind, SnapshotStatus, SnapshotStorage, SnapshotSummary,
};

const SUMMARY_SELECT: &str = "
    SELECT s.id, s.game_id, g.title, s.name, s.description, s.kind, s.status, s.created_at, s.verified_at,
           s.archive_path, s.archive_size, s.total_files, s.total_bytes, s.archive_sha256, s.playtime_seconds,
           s.is_pinned, d.name,
           CASE
             WHEN EXISTS (SELECT 1 FROM cloud_objects c WHERE c.snapshot_id = s.id AND c.status = 'present') THEN 'synced'
             WHEN EXISTS (SELECT 1 FROM sync_operations o WHERE o.snapshot_id = s.id AND o.direction = 'upload' AND o.status = 'running') THEN 'uploading'
             WHEN EXISTS (SELECT 1 FROM sync_operations o WHERE o.snapshot_id = s.id AND o.direction = 'upload' AND o.status IN ('queued', 'paused')) THEN 'queued'
             WHEN EXISTS (SELECT 1 FROM sync_operations o WHERE o.snapshot_id = s.id AND o.direction = 'upload' AND o.status = 'failed') THEN 'failed'
             ELSE 'none'
           END
    FROM snapshots s JOIN games g ON g.id = s.game_id JOIN devices d ON d.id = s.device_id";

fn summary_from_row(row: &Row<'_>) -> rusqlite::Result<SnapshotSummary> {
    let archive_path: Option<String> = row.get(9)?;
    let exists = archive_path.as_deref().is_some_and(|path| Path::new(path).is_file());
    Ok(SnapshotSummary {
        id: row.get(0)?,
        game_id: row.get(1)?,
        game_title: row.get(2)?,
        name: row.get(3)?,
        description: row.get(4)?,
        kind: SnapshotKind::from_db(&row.get::<_, String>(5)?),
        status: SnapshotStatus::from_db(&row.get::<_, String>(6)?, exists),
        created_at: row.get(7)?,
        verified_at: row.get(8)?,
        archive_path,
        archive_size_bytes: row.get(10)?,
        total_files: row.get(11)?,
        total_bytes: row.get(12)?,
        archive_sha256: row.get(13)?,
        playtime_seconds: row.get(14)?,
        pinned: row.get(15)?,
        device_name: row.get(16)?,
        cloud: CloudState::from_db(&row.get::<_, String>(17)?),
    })
}

/// Everything needed to record a verified archive.
pub struct NewSnapshot<'a> {
    pub id: &'a str,
    pub game_id: i64,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub kind: SnapshotKind,
    pub pinned: bool,
    pub storage: SnapshotStorage,
    pub archive_path: &'a Path,
    pub archive_size: i64,
    pub archive_sha256: &'a str,
    pub manifest: &'a ArchiveManifest,
}

pub struct SnapshotRepository<'db> {
    db: &'db Database,
}

impl<'db> SnapshotRepository<'db> {
    pub fn new(db: &'db Database) -> Self {
        Self { db }
    }

    /// Id of this PC in `devices`, created on first use (name = Windows computer name).
    pub fn current_device(&self) -> CoreResult<String> {
        let connection = self.db.connection()?;
        if let Some(id) =
            connection.query_row("SELECT id FROM devices WHERE is_current = 1", [], |row| row.get(0)).optional()?
        {
            return Ok(id);
        }
        let id = uuid::Uuid::now_v7().to_string();
        let name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "This PC".into());
        connection.execute("INSERT INTO devices (id, name, is_current) VALUES (?1, ?2, 1)", params![id, name])?;
        Ok(id)
    }

    /// Records a snapshot and its files in one transaction. Called only after the archive passed
    /// verification and was renamed into place.
    pub fn insert(&self, snapshot: &NewSnapshot<'_>) -> CoreResult<()> {
        let device = self.current_device()?;
        let mut connection = self.db.connection()?;
        let tx = connection.transaction()?;
        let parent = latest_ready(&tx, snapshot.game_id)?;
        let playtime: Option<i64> = tx
            .query_row("SELECT playtime_seconds FROM games WHERE id = ?1", [snapshot.game_id], |row| row.get(0))
            .optional()?;
        let now = now_rfc3339();
        tx.execute(
            "INSERT INTO snapshots (id, game_id, name, description, kind, status, device_id, parent_snapshot_id, archive_path,
                                    archive_size, archive_sha256, total_files, total_bytes, playtime_seconds, is_pinned, created_at, verified_at, storage)
             VALUES (?1, ?2, ?3, ?4, ?5, 'ready', ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                snapshot.id,
                snapshot.game_id,
                snapshot.name,
                snapshot.description,
                snapshot.kind.as_db(),
                device,
                parent,
                snapshot.archive_path.to_string_lossy(),
                snapshot.archive_size,
                snapshot.archive_sha256,
                i64::try_from(snapshot.manifest.files.len()).unwrap_or(i64::MAX),
                i64::try_from(snapshot.manifest.total_bytes()).unwrap_or(i64::MAX),
                playtime,
                snapshot.pinned,
                snapshot.manifest.created_at,
                now,
                snapshot.storage.as_db(),
            ],
        )?;
        for file in &snapshot.manifest.files {
            tx.execute(
                "INSERT INTO snapshot_files (snapshot_id, rel_path, original_path, size_bytes, sha256, modified_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    snapshot.id,
                    file.entry,
                    file.original_path,
                    i64::try_from(file.size).unwrap_or(i64::MAX),
                    file.sha256,
                    file.modified_at.clone().unwrap_or_default()
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> CoreResult<SnapshotSummary> {
        self.db
            .connection()?
            .query_row(&format!("{SUMMARY_SELECT} WHERE s.id = ?1"), [id], summary_from_row)
            .optional()?
            .ok_or_else(|| CoreError::NotFound(format!("snapshot {id}")))
    }

    /// Newest first; all games when `game_id` is `None`.
    pub fn list(&self, game_id: Option<i64>) -> CoreResult<Vec<SnapshotSummary>> {
        let connection = self.db.connection()?;
        let sql = format!(
            "{SUMMARY_SELECT} WHERE s.status <> 'deleted' AND (?1 IS NULL OR s.game_id = ?1) ORDER BY s.created_at DESC"
        );
        let mut statement = connection.prepare(&sql)?;
        let snapshots = statement.query_map([game_id], summary_from_row)?.collect::<Result<Vec<_>, _>>()?;
        Ok(snapshots)
    }

    pub fn latest(&self, game_id: i64) -> CoreResult<Option<SnapshotSummary>> {
        Ok(self.list(Some(game_id))?.into_iter().find(|snapshot| snapshot.status == SnapshotStatus::Ready))
    }

    /// (original path, sha256) of every file in a snapshot.
    pub fn file_hashes(&self, id: &str) -> CoreResult<Vec<(String, String)>> {
        let connection = self.db.connection()?;
        let mut statement =
            connection.prepare("SELECT original_path, sha256 FROM snapshot_files WHERE snapshot_id = ?1")?;
        let rows = statement.query_map([id], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn set_status(&self, id: &str, status: &str) -> CoreResult<()> {
        self.db.connection()?.execute("UPDATE snapshots SET status = ?2 WHERE id = ?1", params![id, status])?;
        Ok(())
    }

    pub fn mark_verified(&self, id: &str) -> CoreResult<()> {
        self.db.connection()?.execute(
            "UPDATE snapshots SET status = 'ready', verified_at = ?2 WHERE id = ?1",
            params![id, now_rfc3339()],
        )?;
        Ok(())
    }

    pub fn set_pinned(&self, id: &str, pinned: bool) -> CoreResult<()> {
        let changed =
            self.db.connection()?.execute("UPDATE snapshots SET is_pinned = ?2 WHERE id = ?1", params![id, pinned])?;
        if changed == 0 {
            return Err(CoreError::NotFound(format!("snapshot {id}")));
        }
        Ok(())
    }

    /// Where the snapshot is meant to live (see [`SnapshotStorage`]).
    pub fn storage(&self, id: &str) -> CoreResult<SnapshotStorage> {
        let value: String =
            self.db.connection()?.query_row("SELECT storage FROM snapshots WHERE id = ?1", [id], |row| row.get(0))?;
        Ok(SnapshotStorage::from_db(&value))
    }

    /// Whether a verified copy of the snapshot is in the cloud.
    pub fn has_cloud_copy(&self, id: &str) -> CoreResult<bool> {
        Ok(self.db.connection()?.query_row(
            "SELECT EXISTS (SELECT 1 FROM cloud_objects WHERE snapshot_id = ?1 AND status = 'present')",
            [id],
            |row| row.get(0),
        )?)
    }

    /// Forgets the local archive of a snapshot that lives on in the cloud (and cancels its
    /// pending uploads); the snapshot stays listed as cloud-only.
    pub fn detach_local(&self, id: &str) -> CoreResult<()> {
        let connection = self.db.connection()?;
        connection.execute("UPDATE snapshots SET archive_path = NULL WHERE id = ?1", [id])?;
        connection.execute(
            "UPDATE sync_operations SET status = 'cancelled', updated_at = ?2
             WHERE snapshot_id = ?1 AND direction = 'upload' AND status IN ('queued', 'paused', 'failed')",
            params![id, now_rfc3339()],
        )?;
        Ok(())
    }

    /// Points a cloud-only snapshot at its freshly downloaded archive.
    pub fn attach_archive(&self, id: &str, path: &Path, size: i64) -> CoreResult<()> {
        self.db.connection()?.execute(
            "UPDATE snapshots SET archive_path = ?2, archive_size = ?3, status = 'ready' WHERE id = ?1",
            params![id, path.to_string_lossy(), size],
        )?;
        Ok(())
    }

    pub fn set_kind(&self, id: &str, kind: SnapshotKind) -> CoreResult<()> {
        self.db.connection()?.execute("UPDATE snapshots SET kind = ?2 WHERE id = ?1", params![id, kind.as_db()])?;
        Ok(())
    }

    /// Removes the rows of a snapshot; its children are re-linked to its parent so lineage survives.
    pub fn remove(&self, id: &str) -> CoreResult<()> {
        let mut connection = self.db.connection()?;
        let tx = connection.transaction()?;
        let parent: Option<String> = tx
            .query_row("SELECT parent_snapshot_id FROM snapshots WHERE id = ?1", [id], |row| row.get(0))
            .optional()?
            .flatten();
        tx.execute("UPDATE snapshots SET parent_snapshot_id = ?2 WHERE parent_snapshot_id = ?1", params![id, parent])?;
        tx.execute("DELETE FROM activity_log WHERE snapshot_id = ?1", [id])?;
        // Bookkeeping that would block the delete. A cloud copy still believed present is kept on
        // purpose: its foreign key refuses the removal until the copy has been handed to the
        // deletion queue, so a Drive file can never be orphaned silently.
        tx.execute("DELETE FROM sync_operations WHERE snapshot_id = ?1", [id])?;
        tx.execute("DELETE FROM cloud_objects WHERE snapshot_id = ?1 AND status <> 'present'", [id])?;
        tx.execute("DELETE FROM snapshots WHERE id = ?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    /// Automatic, unpinned snapshots of a game whose archive is on this PC, beyond the newest
    /// `keep`. Snapshots in a sync conflict are left out.
    pub fn retention_candidates(&self, game_id: i64, keep: u32) -> CoreResult<Vec<String>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(&format!(
            "SELECT id FROM snapshots WHERE game_id = ?1 AND is_pinned = 0 AND archive_path IS NOT NULL
               AND kind IN ('auto_exit', 'auto_launch', 'scheduled') AND {NOT_IN_CONFLICT}
             ORDER BY created_at DESC LIMIT -1 OFFSET ?2"
        ))?;
        let ids = statement.query_map(params![game_id, keep], |row| row.get(0))?.collect::<Result<Vec<String>, _>>()?;
        Ok(ids)
    }

    /// Automatic, unpinned snapshots of a game with a Drive copy, beyond the newest `keep`.
    pub fn cloud_retention_candidates(&self, game_id: i64, keep: u32) -> CoreResult<Vec<String>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(&format!(
            "SELECT id FROM snapshots WHERE game_id = ?1 AND is_pinned = 0
               AND kind IN ('auto_exit', 'auto_launch', 'scheduled') AND {NOT_IN_CONFLICT}
               AND EXISTS (SELECT 1 FROM cloud_objects o WHERE o.snapshot_id = snapshots.id AND o.status = 'present')
             ORDER BY created_at DESC LIMIT -1 OFFSET ?2"
        ))?;
        let ids = statement.query_map(params![game_id, keep], |row| row.get(0))?.collect::<Result<Vec<String>, _>>()?;
        Ok(ids)
    }

    /// Protection figures for every game.
    pub fn save_statuses(&self) -> CoreResult<Vec<GameSaveStatus>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT g.id,
                    (SELECT COUNT(*) FROM save_locations l WHERE l.game_id = g.id AND l.status <> 'rejected'),
                    (SELECT COUNT(*) FROM snapshots s WHERE s.game_id = g.id AND s.status <> 'deleted'),
                    (SELECT MAX(created_at) FROM snapshots s WHERE s.game_id = g.id AND s.status <> 'deleted'),
                    EXISTS (SELECT 1 FROM game_external_ids x WHERE x.game_id = g.id AND x.provider = 'ludusavi' AND x.external_id = '')
             FROM games g",
        )?;
        let statuses = statement
            .query_map([], |row| {
                Ok(GameSaveStatus {
                    game_id: row.get(0)?,
                    location_count: row.get(1)?,
                    snapshot_count: row.get(2)?,
                    last_snapshot_at: row.get(3)?,
                    unknown_to_ludusavi: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(statuses)
    }
}

/// Snapshots that sync conflicts point at are history the user may still need to compare.
const NOT_IN_CONFLICT: &str = "NOT EXISTS (SELECT 1 FROM sync_conflicts c
       WHERE c.local_snapshot_id = snapshots.id OR c.remote_snapshot_id = snapshots.id)";

fn latest_ready(tx: &Transaction<'_>, game_id: i64) -> CoreResult<Option<String>> {
    Ok(tx
        .query_row(
            "SELECT id FROM snapshots WHERE game_id = ?1 AND status = 'ready' ORDER BY created_at DESC LIMIT 1",
            [game_id],
            |row| row.get(0),
        )
        .optional()?)
}
