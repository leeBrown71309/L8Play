//! Import of saves from older backup folders (games that are not installed).

use std::path::{Path, PathBuf};

use l8_core::CoreError;
use l8_core::saves::importer::{
    ImportGameRequest, ImportPlan, ImportReport, PlannedGame, PlannedLayout, ScanMode, import_games, plan_game,
    scan_folder, scan_pc_for_saves, suggest_titles,
};
use l8_core::snapshots::SnapshotStorage;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::artwork::fetch_for_new_game;
use crate::error::{CommandError, CommandErrorCode, CommandResult};
use crate::events;
use crate::state::AppState;

/// Runs blocking import work off the async runtime.
async fn blocking<T, F>(app: &AppHandle, work: F) -> CommandResult<T>
where
    T: Send + 'static,
    F: FnOnce(&AppState) -> l8_core::CoreResult<T> + Send + 'static,
{
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || work(&app.state::<AppState>()))
        .await
        .map_err(|error| CommandError::new(CommandErrorCode::Internal, error.to_string()))?
        .map_err(CommandError::from)
}

/// Reads a backup folder and proposes the games and snapshots it would produce. Writes nothing.
#[tauri::command]
pub async fn scan_backup_folder(app: AppHandle, folder: String, mode: Option<ScanMode>) -> CommandResult<ImportPlan> {
    blocking(&app, move |state| {
        let mut plan = scan_folder(&PathBuf::from(folder), mode.unwrap_or_default())?;
        suggest_titles(&state.ludusavi, &mut plan);
        Ok(plan)
    })
    .await
}

/// Looks on this PC for the saves of games that are not installed (a long scan of Ludusavi's whole
/// database) and proposes them like a backup folder. Writes nothing.
#[tauri::command]
pub async fn scan_pc_for_leftover_saves(app: AppHandle) -> CommandResult<ImportPlan> {
    blocking(&app, move |state| scan_pc_for_saves(&state.db, &state.ludusavi)).await
}

/// Reads one game folder again with a forced layout (one snapshot, or one per sub-folder), when the
/// user disagrees with the first reading.
#[tauri::command]
pub async fn replan_backup_game(
    app: AppHandle,
    folder: String,
    key: String,
    from_collection: Option<String>,
    layout: PlannedLayout,
) -> CommandResult<Option<PlannedGame>> {
    blocking(&app, move |state| {
        let mut game = plan_game(Path::new(&folder), key, from_collection, Some(layout));
        if let Some(game) = &mut game {
            let mut plan =
                ImportPlan { source: folder, mode: ScanMode::OneGame, games: vec![game.clone()], warnings: Vec::new() };
            suggest_titles(&state.ludusavi, &mut plan);
            if let Some(updated) = plan.games.pop() {
                *game = updated;
            }
        }
        Ok(game)
    })
    .await
}

/// Imports the games confirmed in the review step as saves-only games, then queues the new
/// snapshots for Google Drive (when automatic sync is on) and looks up their artwork.
#[tauri::command]
pub async fn import_backup_games(
    app: AppHandle,
    games: Vec<ImportGameRequest>,
    storage: Option<SnapshotStorage>,
) -> CommandResult<ImportReport> {
    if games.is_empty() {
        return Err(CoreError::Validation("choose at least one game to import".into()).into());
    }
    let progress_app = app.clone();
    let report = blocking(&app, move |state| {
        let _guard = state.snapshot_lock()?;
        let mut on_progress = |progress| {
            if let Err(error) = progress_app.emit(events::IMPORT_PROGRESS, progress) {
                tracing::warn!(%error, "could not emit import progress");
            }
        };
        Ok(import_games(&state.db, state.snapshot_root()?, &games, storage.unwrap_or_default(), &mut on_progress))
    })
    .await?;

    events::library_changed(&app);
    for game in &report.games {
        if let Some(game_id) = game.game_id {
            events::saves_changed(&app, game_id);
            if game.created {
                fetch_for_new_game(&app, game_id);
            }
        }
        for snapshot_id in &game.snapshot_ids {
            crate::cloud_service::enqueue_upload(&app, snapshot_id);
        }
    }
    crate::cloud_service::kick_queue(&app);
    Ok(report)
}
