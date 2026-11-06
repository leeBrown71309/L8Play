use std::path::PathBuf;

use l8_core::library::LibraryRepository;
use l8_core::saves::pcgamingwiki::PathEnvironment;
use l8_core::saves::{
    DetectionOutcome, FindSavesOutcome, SaveLocation, SaveLocationRepository, broad_roots_from_environment,
    detect_save_locations, find_save_candidates, validate_user_folder,
};
use l8_core::snapshots::{
    CreateOutcome, CreateRequest, CurrentSaveState, GameSaveStatus, RestoreReport, RestoreTarget, SnapshotEngine,
    SnapshotKind, SnapshotRepository, SnapshotStorage, SnapshotSummary,
};
use l8_core::{CoreError, CoreResult};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::dto::CreateSnapshotResult;
use crate::error::{CommandError, CommandErrorCode, CommandResult};
use crate::events;
use crate::state::AppState;

/// Runs blocking save work off the async runtime.
async fn blocking<T, F>(app: &AppHandle, work: F) -> CommandResult<T>
where
    T: Send + 'static,
    F: FnOnce(&AppState) -> CoreResult<T> + Send + 'static,
{
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || work(&app.state::<AppState>()))
        .await
        .map_err(|error| CommandError::new(CommandErrorCode::Internal, error.to_string()))?
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_save_locations(state: State<'_, AppState>, game_id: i64) -> CommandResult<Vec<SaveLocation>> {
    Ok(SaveLocationRepository::new(&state.db).list(game_id)?)
}

#[tauri::command]
pub async fn detect_saves(app: AppHandle, game_id: i64) -> CommandResult<DetectionOutcome> {
    let outcome = blocking(&app, move |state| detect_save_locations(&state.db, &state.ludusavi, game_id)).await?;
    events::saves_changed(&app, game_id);
    Ok(outcome)
}

#[tauri::command]
pub async fn add_save_folder(app: AppHandle, game_id: i64, folder: String) -> CommandResult<Vec<SaveLocation>> {
    let locations = blocking(&app, move |state| {
        let folder = PathBuf::from(folder);
        let install = LibraryRepository::new(&state.db).launch_target(game_id)?.install_path;
        validate_user_folder(&folder, install.as_deref(), &broad_roots_from_environment())
            .map_err(CoreError::Validation)?;
        let repository = SaveLocationRepository::new(&state.db);
        repository.add_user_folder(game_id, &folder)?;
        repository.list(game_id)
    })
    .await?;
    events::saves_changed(&app, game_id);
    Ok(locations)
}

#[tauri::command]
pub async fn remove_save_location(
    app: AppHandle,
    state: State<'_, AppState>,
    game_id: i64,
    location_id: i64,
) -> CommandResult<Vec<SaveLocation>> {
    let repository = SaveLocationRepository::new(&state.db);
    repository.reject(game_id, location_id)?;
    events::saves_changed(&app, game_id);
    Ok(repository.list(game_id)?)
}

#[tauri::command]
pub async fn list_snapshots(state: State<'_, AppState>, game_id: Option<i64>) -> CommandResult<Vec<SnapshotSummary>> {
    Ok(SnapshotRepository::new(&state.db).list(game_id)?)
}

#[tauri::command]
pub async fn get_save_statuses(state: State<'_, AppState>) -> CommandResult<Vec<GameSaveStatus>> {
    Ok(SnapshotRepository::new(&state.db).save_statuses()?)
}

#[tauri::command]
pub async fn get_current_save(app: AppHandle, game_id: i64) -> CommandResult<CurrentSaveState> {
    blocking(&app, move |state| SnapshotEngine::new(&state.db, state.snapshot_root()?).current_state(game_id)).await
}

#[tauri::command]
pub async fn create_snapshot(
    app: AppHandle,
    game_id: i64,
    name: String,
    description: Option<String>,
    pinned: bool,
    storage: Option<SnapshotStorage>,
) -> CommandResult<CreateSnapshotResult> {
    let outcome = blocking(&app, move |state| {
        state.ensure_not_running(game_id)?;
        let _guard = state.snapshot_lock()?;
        SnapshotEngine::new(&state.db, state.snapshot_root()?).create(&CreateRequest {
            game_id,
            name,
            description,
            kind: SnapshotKind::Manual,
            pinned,
            skip_if_unchanged: false,
            storage: storage.unwrap_or_default(),
        })
    })
    .await?;
    if let CreateOutcome::Created(snapshot) = &outcome {
        crate::cloud_service::enqueue_upload(&app, &snapshot.id);
    }
    events::saves_changed(&app, game_id);
    Ok(outcome.into())
}

#[tauri::command]
pub async fn restore_snapshot(app: AppHandle, snapshot_id: String) -> CommandResult<RestoreReport> {
    let (game_id, report) = blocking(&app, move |state| {
        let snapshot = SnapshotRepository::new(&state.db).get(&snapshot_id)?;
        state.ensure_not_running(snapshot.game_id)?;
        let _guard = state.snapshot_lock()?;
        if snapshot.status == l8_core::snapshots::SnapshotStatus::MissingLocal {
            // A snapshot that lives only in Drive is downloaded and checksum-verified first.
            state.cloud(|sync| sync.download_now(&snapshot_id))?;
        }
        let report = SnapshotEngine::new(&state.db, state.snapshot_root()?).restore(&snapshot_id)?;
        Ok((snapshot.game_id, report))
    })
    .await?;
    events::saves_changed(&app, game_id);
    Ok(report)
}

/// Folders an imported snapshot (one that does not know its own save folder) can be restored into.
#[tauri::command]
pub async fn get_restore_targets(app: AppHandle, snapshot_id: String) -> CommandResult<Vec<RestoreTarget>> {
    blocking(&app, move |state| {
        let snapshot = SnapshotRepository::new(&state.db).get(&snapshot_id)?;
        if snapshot.status == l8_core::snapshots::SnapshotStatus::MissingLocal {
            state.cloud(|sync| sync.download_now(&snapshot_id))?;
        }
        SnapshotEngine::new(&state.db, state.snapshot_root()?).restore_targets(&snapshot_id)
    })
    .await
}

/// Restores an imported snapshot over the folder the user chose. Files are laid over it, never removed.
#[tauri::command]
pub async fn restore_snapshot_to(app: AppHandle, snapshot_id: String, target: String) -> CommandResult<RestoreReport> {
    let (game_id, report) = blocking(&app, move |state| {
        let snapshot = SnapshotRepository::new(&state.db).get(&snapshot_id)?;
        state.ensure_not_running(snapshot.game_id)?;
        let _guard = state.snapshot_lock()?;
        if snapshot.status == l8_core::snapshots::SnapshotStatus::MissingLocal {
            state.cloud(|sync| sync.download_now(&snapshot_id))?;
        }
        let report = SnapshotEngine::new(&state.db, state.snapshot_root()?)
            .restore_imported(&snapshot_id, &PathBuf::from(target))?;
        Ok((snapshot.game_id, report))
    })
    .await?;
    events::saves_changed(&app, game_id);
    Ok(report)
}

/// Deletes a snapshot from this PC. With `also_from_drive` its Google Drive copy is deleted too
/// (queued, so it also works while offline); without it the Drive copy is kept.
#[tauri::command]
pub async fn delete_snapshot(app: AppHandle, snapshot_id: String, also_from_drive: Option<bool>) -> CommandResult<()> {
    let from_drive = also_from_drive.unwrap_or(false);
    let game_id = blocking(&app, move |state| {
        let snapshot = SnapshotRepository::new(&state.db).get(&snapshot_id)?;
        let _guard = state.snapshot_lock()?;
        let engine = SnapshotEngine::new(&state.db, state.snapshot_root()?);
        if from_drive {
            engine.delete_everywhere(&snapshot_id)?;
        } else {
            engine.delete(&snapshot_id)?;
        }
        Ok(snapshot.game_id)
    })
    .await?;
    events::saves_changed(&app, game_id);
    if from_drive {
        crate::cloud_service::kick_queue(&app);
    }
    Ok(())
}

#[tauri::command]
pub async fn verify_snapshot(app: AppHandle, snapshot_id: String) -> CommandResult<SnapshotSummary> {
    let summary =
        blocking(&app, move |state| SnapshotEngine::new(&state.db, state.snapshot_root()?).verify(&snapshot_id))
            .await?;
    events::saves_changed(&app, summary.game_id);
    Ok(summary)
}

#[tauri::command]
pub async fn set_snapshot_pinned(
    app: AppHandle,
    state: State<'_, AppState>,
    snapshot_id: String,
    pinned: bool,
) -> CommandResult<SnapshotSummary> {
    let repository = SnapshotRepository::new(&state.db);
    repository.set_pinned(&snapshot_id, pinned)?;
    let summary = repository.get(&snapshot_id)?;
    events::saves_changed(&app, summary.game_id);
    Ok(summary)
}

/// Names a snapshot (an empty name clears it). Imported snapshots take the name of their source folder.
#[tauri::command]
pub async fn rename_snapshot(
    app: AppHandle,
    state: State<'_, AppState>,
    snapshot_id: String,
    label: Option<String>,
) -> CommandResult<SnapshotSummary> {
    let repository = SnapshotRepository::new(&state.db);
    repository.set_label(&snapshot_id, label.as_deref())?;
    let summary = repository.get(&snapshot_id)?;
    events::saves_changed(&app, summary.game_id);
    Ok(summary)
}

/// Shows a snapshot's archive in Explorer, or the snapshot folder when no id is given.
#[tauri::command]
pub async fn reveal_snapshot(
    app: AppHandle,
    state: State<'_, AppState>,
    snapshot_id: Option<String>,
) -> CommandResult<()> {
    let opener = app.opener();
    let result = match snapshot_id {
        Some(id) => {
            let archive = SnapshotRepository::new(&state.db)
                .get(&id)?
                .archive_path
                .ok_or_else(|| CoreError::NotFound("archive".into()))?;
            opener.reveal_item_in_dir(archive)
        }
        None => {
            let root = state.snapshot_root()?;
            std::fs::create_dir_all(&root).map_err(CoreError::from)?;
            opener.open_path(root.to_string_lossy(), None::<&str>)
        }
    };
    result.map_err(|error| CommandError::new(CommandErrorCode::Io, error.to_string()))
}

#[tauri::command]
pub async fn get_snapshot_folder(state: State<'_, AppState>) -> CommandResult<String> {
    Ok(state.snapshot_root()?.to_string_lossy().into_owned())
}

/// "Find saves": PCGamingWiki and a bounded local search. Only proposes folders that exist; the
/// user confirms one through `add_save_folder`, which validates it again.
#[tauri::command]
pub async fn find_saves(app: AppHandle, game_id: i64) -> CommandResult<FindSavesOutcome> {
    blocking(&app, move |state| {
        let install = LibraryRepository::new(&state.db).launch_target(game_id)?.install_path;
        let environment = PathEnvironment::from_process(install);
        find_save_candidates(&state.db, state.http(), game_id, state.documents_dir(), &environment)
    })
    .await
}

/// Opens a PCGamingWiki page in the browser. Only that site's wiki pages are accepted.
#[tauri::command]
pub async fn open_wiki_page(app: AppHandle, url: String) -> CommandResult<()> {
    if !url.starts_with("https://www.pcgamingwiki.com/wiki/") {
        return Err(CoreError::Validation("only PCGamingWiki pages can be opened".into()).into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|error| CommandError::new(CommandErrorCode::Io, error.to_string()))
}
