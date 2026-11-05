//! The sync engine: upload queue with backoff and resume, import of what other PCs uploaded,
//! lineage-based conflict detection and its resolution.
//!
//! Safety rules: a local archive is checksum-verified before upload, a downloaded archive is
//! checksum-verified before it is used, nothing in Drive is overwritten, a Drive file is deleted
//! only when retention or the user asked for it (and only after its identity is checked), and a
//! divergent history is never merged silently: it becomes a conflict the user resolves.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use crate::clock::now_rfc3339;
use crate::cloud::drive::UploadSession;
use crate::cloud::model::{AccountStatus, ConflictInfo, ConflictResolution, ConflictSide, SyncDirection, SyncReport};
use crate::cloud::remote::{RemoteMeta, RemoteStore, UploadRequest, partial_path};
use crate::cloud::repository::{AccountRow, CloudRepository, DueOperation, OpenConflict};
use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::snapshots::sha256_file;
use crate::snapshots::{SnapshotEngine, SnapshotKind, SnapshotRepository, SnapshotStatus, SnapshotStorage};

/// Operations processed per run, so one slow queue never blocks the worker for long.
const BATCH: u32 = 20;

/// Safety copies and conflict copies stay on this PC.
pub fn is_uploadable(kind: SnapshotKind) -> bool {
    !matches!(kind, SnapshotKind::Safety | SnapshotKind::ConflictCopy)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueueReport {
    pub uploaded: u32,
    pub downloaded: u32,
    pub failed: u32,
    /// Drive copies removed (retention or an explicit "delete from Drive").
    pub deleted: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReconcileReport {
    pub imported: u32,
    pub linked: u32,
    pub marked_missing: u32,
    pub skipped_unknown_games: u32,
}

pub struct CloudSync<'a> {
    db: &'a Database,
    remote: &'a dyn RemoteStore,
    root: PathBuf,
}

struct SnapshotRow {
    meta: RemoteMeta,
    archive_path: Option<PathBuf>,
    archive_size: i64,
    kind: SnapshotKind,
}

impl<'a> CloudSync<'a> {
    pub fn new(db: &'a Database, remote: &'a dyn RemoteStore, root: PathBuf) -> Self {
        Self { db, remote, root }
    }

    fn repository(&self) -> CloudRepository<'a> {
        CloudRepository::new(self.db)
    }

    fn account(&self) -> CoreResult<AccountRow> {
        self.repository().account()?.ok_or_else(|| CoreError::Validation("Google Drive is not connected".into()))
    }

    fn connected_account(&self) -> CoreResult<AccountRow> {
        let account = self.account()?;
        if account.status != AccountStatus::Connected {
            return Err(CoreError::Auth("sign in to Google Drive again".into()));
        }
        Ok(account)
    }

    // ---------------------------------------------------------------- queue

    /// Queues an upload for a snapshot. `false` when nothing was queued (not connected, not an
    /// uploadable kind, no local archive, already in Drive or already queued).
    pub fn queue_upload(&self, snapshot_id: &str) -> CoreResult<bool> {
        let Some(account) = self.repository().account()? else { return Ok(false) };
        let row = self.load_snapshot(snapshot_id)?;
        if !is_uploadable(row.kind) || row.archive_path.is_none() {
            return Ok(false);
        }
        // "On this PC only" snapshots never leave the PC.
        if SnapshotRepository::new(self.db).storage(snapshot_id)? == SnapshotStorage::Local {
            return Ok(false);
        }
        if self.repository().remote_id(account.id, snapshot_id)?.is_some() {
            return Ok(false);
        }
        self.repository().enqueue(account.id, snapshot_id, SyncDirection::Upload, row.archive_size)
    }

    /// Queues every local snapshot that is not in Drive yet (first connection, missed uploads).
    pub fn queue_missing_uploads(&self) -> CoreResult<u32> {
        let ids: Vec<String> = {
            let connection = self.db.connection()?;
            let mut statement = connection.prepare(
                "SELECT id FROM snapshots WHERE archive_path IS NOT NULL AND status = 'ready'
                 AND kind NOT IN ('safety', 'conflict_copy') AND storage <> 'local' ORDER BY created_at",
            )?;
            statement.query_map([], |row| row.get(0))?.collect::<Result<_, _>>()?
        };
        let mut queued = 0;
        for id in ids {
            if self.queue_upload(&id)? {
                queued += 1;
            }
        }
        Ok(queued)
    }

    /// Runs the operations that are due. Authentication failures pause the queue and surface as
    /// an error; every other failure is recorded on its operation and retried with backoff.
    pub fn run_queue(&self) -> CoreResult<QueueReport> {
        let account = self.connected_account()?;
        let repository = self.repository();
        let mut report = QueueReport::default();
        self.run_deletions(&account, &mut report)?;
        for operation in repository.due(account.id, BATCH)? {
            repository.mark_running(operation.id)?;
            let outcome = match operation.direction {
                SyncDirection::Upload => self.run_upload(&account, &operation),
                SyncDirection::Download => self.download_now(&operation.snapshot_id),
            };
            match outcome {
                Ok(()) => {
                    repository.mark_done(operation.id)?;
                    match operation.direction {
                        SyncDirection::Upload => report.uploaded += 1,
                        SyncDirection::Download => report.downloaded += 1,
                    }
                }
                Err(error @ CoreError::Auth(_)) => {
                    repository.set_status(account.id, AccountStatus::NeedsReauth)?;
                    repository.pause_all(account.id)?;
                    return Err(error);
                }
                Err(error) => {
                    // Quota, missing local file and integrity problems cannot fix themselves.
                    let permanent =
                        matches!(error, CoreError::Validation(_) | CoreError::Integrity(_) | CoreError::NotFound(_));
                    tracing::warn!(operation = operation.id, %error, permanent, "sync operation failed");
                    repository.mark_failed(operation.id, operation.attempts + 1, &error.to_string(), permanent)?;
                    report.failed += 1;
                }
            }
        }
        Ok(report)
    }

    /// Deletes the Drive copies handed over by retention. A file that is already gone counts as
    /// done; a file that is not the recorded snapshot is never touched (the entry is parked).
    fn run_deletions(&self, account: &AccountRow, report: &mut QueueReport) -> CoreResult<()> {
        let repository = self.repository();
        for deletion in repository.due_deletions(account.id, BATCH)? {
            match self.remote.delete_snapshot(&deletion.remote_id, &deletion.snapshot_id) {
                Ok(removed) => {
                    repository.finish_deletion(deletion.id)?;
                    report.deleted += 1;
                    tracing::info!(snapshot = %deletion.snapshot_id, removed, "Drive copy deleted");
                }
                Err(error @ CoreError::Auth(_)) => {
                    repository.set_status(account.id, AccountStatus::NeedsReauth)?;
                    repository.pause_all(account.id)?;
                    return Err(error);
                }
                Err(error) => {
                    let permanent = matches!(error, CoreError::Validation(_));
                    tracing::warn!(snapshot = %deletion.snapshot_id, %error, permanent, "could not delete a Drive copy");
                    repository.fail_deletion(deletion.id, deletion.attempts + 1, &error.to_string(), permanent)?;
                    report.failed += 1;
                }
            }
        }
        Ok(())
    }

    fn run_upload(&self, account: &AccountRow, operation: &DueOperation) -> CoreResult<()> {
        let repository = self.repository();
        let row = self.load_snapshot(&operation.snapshot_id)?;
        let path = row
            .archive_path
            .filter(|path| path.is_file())
            .ok_or_else(|| CoreError::NotFound("the local archive is missing".into()))?;
        // Never push a damaged archive: the checksum recorded at creation must still match.
        if sha256_file(&path)? != row.meta.sha256 {
            return Err(CoreError::Integrity("the local archive changed since it was created".into()));
        }
        let size = i64::try_from(std::fs::metadata(&path)?.len()).unwrap_or(i64::MAX);
        let quota = self.remote.quota()?;
        if let Some(limit) = quota.limit
            && quota.used + size > limit
        {
            return Err(CoreError::Validation("your Google Drive storage is full".into()));
        }

        let mut session =
            UploadSession { resumable_uri: operation.resumable_uri.clone(), bytes_done: operation.bytes_done };
        let meta = RemoteMeta { parent_id: self.uploadable_parent(&operation.snapshot_id)?, ..row.meta };
        let operation_id = operation.id;
        let object =
            self.remote.upload(&UploadRequest { meta: &meta, local_path: &path }, &mut session, &mut |state| {
                if let Err(error) =
                    repository.save_progress(operation_id, state.bytes_done, state.resumable_uri.as_deref())
                {
                    tracing::warn!(%error, "could not persist upload progress");
                }
            })?;
        if object.size != size {
            return Err(CoreError::Integrity(format!("Drive stored {} bytes instead of {size}", object.size)));
        }
        repository.record_object(
            account.id,
            &operation.snapshot_id,
            &object.remote_id,
            &object.path,
            size,
            &meta.sha256,
        )?;
        tracing::info!(snapshot = %operation.snapshot_id, "snapshot uploaded to Drive");
        self.drop_local_copy_if_cloud_only(&operation.snapshot_id);
        Ok(())
    }

    /// "In Google Drive only" snapshots lose their local archive, but only now that Drive has
    /// confirmed the upload (size checked and recorded): until then the local copy is the only one.
    /// A failure here is logged, not returned: the upload itself succeeded and must not be repeated.
    fn drop_local_copy_if_cloud_only(&self, snapshot_id: &str) {
        let outcome = SnapshotRepository::new(self.db).storage(snapshot_id).and_then(|storage| {
            if storage == SnapshotStorage::Cloud {
                SnapshotEngine::new(self.db, self.root.clone()).delete(snapshot_id)
            } else {
                Ok(())
            }
        });
        if let Err(error) = outcome {
            tracing::warn!(snapshot = %snapshot_id, %error, "could not remove the local archive of a Drive-only snapshot");
        }
    }

    /// Nearest ancestor that will exist in Drive, skipping safety and conflict copies that stay local.
    fn uploadable_parent(&self, snapshot_id: &str) -> CoreResult<Option<String>> {
        let connection = self.db.connection()?;
        let mut current = snapshot_id.to_owned();
        for _ in 0..10_000 {
            let parent: Option<(Option<String>,)> = connection
                .query_row("SELECT parent_snapshot_id FROM snapshots WHERE id = ?1", [&current], |row| {
                    Ok((row.get(0)?,))
                })
                .optional()?;
            let Some((Some(parent),)) = parent else { return Ok(None) };
            let kind: String =
                connection.query_row("SELECT kind FROM snapshots WHERE id = ?1", [&parent], |row| row.get(0))?;
            if is_uploadable(SnapshotKind::from_db(&kind)) {
                return Ok(Some(parent));
            }
            current = parent;
        }
        Ok(None)
    }

    fn load_snapshot(&self, id: &str) -> CoreResult<SnapshotRow> {
        let connection = self.db.connection()?;
        connection
            .query_row(
                "SELECT s.id, s.parent_snapshot_id, g.slug, g.title, s.kind, s.name, s.description, s.archive_sha256,
                        s.created_at, s.device_id, d.name, s.total_files, s.total_bytes, s.playtime_seconds,
                        s.archive_path, s.archive_size
                 FROM snapshots s JOIN games g ON g.id = s.game_id JOIN devices d ON d.id = s.device_id WHERE s.id = ?1",
                [id],
                |row| {
                    let kind: String = row.get(4)?;
                    Ok(SnapshotRow {
                        kind: SnapshotKind::from_db(&kind),
                        archive_path: row.get::<_, Option<String>>(14)?.map(PathBuf::from),
                        archive_size: row.get(15)?,
                        meta: RemoteMeta {
                            snapshot_id: row.get(0)?,
                            parent_id: row.get(1)?,
                            game_slug: row.get(2)?,
                            game_title: row.get(3)?,
                            kind,
                            name: row.get(5)?,
                            description: row.get(6)?,
                            sha256: row.get(7)?,
                            created_at: row.get(8)?,
                            device_id: row.get(9)?,
                            device_name: row.get(10)?,
                            total_files: row.get(11)?,
                            total_bytes: row.get(12)?,
                            playtime_seconds: row.get(13)?,
                        },
                    })
                },
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound(format!("snapshot {id}")))
    }

    // ------------------------------------------------------------- download

    /// Downloads a cloud-only snapshot, verifies its checksum, and attaches it. A snapshot that is
    /// already on disk is left alone.
    pub fn download_now(&self, snapshot_id: &str) -> CoreResult<()> {
        let account = self.account()?;
        let snapshots = SnapshotRepository::new(self.db);
        let snapshot = snapshots.get(snapshot_id)?;
        if snapshot.archive_path.as_deref().is_some_and(|path| Path::new(path).is_file()) {
            return Ok(());
        }
        let remote_id = self
            .repository()
            .remote_id(account.id, snapshot_id)?
            .ok_or_else(|| CoreError::NotFound("this snapshot is not in Google Drive".into()))?;
        let engine = SnapshotEngine::new(self.db, self.root.clone());
        let destination = engine.archive_destination(snapshot.game_id, snapshot_id)?;
        if let Some(folder) = destination.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let partial = partial_path(&destination);
        let result = self.remote.download(&remote_id, &partial).and_then(|()| {
            if sha256_file(&partial)? == snapshot.archive_sha256 {
                Ok(())
            } else {
                Err(CoreError::Integrity("the downloaded archive does not match its checksum".into()))
            }
        });
        if let Err(error) = result {
            // A bad or partial download is never kept.
            if let Err(cleanup) = std::fs::remove_file(&partial)
                && cleanup.kind() != std::io::ErrorKind::NotFound
            {
                tracing::warn!(%cleanup, "could not remove a partial download");
            }
            if matches!(error, CoreError::Integrity(_)) {
                self.repository().set_object_status(account.id, snapshot_id, "integrity_mismatch")?;
            }
            return Err(error);
        }
        std::fs::rename(&partial, &destination)?;
        let size = i64::try_from(std::fs::metadata(&destination)?.len()).unwrap_or(i64::MAX);
        snapshots.attach_archive(snapshot_id, &destination, size)?;
        tracing::info!(snapshot = %snapshot_id, "snapshot downloaded from Drive");
        Ok(())
    }

    // ------------------------------------------------------------ reconcile

    /// Lists Drive, imports snapshots this PC does not know, links ones it already has, and
    /// marks copies that vanished from Drive.
    pub fn reconcile(&self) -> CoreResult<ReconcileReport> {
        let account = self.connected_account()?;
        let repository = self.repository();
        let mut remote = self.remote.list_snapshots()?;
        remote.sort_by(|a, b| a.meta.created_at.cmp(&b.meta.created_at));
        let mut report = ReconcileReport::default();
        let remote_ids: HashSet<&str> = remote.iter().map(|snapshot| snapshot.meta.snapshot_id.as_str()).collect();
        // Copies waiting to be deleted must not come back as "new" snapshots from Drive.
        let being_deleted = repository.pending_deletion_remote_ids(account.id)?;

        for snapshot in &remote {
            if being_deleted.contains(&snapshot.remote_id) {
                continue;
            }
            let meta = &snapshot.meta;
            let known = {
                let connection = self.db.connection()?;
                connection.query_row(
                    "SELECT EXISTS (SELECT 1 FROM snapshots WHERE id = ?1)",
                    [&meta.snapshot_id],
                    |row| row.get::<_, bool>(0),
                )?
            };
            if known {
                if repository.remote_id(account.id, &meta.snapshot_id)?.is_none() {
                    repository.record_object(
                        account.id,
                        &meta.snapshot_id,
                        &snapshot.remote_id,
                        &format!("L8Play/{}/{}.zip", meta.game_slug, meta.snapshot_id),
                        snapshot.size,
                        &meta.sha256,
                    )?;
                    report.linked += 1;
                }
                continue;
            }
            if self.import(account.id, snapshot)? {
                report.imported += 1;
            } else {
                report.skipped_unknown_games += 1;
            }
        }

        for (snapshot_id, _) in repository.present_objects(account.id)? {
            if !remote_ids.contains(snapshot_id.as_str()) {
                repository.set_object_status(account.id, &snapshot_id, "missing")?;
                report.marked_missing += 1;
            }
        }
        Ok(report)
    }

    /// Records a snapshot that exists only in Drive. `false` when its game is not in the library.
    fn import(&self, account_id: i64, snapshot: &crate::cloud::remote::RemoteSnapshot) -> CoreResult<bool> {
        let meta = &snapshot.meta;
        let mut connection = self.db.connection()?;
        let game_id: Option<i64> = connection
            .query_row("SELECT id FROM games WHERE slug = ?1", [&meta.game_slug], |row| row.get(0))
            .optional()?;
        let Some(game_id) = game_id else { return Ok(false) };
        let tx = connection.transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO devices (id, name, is_current) VALUES (?1, ?2, 0)",
            params![meta.device_id, meta.device_name],
        )?;
        // A parent this PC has never seen would break the foreign key; lineage then starts here.
        let parent = match &meta.parent_id {
            Some(id)
                if tx.query_row("SELECT EXISTS (SELECT 1 FROM snapshots WHERE id = ?1)", [id], |row| {
                    row.get::<_, bool>(0)
                })? =>
            {
                Some(id.clone())
            }
            _ => None,
        };
        tx.execute(
            "INSERT INTO snapshots (id, game_id, name, description, kind, status, device_id, parent_snapshot_id, archive_path,
                                    archive_size, archive_sha256, total_files, total_bytes, playtime_seconds, is_pinned, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'ready', ?6, ?7, NULL, ?8, ?9, ?10, ?11, ?12, 0, ?13)",
            params![
                meta.snapshot_id,
                game_id,
                meta.name,
                meta.description,
                SnapshotKind::from_db(&meta.kind).as_db(),
                meta.device_id,
                parent,
                snapshot.size,
                meta.sha256,
                meta.total_files,
                meta.total_bytes,
                meta.playtime_seconds,
                meta.created_at,
            ],
        )?;
        tx.execute(
            "INSERT INTO cloud_objects (account_id, snapshot_id, remote_id, remote_path, size_bytes, sha256, uploaded_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'present')",
            params![
                account_id,
                meta.snapshot_id,
                snapshot.remote_id,
                format!("L8Play/{}/{}.zip", meta.game_slug, meta.snapshot_id),
                snapshot.size,
                meta.sha256,
                now_rfc3339()
            ],
        )?;
        tx.commit()?;
        Ok(true)
    }

    // ------------------------------------------------------------ conflicts

    /// Looks for games whose history split between this PC and another device. Returns how many
    /// new conflicts were opened.
    pub fn detect_conflicts(&self) -> CoreResult<u32> {
        let current = SnapshotRepository::new(self.db).current_device()?;
        let games: Vec<i64> = {
            let connection = self.db.connection()?;
            let mut statement = connection.prepare(
                "SELECT game_id FROM snapshots WHERE status <> 'deleted' GROUP BY game_id HAVING COUNT(DISTINCT device_id) > 1",
            )?;
            statement.query_map([], |row| row.get(0))?.collect::<Result<_, _>>()?
        };
        let repository = self.repository();
        let mut opened = 0;
        for game_id in games {
            let Some((local, remote)) = self.diverged_heads(game_id, &current)? else { continue };
            if !repository.conflict_exists(&local, &remote)? {
                repository.insert_conflict(game_id, &local, &remote)?;
                opened += 1;
            }
        }
        Ok(opened)
    }

    /// (newest local-device leaf, newest other-device leaf) when both exist.
    fn diverged_heads(&self, game_id: i64, current_device: &str) -> CoreResult<Option<(String, String)>> {
        struct Node {
            parent: Option<String>,
            device: String,
            kind: SnapshotKind,
            created_at: String,
        }
        let nodes: HashMap<String, Node> = {
            let connection = self.db.connection()?;
            let mut statement = connection
                .prepare("SELECT id, parent_snapshot_id, device_id, kind, created_at FROM snapshots WHERE game_id = ?1 AND status <> 'deleted'")?;
            statement
                .query_map([game_id], |row| {
                    let kind: String = row.get(3)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        Node {
                            parent: row.get(1)?,
                            device: row.get(2)?,
                            kind: SnapshotKind::from_db(&kind),
                            created_at: row.get(4)?,
                        },
                    ))
                })?
                .collect::<Result<_, _>>()?
        };
        // Safety and conflict copies are bookkeeping: they never count as a head of the history.
        let eligible = |node: &Node| is_uploadable(node.kind);
        let mut has_descendant: HashSet<&str> = HashSet::new();
        for (id, node) in &nodes {
            if !eligible(node) {
                continue;
            }
            // Walk up through every ancestor, safety copies included, so a chain A ← safety ← B still
            // makes A an ancestor of B. The step bound protects against a corrupted cycle.
            let mut cursor = node.parent.as_deref();
            for _ in 0..nodes.len() {
                let Some(parent_id) = cursor else { break };
                if parent_id == id {
                    break;
                }
                has_descendant.insert(parent_id);
                cursor = nodes.get(parent_id).and_then(|parent| parent.parent.as_deref());
            }
        }
        let newest = |local: bool| {
            nodes
                .iter()
                .filter(|(id, node)| {
                    eligible(node) && !has_descendant.contains(id.as_str()) && (node.device == current_device) == local
                })
                .max_by(|a, b| a.1.created_at.cmp(&b.1.created_at))
                .map(|(id, _)| id.clone())
        };
        Ok(newest(true).zip(newest(false)))
    }

    pub fn conflicts(&self) -> CoreResult<Vec<ConflictInfo>> {
        let snapshots = SnapshotRepository::new(self.db);
        let side = |id: &str| -> CoreResult<ConflictSide> {
            let snapshot = snapshots.get(id)?;
            Ok(ConflictSide {
                snapshot_id: snapshot.id,
                name: snapshot.name,
                created_at: snapshot.created_at,
                device_name: snapshot.device_name,
                total_files: snapshot.total_files,
                total_bytes: snapshot.total_bytes,
                playtime_seconds: snapshot.playtime_seconds,
                local_copy: snapshot.status != SnapshotStatus::MissingLocal,
            })
        };
        let mut result = Vec::new();
        for OpenConflict { id, game_id, local_id: local, remote_id: remote, detected_at } in
            self.repository().open_conflicts()?
        {
            let game_title: String =
                self.db
                    .connection()?
                    .query_row("SELECT title FROM games WHERE id = ?1", [game_id], |row| row.get(0))?;
            result.push(ConflictInfo {
                id,
                game_id,
                game_title,
                detected_at,
                local: side(&local)?,
                remote: side(&remote)?,
            });
        }
        Ok(result)
    }

    /// Applies the user's choice. Nothing is deleted: the losing side becomes a conflict copy.
    pub fn resolve_conflict(
        &self,
        id: i64,
        resolution: ConflictResolution,
        engine: &SnapshotEngine<'_>,
    ) -> CoreResult<()> {
        let repository = self.repository();
        let (_, local_id, remote_id) = repository.conflict(id)?;
        let snapshots = SnapshotRepository::new(self.db);
        match resolution {
            ConflictResolution::UseLocal => snapshots.set_kind(&remote_id, SnapshotKind::ConflictCopy)?,
            ConflictResolution::KeepBoth => {
                self.download_now(&remote_id)?;
                snapshots.set_kind(&remote_id, SnapshotKind::ConflictCopy)?;
            }
            ConflictResolution::UseRemote => {
                self.download_now(&remote_id)?;
                engine.restore(&remote_id)?;
                snapshots.set_kind(&local_id, SnapshotKind::ConflictCopy)?;
            }
        }
        repository.resolve_conflict(id, resolution)
    }

    // ---------------------------------------------------------------- all

    /// One full pass: import from Drive, queue what is missing, run the queue, look for conflicts.
    pub fn sync_all(&self) -> CoreResult<SyncReport> {
        self.connected_account()?;
        let reconciled = self.reconcile()?;
        self.queue_missing_uploads()?;
        let queue = self.run_queue()?;
        let new_conflicts = self.detect_conflicts()?;
        self.repository().set_last_sync()?;
        Ok(SyncReport {
            uploaded: queue.uploaded,
            downloaded: queue.downloaded,
            failed: queue.failed,
            imported: reconciled.imported,
            new_conflicts,
            skipped_unknown_games: reconciled.skipped_unknown_games,
        })
    }
}
