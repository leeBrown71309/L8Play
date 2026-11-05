//! The snapshot engine: create, verify, restore, delete, retention.
//!
//! Guarantees (see docs/architecture/02-system-architecture.md):
//! - Original saves are only ever opened for reading while a snapshot is made.
//! - An archive is written to a temporary file, verified entry by entry, then renamed into place;
//!   only then is it recorded. A failure leaves no partial snapshot.
//! - A restore first protects the current save with a safety snapshot (and aborts if that fails),
//!   stages the files on the same volume, swaps them in with renames, verifies them, and rolls
//!   everything back on any error.
//! - Deletions are limited to files and folders this engine created.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::clock::now_rfc3339;
use crate::cloud::CloudRepository;
use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::library::LibraryRepository;
use crate::saves::{LocationKind, SaveLocationRepository, normalize_path};
use crate::snapshots::archive::{ArchiveManifest, extract_file, open_archive, verify_archive, write_archive};
use crate::snapshots::files::{collect_files, safe_relative, sha256_file};
use crate::snapshots::model::{
    CreateOutcome, CurrentSaveState, RestoreReport, SnapshotKind, SnapshotStatus, SnapshotStorage, SnapshotSummary,
};
use crate::snapshots::repository::{NewSnapshot, SnapshotRepository};

/// What to snapshot and how.
#[derive(Debug, Clone)]
pub struct CreateRequest {
    pub game_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub kind: SnapshotKind,
    pub pinned: bool,
    /// Skip when the save is byte-identical to the latest snapshot (automatic snapshots).
    pub skip_if_unchanged: bool,
    /// Where the snapshot should live (this PC, Google Drive, or both).
    pub storage: SnapshotStorage,
}

/// How many automatic snapshots each game keeps, on this PC and in Google Drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionPolicy {
    /// Newest automatic snapshots whose archive stays on this PC.
    pub keep_local: u32,
    /// Newest automatic snapshots whose copy stays in Drive; `0` means the same as `keep_local`.
    pub keep_cloud: u32,
}

impl RetentionPolicy {
    /// Drive never keeps fewer than this PC: a copy dropped from Drive while its archive is still
    /// local would simply be uploaded again.
    pub fn effective_keep_cloud(self) -> u32 {
        self.keep_cloud.max(self.keep_local)
    }
}

/// What one retention pass removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetentionReport {
    /// Archives deleted from this PC.
    pub removed_local: u32,
    /// Drive copies handed to the deletion queue (they leave Drive on the next sync pass).
    pub removed_cloud: u32,
}

pub struct SnapshotEngine<'db> {
    db: &'db Database,
    root: PathBuf,
}

impl<'db> SnapshotEngine<'db> {
    /// `root` is the snapshot folder (one sub-folder per game).
    pub fn new(db: &'db Database, root: PathBuf) -> Self {
        Self { db, root }
    }

    pub fn create(&self, request: &CreateRequest) -> CoreResult<CreateOutcome> {
        let library = LibraryRepository::new(self.db);
        let game = library
            .summary(request.game_id)?
            .ok_or_else(|| CoreError::NotFound(format!("game {}", request.game_id)))?;
        let locations = SaveLocationRepository::new(self.db).list(request.game_id)?;
        if locations.is_empty() {
            return Err(CoreError::Validation("this game has no save location yet".into()));
        }
        let files = collect_files(&locations)?;
        if files.is_empty() {
            return Ok(CreateOutcome::NothingToBackUp);
        }
        let repository = SnapshotRepository::new(self.db);
        if request.skip_if_unchanged
            && let Some(latest) = repository.latest(request.game_id)?
            && same_files(
                &repository.file_hashes(&latest.id)?,
                &files.iter().map(|file| file.absolute.clone()).collect::<Vec<_>>(),
            )?
        {
            return Ok(CreateOutcome::Unchanged { latest_snapshot_id: latest.id });
        }

        let id = uuid::Uuid::now_v7().to_string();
        let folder = self.root.join(game_folder(self.db, request.game_id)?);
        std::fs::create_dir_all(&folder)?;
        let partial = folder.join(format!(".{id}.zip.partial"));
        let final_path = folder.join(format!("{id}.zip"));
        let guard = RemoveOnDrop(Some(partial.clone()));

        let created_at = now_rfc3339();
        let manifest = write_archive(&partial, &id, &game.title, &created_at, &locations, &files)?;
        verify_archive(&partial)?;
        let archive_sha256 = sha256_file(&partial)?;
        let archive_size = i64::try_from(std::fs::metadata(&partial)?.len()).unwrap_or(i64::MAX);
        std::fs::rename(&partial, &final_path)?;
        guard.disarm();

        let recorded = repository.insert(&NewSnapshot {
            id: &id,
            game_id: request.game_id,
            name: request.name.trim(),
            description: request.description.as_deref().map(str::trim).filter(|text| !text.is_empty()),
            kind: request.kind,
            pinned: request.pinned,
            storage: request.storage,
            archive_path: &final_path,
            archive_size,
            archive_sha256: &archive_sha256,
            manifest: &manifest,
        });
        if let Err(error) = recorded {
            // Keep the archive on disk (it is valid) but do not leave an unrecorded orphan silently.
            tracing::error!(%error, archive = %final_path.display(), "snapshot archive written but not recorded");
            return Err(error);
        }
        tracing::info!(snapshot = %id, game_id = request.game_id, files = manifest.files.len(), "snapshot created");
        Ok(CreateOutcome::Created(Box::new(repository.get(&id)?)))
    }

    /// Re-checks the archive checksum and every entry. Marks the snapshot corrupt on mismatch.
    pub fn verify(&self, id: &str) -> CoreResult<SnapshotSummary> {
        let repository = SnapshotRepository::new(self.db);
        let snapshot = repository.get(id)?;
        let path = self.archive_path(&snapshot)?;
        match check_archive(&path, &snapshot.archive_sha256) {
            Ok(_) => repository.mark_verified(id)?,
            Err(error @ CoreError::Integrity(_)) => {
                repository.set_status(id, "corrupt")?;
                return Err(error);
            }
            Err(error) => return Err(error),
        }
        repository.get(id)
    }

    pub fn restore(&self, id: &str) -> CoreResult<RestoreReport> {
        let repository = SnapshotRepository::new(self.db);
        let snapshot = repository.get(id)?;
        let path = self.archive_path(&snapshot)?;
        let manifest = match check_archive(&path, &snapshot.archive_sha256) {
            Ok(manifest) => manifest,
            Err(error @ CoreError::Integrity(_)) => {
                repository.set_status(id, "corrupt")?;
                return Err(error);
            }
            Err(error) => return Err(error),
        };

        // Only locations still registered for this game may be written to.
        let locations = SaveLocationRepository::new(self.db);
        for location in &manifest.locations {
            if !locations.is_registered(snapshot.game_id, Path::new(&location.path))? {
                return Err(CoreError::Validation(format!(
                    "{} is no longer a save location of this game",
                    location.path
                )));
            }
        }

        // Protect the current save first. Identical to an existing snapshot = already protected.
        let safety = self.create(&CreateRequest {
            game_id: snapshot.game_id,
            name: String::new(),
            description: None,
            kind: SnapshotKind::Safety,
            pinned: false,
            skip_if_unchanged: true,
            storage: SnapshotStorage::Both,
        })?;
        let safety_snapshot_id = match safety {
            CreateOutcome::Created(summary) => Some(summary.id),
            CreateOutcome::Unchanged { .. } | CreateOutcome::NothingToBackUp => None,
        };

        let files_restored = swap_in(&path, &manifest, id)?;
        tracing::info!(snapshot = %id, files_restored, ?safety_snapshot_id, "snapshot restored");
        Ok(RestoreReport { restored_snapshot_id: id.to_owned(), safety_snapshot_id, files_restored })
    }

    /// Deletes a snapshot's archive (only inside the snapshot folder) and its records.
    /// A Drive copy is kept: the snapshot stays listed, without its local archive.
    pub fn delete(&self, id: &str) -> CoreResult<()> {
        self.remove(id, false)
    }

    /// Deletes a snapshot from this PC and queues the deletion of its Drive copy. The Drive file
    /// leaves Drive on the next sync pass (immediately when connected, later otherwise).
    pub fn delete_everywhere(&self, id: &str) -> CoreResult<()> {
        self.remove(id, true)
    }

    fn remove(&self, id: &str, from_drive: bool) -> CoreResult<()> {
        let repository = SnapshotRepository::new(self.db);
        let snapshot = repository.get(id)?;
        if let Some(archive) = snapshot.archive_path.as_deref().map(PathBuf::from) {
            if !is_inside(&archive, &self.root) {
                return Err(CoreError::Validation(format!("{} is outside the snapshot folder", archive.display())));
            }
            match std::fs::remove_file(&archive) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        if from_drive {
            CloudRepository::new(self.db).schedule_remote_deletion(id)?;
        } else if repository.has_cloud_copy(id)? {
            // The cloud copy is kept: the snapshot stays listed, without its local archive.
            repository.detach_local(id)?;
            tracing::info!(snapshot = %id, "local snapshot archive deleted; the cloud copy remains");
            return Ok(());
        }
        repository.remove(id)?;
        tracing::info!(snapshot = %id, from_drive, "snapshot deleted");
        Ok(())
    }

    /// Where the archive of a snapshot lives (or will live once downloaded).
    pub fn archive_destination(&self, game_id: i64, snapshot_id: &str) -> CoreResult<PathBuf> {
        Ok(self.root.join(game_folder(self.db, game_id)?).join(format!("{snapshot_id}.zip")))
    }

    /// Prunes the automatic, unpinned snapshots of a game that fall outside the policy.
    ///
    /// - Beyond `keep_local`: the archive leaves this PC. A Drive copy that is still within the
    ///   Drive limit stays (the snapshot lives on as cloud-only).
    /// - Beyond the Drive limit: the Drive copy is handed to the deletion queue, and the snapshot is
    ///   forgotten once no archive is left anywhere.
    ///
    /// Manual, pinned, safety and conflict-copy snapshots, and snapshots in a sync conflict, are
    /// never touched.
    pub fn apply_retention(&self, game_id: i64, policy: RetentionPolicy) -> CoreResult<RetentionReport> {
        let repository = SnapshotRepository::new(self.db);
        let local_expired = repository.retention_candidates(game_id, policy.keep_local)?;
        let cloud_expired = repository.cloud_retention_candidates(game_id, policy.effective_keep_cloud())?;
        let mut report = RetentionReport::default();
        for id in &local_expired {
            if cloud_expired.contains(id) {
                self.remove(id, true)?;
                report.removed_cloud += 1;
            } else {
                self.remove(id, false)?;
            }
            report.removed_local += 1;
        }
        // Drive-only snapshots (no archive on this PC) that are past the Drive limit.
        for id in cloud_expired.iter().filter(|id| !local_expired.contains(id)) {
            self.remove(id, true)?;
            report.removed_cloud += 1;
        }
        Ok(report)
    }

    /// Size of the current save and whether it is identical to the latest snapshot.
    pub fn current_state(&self, game_id: i64) -> CoreResult<CurrentSaveState> {
        let files = collect_files(&SaveLocationRepository::new(self.db).list(game_id)?)?;
        let repository = SnapshotRepository::new(self.db);
        let latest = repository.latest(game_id)?;
        let matches_latest = match &latest {
            Some(snapshot) if !files.is_empty() => same_files(
                &repository.file_hashes(&snapshot.id)?,
                &files.iter().map(|file| file.absolute.clone()).collect::<Vec<_>>(),
            )?,
            _ => false,
        };
        Ok(CurrentSaveState {
            total_files: i64::try_from(files.len()).unwrap_or(i64::MAX),
            total_bytes: i64::try_from(files.iter().map(|file| file.size).sum::<u64>()).unwrap_or(i64::MAX),
            last_modified_at: files.iter().filter_map(|file| file.modified).max().map(crate::clock::to_rfc3339),
            latest_snapshot_id: latest.map(|snapshot| snapshot.id),
            matches_latest,
        })
    }

    fn archive_path(&self, snapshot: &SnapshotSummary) -> CoreResult<PathBuf> {
        if snapshot.status == SnapshotStatus::MissingLocal {
            return Err(CoreError::NotFound("the snapshot archive is missing from this PC".into()));
        }
        snapshot
            .archive_path
            .as_deref()
            .map(PathBuf::from)
            .ok_or_else(|| CoreError::NotFound("the snapshot has no local archive".into()))
    }
}

/// Archive checksum, then per-entry verification.
fn check_archive(path: &Path, expected_sha256: &str) -> CoreResult<ArchiveManifest> {
    if sha256_file(path)? != expected_sha256 {
        return Err(CoreError::Integrity("the archive changed since it was created".into()));
    }
    verify_archive(path)
}

/// True when the current files are exactly the snapshot's files (same set, same contents).
fn same_files(recorded: &[(String, String)], current: &[PathBuf]) -> CoreResult<bool> {
    if recorded.len() != current.len() {
        return Ok(false);
    }
    let recorded: BTreeSet<(String, String)> =
        recorded.iter().map(|(path, hash)| (normalize_path(Path::new(path)), hash.clone())).collect();
    let mut actual = BTreeSet::new();
    for path in current {
        actual.insert((normalize_path(path), sha256_file(path)?));
    }
    Ok(recorded == actual)
}

fn game_folder(db: &Database, game_id: i64) -> CoreResult<String> {
    Ok(db.connection()?.query_row("SELECT slug FROM games WHERE id = ?1", [game_id], |row| row.get(0))?)
}

fn is_inside(path: &Path, root: &Path) -> bool {
    let path = normalize_path(path);
    let root = normalize_path(root);
    path.starts_with(&format!("{root}\\"))
}

/// One location swapped in, kept for rollback.
struct Swap {
    target: PathBuf,
    previous: Option<PathBuf>,
}

/// Stages every location next to its target, swaps them in with renames, verifies, and rolls
/// back all locations if anything fails. Returns the number of files restored.
fn swap_in(archive_path: &Path, manifest: &ArchiveManifest, snapshot_id: &str) -> CoreResult<i64> {
    let tag: String = snapshot_id.chars().rev().take(8).collect();
    let mut archive = open_archive(archive_path)?;
    let mut applied: Vec<Swap> = Vec::new();
    let mut staged: Vec<PathBuf> = Vec::new();

    let result = (|| -> CoreResult<i64> {
        let mut restored = 0_i64;
        for location in &manifest.locations {
            let target = PathBuf::from(&location.path);
            let parent = target
                .parent()
                .ok_or_else(|| CoreError::Validation(format!("{} has no parent folder", target.display())))?;
            std::fs::create_dir_all(parent)?;
            let staging = parent.join(format!(".l8-{tag}-{}-restore", location.index));
            let previous = parent.join(format!(".l8-{tag}-{}-previous", location.index));
            remove_owned(&staging)?;
            remove_owned(&previous)?;
            let files: Vec<_> = manifest.files.iter().filter(|file| file.location == location.index).collect();

            let has_new_content = match location.kind {
                LocationKind::Directory => {
                    std::fs::create_dir_all(&staging)?;
                    staged.push(staging.clone());
                    for file in &files {
                        let relative = safe_relative(&file.relative_path)
                            .ok_or_else(|| CoreError::Integrity(format!("unsafe path {}", file.relative_path)))?;
                        extract_file(&mut archive, file, &staging.join(relative))?;
                    }
                    true
                }
                LocationKind::File => match files.first() {
                    Some(file) => {
                        staged.push(staging.clone());
                        extract_file(&mut archive, file, &staging)?;
                        true
                    }
                    None => false,
                },
            };

            let had_previous = target.exists();
            if had_previous {
                std::fs::rename(&target, &previous)?;
            }
            let swap = Swap { target: target.clone(), previous: had_previous.then_some(previous.clone()) };
            if has_new_content && let Err(error) = std::fs::rename(&staging, &target) {
                if had_previous {
                    let _ = std::fs::rename(&previous, &target);
                }
                return Err(error.into());
            }
            applied.push(swap);
            restored += i64::try_from(files.len()).unwrap_or(0);
        }

        // Post-check: what is on disk now is exactly the snapshot.
        for file in &manifest.files {
            let location =
                manifest.locations.get(file.location).ok_or_else(|| CoreError::Integrity("unknown location".into()))?;
            let on_disk = match location.kind {
                LocationKind::File => PathBuf::from(&location.path),
                LocationKind::Directory => PathBuf::from(&location.path).join(
                    safe_relative(&file.relative_path).ok_or_else(|| CoreError::Integrity("unsafe path".into()))?,
                ),
            };
            if sha256_file(&on_disk)? != file.sha256 {
                return Err(CoreError::Integrity(format!("{} did not restore correctly", on_disk.display())));
            }
        }
        Ok(restored)
    })();

    match result {
        Ok(restored) => {
            for swap in &applied {
                if let Some(previous) = &swap.previous
                    && let Err(error) = remove_owned(previous)
                {
                    tracing::warn!(%error, path = %previous.display(), "could not remove the replaced save; it is also in the safety snapshot");
                }
            }
            Ok(restored)
        }
        Err(error) => {
            for swap in applied.iter().rev() {
                if let Err(rollback_error) = rollback(swap) {
                    tracing::error!(%rollback_error, target = %swap.target.display(), "rollback failed; the safety snapshot holds the previous save");
                }
            }
            for path in &staged {
                let _ = remove_owned(path);
            }
            Err(error)
        }
    }
}

fn rollback(swap: &Swap) -> CoreResult<()> {
    if swap.target.exists() {
        remove_path(&swap.target)?;
    }
    if let Some(previous) = &swap.previous {
        std::fs::rename(previous, &swap.target)?;
    }
    Ok(())
}

/// Removes a path this engine created (`.l8-…` staging or previous copies). Refuses anything else.
fn remove_owned(path: &Path) -> CoreResult<()> {
    let owned = path.file_name().is_some_and(|name| name.to_string_lossy().starts_with(".l8-"));
    if !owned {
        return Err(CoreError::Validation(format!("refusing to delete {}", path.display())));
    }
    if path.exists() { remove_path(path) } else { Ok(()) }
}

fn remove_path(path: &Path) -> CoreResult<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)?
    } else {
        std::fs::remove_file(path)?
    }
    Ok(())
}

/// Deletes a temporary file unless disarmed (used for partially written archives).
struct RemoveOnDrop(Option<PathBuf>);

impl RemoveOnDrop {
    fn disarm(mut self) {
        self.0 = None;
    }
}

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}
