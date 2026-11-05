use std::path::PathBuf;

use l8_core::launcher::{LaunchPlan, plan_launch};
use l8_core::library::{
    FolderScanReport, GameDetail, GameSummary, LibraryRepository, ScanReport, scan_libraries, sync_owned_steam_library,
};
use l8_core::platforms::epic::EpicProvider;
use l8_core::platforms::gog::GogProvider;
use l8_core::platforms::steam::SteamProvider;
use l8_core::platforms::steam_owned::{
    OwnedLibraryStatus, clear_steam_api_key, has_steam_api_key, locate_steam_id, save_steam_api_key, steam_api_key,
};
use l8_core::platforms::xbox::XboxProvider;
use l8_core::settings::SettingsRepository;
use l8_core::snapshots::SnapshotKind;
use l8_core::{CoreError, CoreResult};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::dto::{AddGamesReport, NewGameInput};
use crate::error::{CommandError, CommandErrorCode, CommandResult};
use crate::events;
use crate::saves_service;
use crate::state::AppState;

/// Reads the launchers' installed games (Steam, Epic, the Xbox app and GOG). A game the user already
/// added by hand is moved to its launcher's platform. Purely local: no network.
fn run_local_scan(state: &AppState) -> CoreResult<ScanReport> {
    let _guard = state.scan_lock()?;
    let steam = SteamProvider;
    let epic = EpicProvider::from_environment();
    let xbox = XboxProvider::from_environment();
    let gog = GogProvider::from_environment();
    scan_libraries(&state.db, &[&steam, &epic, &xbox, &gog])
}

/// The full scan (startup and the "Scan libraries" button): the local scan, then, when the setting is
/// on, the Steam games the account owns but has not installed. That second step reads the public
/// Steam profile, never fails the scan and never removes anything.
pub fn run_scan(state: &AppState) -> CoreResult<ScanReport> {
    let mut report = run_local_scan(state)?;
    let enabled = SettingsRepository::new(&state.db).load().map(|settings| settings.show_owned_not_installed)?;
    let steam_id = if enabled { locate_steam_id() } else { None };
    let api_key = if enabled {
        steam_api_key(state.secrets()).unwrap_or_else(|error| {
            tracing::warn!(%error, "the saved Steam Web API key could not be read");
            None
        })
    } else {
        None
    };
    // No scan lock here: the request can take a while and the database write is one transaction.
    let owned = sync_owned_steam_library(&state.db, state.http(), steam_id.as_deref(), api_key.as_deref(), enabled);
    state.set_owned_library_status(owned.status);
    report.owned_library = owned.status;
    report.owned_added = owned.added;
    report.added += owned.added;
    report.total += owned.added;
    Ok(report)
}

/// Re-reads the launchers after games were added by hand, so one that belongs to the Xbox app or
/// GOG gets its platform at once. Never fails the add: the next scan would do the same.
fn reclassify_new_games(state: &AppState, games: &mut [GameSummary]) {
    if games.is_empty() {
        return;
    }
    if let Err(error) = run_local_scan(state) {
        tracing::warn!(%error, "the platform check after adding games failed");
        return;
    }
    let repository = LibraryRepository::new(&state.db);
    for game in games {
        if let Ok(Some(fresh)) = repository.summary(game.id) {
            *game = fresh;
        }
    }
}

/// Runs blocking library work off the async runtime.
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
pub async fn scan_libraries_now(app: AppHandle) -> CommandResult<ScanReport> {
    let report = blocking(&app, run_scan).await?;
    events::library_changed(&app);
    Ok(report)
}

/// Whether a Steam Web API key is saved. The key itself never goes back to the WebView.
#[tauri::command]
pub async fn get_steam_key_status(app: AppHandle) -> CommandResult<bool> {
    blocking(&app, |state| has_steam_api_key(state.secrets())).await
}

/// Stores the user's own Steam Web API key in the Windows Credential Manager (32 hex characters).
#[tauri::command]
pub async fn save_steam_key(app: AppHandle, key: String) -> CommandResult<()> {
    blocking(&app, move |state| save_steam_api_key(state.secrets(), &key)).await
}

#[tauri::command]
pub async fn clear_steam_key(app: AppHandle) -> CommandResult<()> {
    blocking(&app, |state| clear_steam_api_key(state.secrets())).await
}

/// Opens Steam's API-key page in the default browser. The address is fixed here, never supplied by the
/// WebView.
#[tauri::command]
pub async fn open_steam_api_key_page(app: AppHandle) -> CommandResult<()> {
    app.opener()
        .open_url("https://steamcommunity.com/dev/apikey", None::<&str>)
        .map_err(|error| CommandError::new(CommandErrorCode::Io, error.to_string()))
}

/// What the last owned-games lookup found, for the hint under the Game Libraries setting.
#[tauri::command]
pub async fn get_owned_library_status(state: State<'_, AppState>) -> CommandResult<OwnedLibraryStatus> {
    Ok(state.owned_library_status())
}

#[tauri::command]
pub async fn list_games(state: State<'_, AppState>) -> CommandResult<Vec<GameSummary>> {
    Ok(LibraryRepository::new(&state.db).list()?)
}

#[tauri::command]
pub async fn get_game(state: State<'_, AppState>, game_id: i64) -> CommandResult<GameDetail> {
    LibraryRepository::new(&state.db)
        .detail(game_id)?
        .ok_or_else(|| CoreError::NotFound(format!("game {game_id}")).into())
}

#[tauri::command]
pub async fn set_favorite(
    app: AppHandle,
    state: State<'_, AppState>,
    game_id: i64,
    favorite: bool,
) -> CommandResult<GameSummary> {
    let game = LibraryRepository::new(&state.db).set_favorite(game_id, favorite)?;
    events::library_changed(&app);
    Ok(game)
}

#[tauri::command]
pub async fn add_standalone_game(
    app: AppHandle,
    executable_path: String,
    title: Option<String>,
) -> CommandResult<GameSummary> {
    let game = blocking(&app, move |state| {
        let game =
            LibraryRepository::new(&state.db).add_standalone(&PathBuf::from(executable_path), title.as_deref())?;
        let mut games = [game];
        reclassify_new_games(state, &mut games);
        let [game] = games;
        Ok(game)
    })
    .await?;
    super::artwork::fetch_for_new_game(&app, game.id);
    events::library_changed(&app);
    Ok(game)
}

#[tauri::command]
pub async fn get_running_games(state: State<'_, AppState>) -> CommandResult<Vec<i64>> {
    Ok(state.tracker()?.running())
}

/// Starts a game through its launcher (Steam/Epic URI, Xbox package) or its executable.
#[tauri::command]
pub async fn launch_game(app: AppHandle, state: State<'_, AppState>, game_id: i64) -> CommandResult<()> {
    let target = LibraryRepository::new(&state.db).launch_target(game_id)?;
    // A game that is not installed is only sent to its store's install page: nothing to back up,
    // and no play session to expect.
    if target.installed {
        pre_launch_snapshot(&app, game_id).await;
    }
    let plan = plan_launch(&target)?;
    match &plan {
        LaunchPlan::Uri(uri) => app
            .opener()
            .open_url(uri, None::<&str>)
            .map_err(|error| CommandError::new(CommandErrorCode::Launch, format!("could not open {uri}: {error}")))?,
        LaunchPlan::AppPackage(id) => {
            // The Start menu's own way to start an installed package (also a Game Pass game).
            std::process::Command::new("explorer.exe").arg(format!("shell:AppsFolder\\{id}")).spawn().map_err(
                |error| CommandError::new(CommandErrorCode::Launch, format!("could not start {id}: {error}")),
            )?;
        }
        LaunchPlan::Executable { path, working_dir } => {
            std::process::Command::new(path).current_dir(working_dir).spawn().map_err(|error| {
                CommandError::new(CommandErrorCode::Launch, format!("could not start {}: {error}", path.display()))
            })?;
        }
    }
    if target.installed {
        state.tracker()?.note_launch(game_id);
    }
    tracing::info!(game_id, installed = target.installed, ?plan, "game launch requested");
    Ok(())
}

#[tauri::command]
pub async fn open_install_folder(app: AppHandle, state: State<'_, AppState>, game_id: i64) -> CommandResult<()> {
    let target = LibraryRepository::new(&state.db).launch_target(game_id)?;
    let folder = target
        .install_path
        .filter(|path| path.is_dir())
        .ok_or_else(|| CoreError::Validation("the install folder no longer exists".into()))?;
    app.opener().open_path(folder.to_string_lossy(), None::<&str>).map_err(|error| {
        CommandError::new(CommandErrorCode::Io, format!("could not open {}: {error}", folder.display()))
    })
}

/// Optional snapshot before launching. A failure is reported but never blocks the launch.
async fn pre_launch_snapshot(app: &AppHandle, game_id: i64) {
    let app = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        match SettingsRepository::new(&state.db).load() {
            Ok(settings) if settings.snapshot_before_launch => {
                saves_service::automatic_snapshot_and_notify(&app, &state, game_id, SnapshotKind::AutoLaunch);
            }
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "could not read settings; skipping the pre-launch snapshot"),
        }
    })
    .await;
    if let Err(error) = result {
        tracing::warn!(%error, "pre-launch snapshot task failed");
    }
}

/// Scans a folder for games that are not yet in the library (read-only).
#[tauri::command]
pub async fn scan_game_folder(app: AppHandle, folder: String) -> CommandResult<FolderScanReport> {
    blocking(&app, move |state| l8_core::library::scan_game_folder(&state.db, &PathBuf::from(folder))).await
}

/// Adds the games chosen after a folder scan. A game whose executable is already in the library
/// is skipped, so repeating a scan or a click never creates duplicates.
#[tauri::command]
pub async fn add_games(app: AppHandle, games: Vec<NewGameInput>) -> CommandResult<AddGamesReport> {
    if games.len() > 300 {
        return Err(CoreError::Validation("too many games at once".into()).into());
    }
    let report = blocking(&app, move |state| {
        let repository = LibraryRepository::new(&state.db);
        let mut report = AddGamesReport { added: Vec::new(), skipped: 0, failed: 0 };
        for game in games {
            let outcome = repository.add_standalone_in(
                Some(&PathBuf::from(&game.folder)),
                &PathBuf::from(&game.executable),
                Some(&game.title),
            );
            match outcome {
                Ok(added) if added.created => report.added.push(added.game),
                Ok(_) => report.skipped += 1,
                Err(error) => {
                    tracing::warn!(%error, executable = %game.executable, "a scanned game could not be added");
                    report.failed += 1;
                }
            }
        }
        reclassify_new_games(state, &mut report.added);
        Ok(report)
    })
    .await?;
    for game in &report.added {
        super::artwork::fetch_for_new_game(&app, game.id);
    }
    events::library_changed(&app);
    Ok(report)
}
