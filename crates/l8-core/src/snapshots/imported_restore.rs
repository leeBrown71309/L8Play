//! Restoring a snapshot imported from an old backup folder.
//!
//! Such a snapshot does not know where its files belong on this PC, so the destination is chosen when
//! it is restored. Unlike a regular restore (which swaps a whole save folder), files are *laid over*
//! the chosen folder: only files the snapshot contains are written, nothing else is removed, so a folder
//! picked a little too wide can never lose data. Files about to be overwritten are first kept in a
//! safety snapshot, and any failure puts every file back.

use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

use crate::clock::now_rfc3339;
use crate::error::{CoreError, CoreResult};
use crate::saves::{
    LocationKind, SaveLocationRepository, broad_roots_from_environment, normalize_path, validate_user_folder,
};
use crate::snapshots::archive::{ArchiveManifest, ArchivedLocation, extract_file, open_archive, verify_archive, write_archive};
use crate::snapshots::engine::{RemoveOnDrop, SnapshotEngine, check_archive, game_folder, remove_owned};
use crate::snapshots::files::{SourceFile, safe_relative, sha256_file};
use crate::snapshots::model::{RestoreReport, SnapshotKind, SnapshotStorage};
use crate::snapshots::repository::{NewSnapshot, SnapshotRepository};

/// Why a folder is proposed as the destination of an imported snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TargetSource {
    /// Written in a note that came with the backup.
    Hint,
    /// A save folder found for this game on this PC.
    Detected,
}

/// A folder an imported snapshot can be restored into.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RestoreTarget {
    pub path: String,
    pub source: TargetSource,
    /// The folder exists on this PC right now.
    pub exists: bool,
}

/// `%LOCALAPPDATA%\Game\Saved` → the same path with the variable replaced by its value.
pub fn expand_environment(path: &str) -> Option<PathBuf> {
    let mut out = String::new();
    let mut rest = path;
    while let Some(start) = rest.find('%') {
        out.push_str(rest.get(..start)?);
        let after = rest.get(start + 1..)?;
        let end = after.find('%')?;
        let value = std::env::var(after.get(..end)?).ok()?;
        out.push_str(&value);
        rest = after.get(end + 1..)?;
    }
    out.push_str(rest);
    Some(PathBuf::from(out))
}

/// First folder name of every file in the snapshot (`SaveGames` for `SaveGames/76561/slot.sav`).
fn top_level_names(manifest: &ArchiveManifest) -> Vec<String> {
    let mut names: Vec<String> = manifest
        .files
        .iter()
        .filter_map(|file| {
            let mut parts = file.relative_path.split('/');
            let first = parts.next()?;
            parts.next().map(|_| first.to_lowercase())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

impl SnapshotEngine<'_> {
    /// Folders an imported snapshot could go to: the one named by its hint file, then this game's
    /// known save folders. When the backup's own top folder carries the name of a save folder
    /// (`SaveGames/…` for `…\Saved\SaveGames`) the files belong in that folder's parent.
    pub fn restore_targets(&self, id: &str) -> CoreResult<Vec<RestoreTarget>> {
        let repository = SnapshotRepository::new(self.database());
        let snapshot = repository.get(id)?;
        let path = self.archive_file(&snapshot)?;
        let manifest = verify_archive(&path)?;
        let tops = top_level_names(&manifest);

        let mut targets: Vec<RestoreTarget> = Vec::new();
        let mut add = |candidate: PathBuf, source: TargetSource| {
            let key = normalize_path(&candidate);
            if targets.iter().all(|known| normalize_path(Path::new(&known.path)) != key) {
                targets.push(RestoreTarget {
                    exists: candidate.is_dir(),
                    path: candidate.to_string_lossy().into_owned(),
                    source,
                });
            }
        };
        for location in &manifest.locations {
            if let Some(expanded) = location.hint.as_deref().and_then(expand_environment) {
                add(expanded, TargetSource::Hint);
            }
        }
        for location in SaveLocationRepository::new(self.database()).list(snapshot.game_id)? {
            let folder = PathBuf::from(&location.path);
            let named_like_top = folder
                .file_name()
                .is_some_and(|name| tops.contains(&name.to_string_lossy().to_lowercase()));
            match (named_like_top, folder.parent()) {
                (true, Some(parent)) => add(parent.to_path_buf(), TargetSource::Detected),
                _ => add(folder, TargetSource::Detected),
            }
        }
        Ok(targets)
    }

    /// Restores an imported snapshot over `target`. The folder becomes a save location of the game, so
    /// later snapshots and restores work like for any installed game.
    pub fn restore_imported(&self, id: &str, target: &Path) -> CoreResult<RestoreReport> {
        let repository = SnapshotRepository::new(self.database());
        let snapshot = repository.get(id)?;
        let path = self.archive_file(&snapshot)?;
        let manifest = match check_archive(&path, &snapshot.archive_sha256) {
            Ok(manifest) => manifest,
            Err(error @ CoreError::Integrity(_)) => {
                repository.set_status(id, "corrupt")?;
                return Err(error);
            }
            Err(error) => return Err(error),
        };
        if !manifest.locations.iter().all(|location| location.unresolved) {
            return Err(CoreError::Validation("this snapshot has its own save folder: use a regular restore".into()));
        }
        if !target.is_absolute() {
            return Err(CoreError::Validation("choose a folder on this PC".into()));
        }
        if target.exists() {
            validate_user_folder(target, None, &broad_roots_from_environment()).map_err(CoreError::Validation)?;
        }

        let safety_snapshot_id = self.protect_files_to_overwrite(snapshot.game_id, target, &manifest)?;
        SaveLocationRepository::new(self.database()).add_user_folder(snapshot.game_id, target)?;
        let files_restored = overlay_in(&path, &manifest, target, id)?;
        tracing::info!(snapshot = %id, files_restored, ?safety_snapshot_id, "imported snapshot restored");
        Ok(RestoreReport { restored_snapshot_id: id.to_owned(), safety_snapshot_id, files_restored })
    }

    /// Keeps the files that the restore would overwrite in a safety snapshot (none: nothing to protect).
    fn protect_files_to_overwrite(
        &self,
        game_id: i64,
        target: &Path,
        manifest: &ArchiveManifest,
    ) -> CoreResult<Option<String>> {
        let existing: Vec<SourceFile> = manifest
            .files
            .iter()
            .filter_map(|file| {
                let relative = safe_relative(&file.relative_path)?;
                let absolute = target.join(&relative);
                let metadata = std::fs::symlink_metadata(&absolute).ok().filter(|meta| meta.is_file())?;
                Some(SourceFile {
                    location_index: 0,
                    relative,
                    absolute,
                    size: metadata.len(),
                    modified: metadata.modified().ok(),
                })
            })
            .collect();
        if existing.is_empty() {
            return Ok(None);
        }

        let repository = SnapshotRepository::new(self.database());
        let title: String = self
            .database()
            .connection()?
            .query_row("SELECT title FROM games WHERE id = ?1", [game_id], |row| row.get(0))?;
        let id = uuid::Uuid::now_v7().to_string();
        let folder = self.root_folder().join(game_folder(self.database(), game_id)?);
        std::fs::create_dir_all(&folder)?;
        let partial = folder.join(format!(".{id}.zip.partial"));
        let final_path = folder.join(format!("{id}.zip"));
        let guard = RemoveOnDrop::new(partial.clone());

        let location = ArchivedLocation {
            index: 0,
            path: target.to_string_lossy().into_owned(),
            kind: LocationKind::Directory,
            unresolved: false,
            hint: None,
        };
        let created_at = now_rfc3339();
        let safety_manifest = write_archive(&partial, &id, &title, &created_at, &[location], &existing)?;
        verify_archive(&partial)?;
        let archive_sha256 = sha256_file(&partial)?;
        let archive_size = i64::try_from(std::fs::metadata(&partial)?.len()).unwrap_or(i64::MAX);
        std::fs::rename(&partial, &final_path)?;
        guard.disarm();
        repository.insert(&NewSnapshot {
            id: &id,
            game_id,
            name: "",
            description: None,
            kind: SnapshotKind::Safety,
            pinned: false,
            storage: SnapshotStorage::Both,
            archive_path: &final_path,
            archive_size,
            archive_sha256: &archive_sha256,
            manifest: &safety_manifest,
            label: None,
            imported: false,
        })?;
        Ok(Some(id))
    }
}

/// Lays every file of the snapshot over `target`: staged and checksum-verified next to it first, then
/// moved into place one by one, the replaced files kept aside until everything succeeded.
fn overlay_in(archive_path: &Path, manifest: &ArchiveManifest, target: &Path, snapshot_id: &str) -> CoreResult<i64> {
    let tag: String = snapshot_id.chars().rev().take(8).collect();
    let parent = target
        .parent()
        .ok_or_else(|| CoreError::Validation(format!("{} has no parent folder", target.display())))?;
    std::fs::create_dir_all(parent)?;
    let staging = parent.join(format!(".l8-{tag}-overlay-staging"));
    let previous = parent.join(format!(".l8-{tag}-overlay-previous"));
    remove_owned(&staging)?;
    remove_owned(&previous)?;
    std::fs::create_dir_all(&staging)?;
    std::fs::create_dir_all(&previous)?;

    let mut archive = open_archive(archive_path)?;
    // (destination, the file it replaced) for rollback, and the folders this restore created.
    let mut placed: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    let mut created_dirs: Vec<PathBuf> = Vec::new();

    let result = (|| -> CoreResult<i64> {
        let mut relatives = Vec::with_capacity(manifest.files.len());
        for file in &manifest.files {
            let relative = safe_relative(&file.relative_path)
                .ok_or_else(|| CoreError::Integrity(format!("unsafe path {}", file.relative_path)))?;
            extract_file(&mut archive, file, &staging.join(&relative))?;
            relatives.push(relative);
        }
        for relative in &relatives {
            let destination = target.join(relative);
            if let Some(folder) = destination.parent() {
                create_missing_dirs(folder, &mut created_dirs)?;
            }
            let backup = if destination.exists() {
                let backup = previous.join(relative);
                if let Some(folder) = backup.parent() {
                    std::fs::create_dir_all(folder)?;
                }
                std::fs::rename(&destination, &backup)?;
                Some(backup)
            } else {
                None
            };
            placed.push((destination.clone(), backup));
            std::fs::rename(staging.join(relative), &destination)?;
        }
        for (file, relative) in manifest.files.iter().zip(&relatives) {
            if sha256_file(&target.join(relative))? != file.sha256 {
                return Err(CoreError::Integrity(format!("{} did not restore correctly", relative.display())));
            }
        }
        Ok(i64::try_from(relatives.len()).unwrap_or(i64::MAX))
    })();

    if result.is_err() {
        for (destination, backup) in placed.iter().rev() {
            if destination.is_file() {
                let _ = std::fs::remove_file(destination);
            }
            if let Some(backup) = backup
                && let Err(error) = std::fs::rename(backup, destination)
            {
                tracing::error!(%error, path = %destination.display(), "could not put a replaced file back");
            }
        }
        for folder in created_dirs.iter().rev() {
            let _ = std::fs::remove_dir(folder);
        }
    }
    for leftover in [&staging, &previous] {
        if let Err(error) = remove_owned(leftover) {
            tracing::warn!(%error, path = %leftover.display(), "could not remove a temporary restore folder");
        }
    }
    result
}

/// Creates `folder` and its missing parents, remembering each folder that did not exist.
fn create_missing_dirs(folder: &Path, created: &mut Vec<PathBuf>) -> CoreResult<()> {
    let mut missing: Vec<PathBuf> = folder.ancestors().take_while(|path| !path.exists()).map(Path::to_path_buf).collect();
    missing.reverse();
    for path in missing {
        std::fs::create_dir(&path)?;
        created.push(path);
    }
    Ok(())
}
