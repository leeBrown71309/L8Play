//! Persistence of cloud state: account, uploaded objects, the sync queue and conflicts.

use rusqlite::{OptionalExtension, params};

use crate::clock::{now_plus_seconds_rfc3339, now_rfc3339};
use crate::cloud::model::{
    AccountStatus, CloudAccountInfo, ConflictResolution, SyncDirection, SyncOperationInfo, SyncStatus,
};
use crate::db::Database;
use crate::error::{CoreError, CoreResult};

pub const MAX_ATTEMPTS: i64 = 8;
const BACKOFF_BASE_SECONDS: i64 = 30;
const BACKOFF_CAP_SECONDS: i64 = 3600;
const KEY_LAST_SYNC: &str = "cloud.lastSyncAt";
const CREDENTIAL_REF: &str = "L8Play/google_refresh_token";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRow {
    pub id: i64,
    pub email: String,
    pub status: AccountStatus,
    pub connected_at: String,
}

impl From<&AccountRow> for CloudAccountInfo {
    fn from(row: &AccountRow) -> Self {
        Self { email: row.email.clone(), status: row.status, connected_at: row.connected_at.clone() }
    }
}

/// An operation ready to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueOperation {
    pub id: i64,
    pub snapshot_id: String,
    pub direction: SyncDirection,
    pub attempts: i64,
    pub bytes_total: i64,
    pub resumable_uri: Option<String>,
    pub bytes_done: i64,
}

/// A Drive copy waiting to be deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueDeletion {
    pub id: i64,
    pub snapshot_id: String,
    pub remote_id: String,
    pub attempts: i64,
}

/// An unresolved conflict between this PC's head and another device's head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenConflict {
    pub id: i64,
    pub game_id: i64,
    pub local_id: String,
    pub remote_id: String,
    pub detected_at: String,
}

pub fn backoff_seconds(attempts: i64) -> i64 {
    let exponent = u32::try_from((attempts - 1).clamp(0, 12)).unwrap_or(0);
    (BACKOFF_BASE_SECONDS * 2_i64.pow(exponent)).min(BACKOFF_CAP_SECONDS)
}

pub struct CloudRepository<'db> {
    db: &'db Database,
}

impl<'db> CloudRepository<'db> {
    pub fn new(db: &'db Database) -> Self {
        Self { db }
    }

    pub fn account(&self) -> CoreResult<Option<AccountRow>> {
        Ok(self
            .db
            .connection()?
            .query_row(
                "SELECT id, account_email, status, connected_at FROM cloud_accounts WHERE provider = 'google_drive' LIMIT 1",
                [],
                |row| {
                    let status: String = row.get(2)?;
                    Ok(AccountRow {
                        id: row.get(0)?,
                        email: row.get(1)?,
                        status: if status == "connected" { AccountStatus::Connected } else { AccountStatus::NeedsReauth },
                        connected_at: row.get(3)?,
                    })
                },
            )
            .optional()?)
    }

    /// One Drive account at a time: connecting replaces any previous one.
    pub fn connect(&self, email: &str, root_folder_id: &str) -> CoreResult<AccountRow> {
        let mut connection = self.db.connection()?;
        let tx = connection.transaction()?;
        tx.execute("DELETE FROM cloud_accounts", [])?;
        tx.execute(
            "INSERT INTO cloud_accounts (provider, account_email, root_folder_id, credential_ref, status, connected_at)
             VALUES ('google_drive', ?1, ?2, ?3, 'connected', ?4)",
            params![email, root_folder_id, CREDENTIAL_REF, now_rfc3339()],
        )?;
        tx.commit()?;
        drop(connection);
        self.account()?.ok_or_else(|| CoreError::NotFound("cloud account".into()))
    }

    /// Forgets the account; cascades to its objects and queue. Snapshots are untouched.
    pub fn disconnect(&self) -> CoreResult<()> {
        self.db.connection()?.execute("DELETE FROM cloud_accounts", [])?;
        Ok(())
    }

    pub fn set_status(&self, account_id: i64, status: AccountStatus) -> CoreResult<()> {
        let value = if status == AccountStatus::Connected { "connected" } else { "needs_reauth" };
        self.db
            .connection()?
            .execute("UPDATE cloud_accounts SET status = ?2 WHERE id = ?1", params![account_id, value])?;
        Ok(())
    }

    // ----- objects -----

    pub fn remote_id(&self, account_id: i64, snapshot_id: &str) -> CoreResult<Option<String>> {
        Ok(self
            .db
            .connection()?
            .query_row(
                "SELECT remote_id FROM cloud_objects WHERE account_id = ?1 AND snapshot_id = ?2 AND status = 'present'",
                params![account_id, snapshot_id],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn has_object_row(&self, account_id: i64, snapshot_id: &str) -> CoreResult<bool> {
        Ok(self.db.connection()?.query_row(
            "SELECT EXISTS (SELECT 1 FROM cloud_objects WHERE account_id = ?1 AND snapshot_id = ?2)",
            params![account_id, snapshot_id],
            |row| row.get(0),
        )?)
    }

    pub fn record_object(
        &self,
        account_id: i64,
        snapshot_id: &str,
        remote_id: &str,
        remote_path: &str,
        size: i64,
        sha256: &str,
    ) -> CoreResult<()> {
        let mut connection = self.db.connection()?;
        let tx = connection.transaction()?;
        tx.execute(
            "DELETE FROM cloud_objects WHERE account_id = ?1 AND (snapshot_id = ?2 OR remote_id = ?3)",
            params![account_id, snapshot_id, remote_id],
        )?;
        tx.execute(
            "INSERT INTO cloud_objects (account_id, snapshot_id, remote_id, remote_path, size_bytes, sha256, uploaded_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'present')",
            params![account_id, snapshot_id, remote_id, remote_path, size, sha256, now_rfc3339()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_object_status(&self, account_id: i64, snapshot_id: &str, status: &str) -> CoreResult<()> {
        self.db.connection()?.execute(
            "UPDATE cloud_objects SET status = ?3 WHERE account_id = ?1 AND snapshot_id = ?2",
            params![account_id, snapshot_id, status],
        )?;
        Ok(())
    }

    /// (snapshot id, remote id) of every object believed present.
    pub fn present_objects(&self, account_id: i64) -> CoreResult<Vec<(String, String)>> {
        let connection = self.db.connection()?;
        let mut statement = connection
            .prepare("SELECT snapshot_id, remote_id FROM cloud_objects WHERE account_id = ?1 AND status = 'present'")?;
        let rows =
            statement.query_map([account_id], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ----- deletions -----

    /// Hands the Drive copies of a snapshot over to the deletion queue and forgets them locally,
    /// together with the snapshot's own sync operations (they would block removing the row).
    /// Returns how many Drive files were scheduled. Nothing is contacted here: the queue does that
    /// when Drive is reachable, so being offline or signed out only delays the deletion.
    pub fn schedule_remote_deletion(&self, snapshot_id: &str) -> CoreResult<usize> {
        let mut connection = self.db.connection()?;
        let tx = connection.transaction()?;
        let scheduled = tx.execute(
            "INSERT OR IGNORE INTO cloud_deletions (account_id, snapshot_id, remote_id, created_at)
             SELECT account_id, snapshot_id, remote_id, ?2 FROM cloud_objects
             WHERE snapshot_id = ?1 AND status IN ('present', 'integrity_mismatch')",
            params![snapshot_id, now_rfc3339()],
        )?;
        tx.execute("DELETE FROM cloud_objects WHERE snapshot_id = ?1", [snapshot_id])?;
        tx.execute("DELETE FROM sync_operations WHERE snapshot_id = ?1", [snapshot_id])?;
        tx.commit()?;
        Ok(scheduled)
    }

    /// Deletions whose retry time has come, oldest first. After [`MAX_ATTEMPTS`] failures an entry
    /// waits for the user's "Retry".
    pub fn due_deletions(&self, account_id: i64, limit: u32) -> CoreResult<Vec<DueDeletion>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, snapshot_id, remote_id, attempts FROM cloud_deletions
             WHERE account_id = ?1 AND attempts < ?2 AND (next_attempt_at IS NULL OR next_attempt_at <= ?3)
             ORDER BY created_at LIMIT ?4",
        )?;
        let rows = statement
            .query_map(params![account_id, MAX_ATTEMPTS, now_rfc3339(), limit], |row| {
                Ok(DueDeletion {
                    id: row.get(0)?,
                    snapshot_id: row.get(1)?,
                    remote_id: row.get(2)?,
                    attempts: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn finish_deletion(&self, id: i64) -> CoreResult<()> {
        self.db.connection()?.execute("DELETE FROM cloud_deletions WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Records a failed attempt; `permanent` skips the automatic retries.
    pub fn fail_deletion(&self, id: i64, attempts: i64, error: &str, permanent: bool) -> CoreResult<()> {
        let attempts = if permanent { MAX_ATTEMPTS } else { attempts };
        self.db.connection()?.execute(
            "UPDATE cloud_deletions SET attempts = ?2, last_error = ?3, next_attempt_at = ?4 WHERE id = ?1",
            params![id, attempts, error, now_plus_seconds_rfc3339(backoff_seconds(attempts))],
        )?;
        Ok(())
    }

    /// Drive files that are being deleted: sync must not re-import or re-link them meanwhile.
    pub fn pending_deletion_remote_ids(&self, account_id: i64) -> CoreResult<std::collections::HashSet<String>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare("SELECT remote_id FROM cloud_deletions WHERE account_id = ?1")?;
        let ids = statement.query_map([account_id], |row| row.get(0))?.collect::<Result<_, _>>()?;
        Ok(ids)
    }

    pub fn pending_deletion_count(&self, account_id: i64) -> CoreResult<u32> {
        Ok(self.db.connection()?.query_row(
            "SELECT COUNT(*) FROM cloud_deletions WHERE account_id = ?1",
            [account_id],
            |row| row.get(0),
        )?)
    }

    // ----- queue -----

    /// Queues an operation unless an equivalent one is already waiting or running.
    pub fn enqueue(
        &self,
        account_id: i64,
        snapshot_id: &str,
        direction: SyncDirection,
        bytes_total: i64,
    ) -> CoreResult<bool> {
        let dir = direction_db(direction);
        let connection = self.db.connection()?;
        let active: bool = connection.query_row(
            "SELECT EXISTS (SELECT 1 FROM sync_operations WHERE account_id = ?1 AND snapshot_id = ?2 AND direction = ?3
                            AND (status IN ('queued', 'running', 'paused') OR (status = 'failed' AND attempts < ?4)))",
            params![account_id, snapshot_id, dir, MAX_ATTEMPTS],
            |row| row.get(0),
        )?;
        if active {
            return Ok(false);
        }
        let now = now_rfc3339();
        connection.execute(
            "INSERT INTO sync_operations (account_id, snapshot_id, direction, status, bytes_total, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'queued', ?4, ?5, ?5)",
            params![account_id, snapshot_id, dir, bytes_total, now],
        )?;
        Ok(true)
    }

    /// Queued, paused and retryable failed operations whose retry time has come, oldest first.
    pub fn due(&self, account_id: i64, limit: u32) -> CoreResult<Vec<DueOperation>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, snapshot_id, direction, attempts, bytes_total, resumable_uri, bytes_done FROM sync_operations
             WHERE account_id = ?1 AND (status IN ('queued', 'paused') OR (status = 'failed' AND attempts < ?2))
               AND (next_attempt_at IS NULL OR next_attempt_at <= ?3)
             ORDER BY created_at LIMIT ?4",
        )?;
        let rows = statement
            .query_map(params![account_id, MAX_ATTEMPTS, now_rfc3339(), limit], |row| {
                Ok(DueOperation {
                    id: row.get(0)?,
                    snapshot_id: row.get(1)?,
                    direction: if row.get::<_, String>(2)? == "upload" {
                        SyncDirection::Upload
                    } else {
                        SyncDirection::Download
                    },
                    attempts: row.get(3)?,
                    bytes_total: row.get(4)?,
                    resumable_uri: row.get(5)?,
                    bytes_done: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn mark_running(&self, id: i64) -> CoreResult<()> {
        self.db.connection()?.execute(
            "UPDATE sync_operations SET status = 'running', updated_at = ?2 WHERE id = ?1",
            params![id, now_rfc3339()],
        )?;
        Ok(())
    }

    pub fn save_progress(&self, id: i64, bytes_done: i64, resumable_uri: Option<&str>) -> CoreResult<()> {
        self.db.connection()?.execute(
            "UPDATE sync_operations SET bytes_done = ?2, resumable_uri = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, bytes_done, resumable_uri, now_rfc3339()],
        )?;
        Ok(())
    }

    pub fn mark_done(&self, id: i64) -> CoreResult<()> {
        self.db.connection()?.execute(
            "UPDATE sync_operations SET status = 'done', last_error = NULL, resumable_uri = NULL, updated_at = ?2 WHERE id = ?1",
            params![id, now_rfc3339()],
        )?;
        Ok(())
    }

    pub fn mark_cancelled(&self, id: i64, reason: &str) -> CoreResult<()> {
        self.db.connection()?.execute(
            "UPDATE sync_operations SET status = 'cancelled', last_error = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, reason, now_rfc3339()],
        )?;
        Ok(())
    }

    /// Records a failure; `permanent` skips the automatic retries (the user can retry by hand).
    pub fn mark_failed(&self, id: i64, attempts: i64, error: &str, permanent: bool) -> CoreResult<()> {
        let attempts = if permanent { MAX_ATTEMPTS } else { attempts };
        self.db.connection()?.execute(
            "UPDATE sync_operations SET status = 'failed', attempts = ?2, last_error = ?3, next_attempt_at = ?4, updated_at = ?5
             WHERE id = ?1",
            params![id, attempts, error, now_plus_seconds_rfc3339(backoff_seconds(attempts)), now_rfc3339()],
        )?;
        Ok(())
    }

    /// Operations of an account that stop waiting for a valid sign-in.
    pub fn pause_all(&self, account_id: i64) -> CoreResult<()> {
        self.db.connection()?.execute(
            "UPDATE sync_operations SET status = 'paused', updated_at = ?2
             WHERE account_id = ?1 AND status IN ('queued', 'running', 'failed')",
            params![account_id, now_rfc3339()],
        )?;
        Ok(())
    }

    /// Gives every failed operation a fresh start ("Retry" button).
    pub fn retry_failed(&self, account_id: i64) -> CoreResult<usize> {
        self.db.connection()?.execute(
            "UPDATE cloud_deletions SET attempts = 0, next_attempt_at = NULL, last_error = NULL
             WHERE account_id = ?1 AND attempts > 0",
            [account_id],
        )?;
        Ok(self.db.connection()?.execute(
            "UPDATE sync_operations SET status = 'queued', attempts = 0, next_attempt_at = NULL, last_error = NULL, updated_at = ?2
             WHERE account_id = ?1 AND status = 'failed'",
            params![account_id, now_rfc3339()],
        )?)
    }

    /// Operations still to do, newest first (for the Cloud page).
    pub fn pending(&self, account_id: i64) -> CoreResult<Vec<SyncOperationInfo>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT o.id, o.snapshot_id, g.title, s.name, o.direction, o.status, o.attempts, o.bytes_done, o.bytes_total,
                    o.last_error, o.next_attempt_at
             FROM sync_operations o JOIN snapshots s ON s.id = o.snapshot_id JOIN games g ON g.id = s.game_id
             WHERE o.account_id = ?1 AND o.status IN ('queued', 'running', 'failed', 'paused')
             ORDER BY o.created_at DESC LIMIT 200",
        )?;
        let rows = statement
            .query_map([account_id], |row| {
                Ok(SyncOperationInfo {
                    id: row.get(0)?,
                    snapshot_id: row.get(1)?,
                    game_title: row.get(2)?,
                    snapshot_name: row.get(3)?,
                    direction: if row.get::<_, String>(4)? == "upload" {
                        SyncDirection::Upload
                    } else {
                        SyncDirection::Download
                    },
                    status: match row.get::<_, String>(5)?.as_str() {
                        "running" => SyncStatus::Running,
                        "failed" => SyncStatus::Failed,
                        "paused" => SyncStatus::Paused,
                        _ => SyncStatus::Queued,
                    },
                    attempts: row.get(6)?,
                    bytes_done: row.get(7)?,
                    bytes_total: row.get(8)?,
                    last_error: row.get(9)?,
                    next_attempt_at: row.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn counts(&self, account_id: i64) -> CoreResult<(u32, u32, u32)> {
        let connection = self.db.connection()?;
        let count = |status: &str| -> CoreResult<u32> {
            Ok(connection.query_row(
                "SELECT COUNT(*) FROM sync_operations WHERE account_id = ?1 AND status = ?2",
                params![account_id, status],
                |row| row.get(0),
            )?)
        };
        // A paused operation waits like a queued one.
        Ok((count("queued")? + count("paused")?, count("running")?, count("failed")?))
    }

    /// Operations left in `running` by a crash are put back in the queue at start-up.
    pub fn requeue_interrupted(&self) -> CoreResult<usize> {
        Ok(self
            .db
            .connection()?
            .execute("UPDATE sync_operations SET status = 'queued' WHERE status = 'running'", [])?)
    }

    // ----- conflicts -----

    pub fn conflict_exists(&self, local_id: &str, remote_id: &str) -> CoreResult<bool> {
        Ok(self.db.connection()?.query_row(
            "SELECT EXISTS (SELECT 1 FROM sync_conflicts WHERE local_snapshot_id = ?1 AND remote_snapshot_id = ?2)",
            params![local_id, remote_id],
            |row| row.get(0),
        )?)
    }

    pub fn insert_conflict(&self, game_id: i64, local_id: &str, remote_id: &str) -> CoreResult<i64> {
        let connection = self.db.connection()?;
        connection.execute(
            "INSERT INTO sync_conflicts (game_id, local_snapshot_id, remote_snapshot_id, detected_at) VALUES (?1, ?2, ?3, ?4)",
            params![game_id, local_id, remote_id, now_rfc3339()],
        )?;
        Ok(connection.last_insert_rowid())
    }

    pub fn open_conflicts(&self) -> CoreResult<Vec<OpenConflict>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, game_id, local_snapshot_id, remote_snapshot_id, detected_at FROM sync_conflicts
             WHERE resolved_at IS NULL ORDER BY detected_at",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(OpenConflict {
                    id: row.get(0)?,
                    game_id: row.get(1)?,
                    local_id: row.get(2)?,
                    remote_id: row.get(3)?,
                    detected_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn conflict(&self, id: i64) -> CoreResult<(i64, String, String)> {
        self.db
            .connection()?
            .query_row(
                "SELECT game_id, local_snapshot_id, remote_snapshot_id FROM sync_conflicts WHERE id = ?1 AND resolved_at IS NULL",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound(format!("open conflict {id}")))
    }

    pub fn resolve_conflict(&self, id: i64, resolution: ConflictResolution) -> CoreResult<()> {
        self.db.connection()?.execute(
            "UPDATE sync_conflicts SET resolution = ?2, resolved_at = ?3 WHERE id = ?1",
            params![id, resolution.as_db(), now_rfc3339()],
        )?;
        Ok(())
    }

    // ----- misc -----

    pub fn last_sync(&self) -> CoreResult<Option<String>> {
        Ok(self
            .db
            .connection()?
            .query_row("SELECT value FROM settings WHERE key = ?1", [KEY_LAST_SYNC], |row| row.get::<_, String>(0))
            .optional()?
            .and_then(|json| serde_json::from_str(&json).ok()))
    }

    pub fn set_last_sync(&self) -> CoreResult<()> {
        let value = serde_json::to_string(&now_rfc3339())
            .map_err(|source| CoreError::Serialization { key: KEY_LAST_SYNC.into(), source })?;
        self.db.connection()?.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![KEY_LAST_SYNC, value, now_rfc3339()],
        )?;
        Ok(())
    }
}

fn direction_db(direction: SyncDirection) -> &'static str {
    match direction {
        SyncDirection::Upload => "upload",
        SyncDirection::Download => "download",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_and_is_capped() {
        assert_eq!(backoff_seconds(1), 30);
        assert_eq!(backoff_seconds(2), 60);
        assert_eq!(backoff_seconds(3), 120);
        assert_eq!(backoff_seconds(8), 3600);
        assert_eq!(backoff_seconds(50), 3600);
    }
}
