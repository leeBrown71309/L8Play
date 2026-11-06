//! Imports the games confirmed in the review step.

use std::path::{Path, PathBuf};

use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::library::LibraryRepository;
use crate::saves::importer::model::{ImportGameRequest, ImportProgress, ImportReport, ImportedGame};
use crate::saves::importer::noise::is_noise;
use crate::snapshots::{ImportOutcome, ImportRequest, SnapshotEngine, SnapshotStorage};

/// Imports every game of `games`, one snapshot per planned folder, into the snapshot folder `root`.
///
/// A failure on one game (or one snapshot) is recorded in the report and never stops the others.
/// Folders that were already imported are recognised by their content and skipped.
pub fn import_games(
    db: &Database,
    root: PathBuf,
    games: &[ImportGameRequest],
    storage: SnapshotStorage,
    on_progress: &mut dyn FnMut(ImportProgress),
) -> ImportReport {
    let engine = SnapshotEngine::new(db, root);
    let total = u32::try_from(games.len()).unwrap_or(u32::MAX);
    let mut report = ImportReport::default();
    for (index, request) in games.iter().enumerate() {
        on_progress(ImportProgress {
            done: u32::try_from(index).unwrap_or(u32::MAX),
            total,
            current_title: request.title.clone(),
        });
        report.games.push(import_one(db, &engine, request, storage));
    }
    on_progress(ImportProgress { done: total, total, current_title: String::new() });
    report
}

fn import_one(
    db: &Database,
    engine: &SnapshotEngine<'_>,
    request: &ImportGameRequest,
    storage: SnapshotStorage,
) -> ImportedGame {
    let mut result = ImportedGame {
        game_id: None,
        title: request.title.trim().to_owned(),
        created: false,
        imported: 0,
        snapshot_ids: Vec::new(),
        duplicates: 0,
        errors: Vec::new(),
    };
    let added = match LibraryRepository::new(db).find_or_create_archived(&request.title) {
        Ok(added) => added,
        Err(error) => {
            result.errors.push(error.to_string());
            return result;
        }
    };
    result.game_id = Some(added.game.id);
    result.created = added.created;
    for snapshot in &request.snapshots {
        match import_snapshot(engine, added.game.id, snapshot, storage) {
            Ok(ImportOutcome::Imported(created)) => {
                result.imported += 1;
                result.snapshot_ids.push(created.id);
            }
            Ok(ImportOutcome::Duplicate { .. }) => result.duplicates += 1,
            Ok(ImportOutcome::Empty) => {}
            Err(error) => {
                tracing::warn!(%error, root = %snapshot.root, "snapshot import failed");
                result.errors.push(format!("{}: {error}", snapshot.label.as_deref().unwrap_or(&snapshot.root)));
            }
        }
    }
    result
}

fn import_snapshot(
    engine: &SnapshotEngine<'_>,
    game_id: i64,
    snapshot: &crate::saves::importer::model::PlannedSnapshot,
    storage: SnapshotStorage,
) -> CoreResult<ImportOutcome> {
    let source = Path::new(&snapshot.root);
    if !source.is_dir() {
        return Err(CoreError::NotFound(format!("{} no longer exists", source.display())));
    }
    let hint_file = snapshot.hint_file.clone();
    let skip = move |relative: &Path| is_noise(relative) || hint_file.as_deref().is_some_and(|name| relative == Path::new(name));
    engine.import_folder(&ImportRequest {
        game_id,
        source,
        label: snapshot.label.clone(),
        hint: snapshot.hint.clone(),
        skip: &skip,
        storage,
    })
}
