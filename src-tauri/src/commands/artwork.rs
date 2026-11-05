//! Artwork commands: SteamGridDB key, candidate search, choosing, importing, resetting.
//! The API key travels from the WebView to the Credential Manager once and is never sent back.

use std::path::PathBuf;

use l8_core::artwork::{ArtworkCandidate, ArtworkFetchReport, ArtworkKind, ArtworkSelection};
use l8_core::library::{GameSummary, LibraryRepository, MetadataPatch};
use l8_core::{CoreError, CoreResult};
use tauri::{AppHandle, Manager};

use crate::error::{CommandError, CommandErrorCode, CommandResult};
use crate::events;
use crate::state::AppState;

/// Runs blocking artwork work (network, disk) off the async runtime.
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
pub async fn get_artwork_key_status(app: AppHandle) -> CommandResult<bool> {
    blocking(&app, |state| state.artwork().has_key()).await
}

#[tauri::command]
pub async fn save_artwork_key(app: AppHandle, key: String) -> CommandResult<()> {
    blocking(&app, move |state| state.artwork().save_key(&key)).await
}

#[tauri::command]
pub async fn clear_artwork_key(app: AppHandle) -> CommandResult<()> {
    blocking(&app, |state| state.artwork().clear_key()).await
}

/// Proves the saved key works by asking SteamGridDB one question.
#[tauri::command]
pub async fn verify_artwork_key(app: AppHandle) -> CommandResult<()> {
    blocking(&app, |state| state.artwork().verify_key()).await
}

#[tauri::command]
pub async fn get_artwork_selections(app: AppHandle, game_id: i64) -> CommandResult<Vec<ArtworkSelection>> {
    blocking(&app, move |state| state.artwork().selections(game_id)).await
}

#[tauri::command]
pub async fn find_artwork(
    app: AppHandle,
    game_id: i64,
    kind: ArtworkKind,
    query: Option<String>,
) -> CommandResult<Vec<ArtworkCandidate>> {
    blocking(&app, move |state| state.artwork().find_candidates(game_id, kind, query.as_deref())).await
}

#[tauri::command]
pub async fn apply_artwork(app: AppHandle, game_id: i64, candidate: ArtworkCandidate) -> CommandResult<()> {
    blocking(&app, move |state| state.artwork().apply_candidate(game_id, &candidate)).await?;
    events::library_changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn import_artwork(app: AppHandle, game_id: i64, kind: ArtworkKind, path: String) -> CommandResult<()> {
    blocking(&app, move |state| state.artwork().import_file(game_id, kind, &PathBuf::from(path))).await?;
    events::library_changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn reset_artwork(app: AppHandle, game_id: i64, kind: ArtworkKind) -> CommandResult<()> {
    blocking(&app, move |state| state.artwork().reset(game_id, kind)).await?;
    events::library_changed(&app);
    Ok(())
}

/// "Find missing artwork" for every Epic and standalone game.
#[tauri::command]
pub async fn fetch_missing_artwork(app: AppHandle) -> CommandResult<ArtworkFetchReport> {
    let report = blocking(&app, |state| state.artwork().fetch_missing()).await?;
    if report.fetched > 0 {
        events::library_changed(&app);
    }
    Ok(report)
}

#[tauri::command]
pub async fn update_game_metadata(
    app: AppHandle,
    game_id: i64,
    title: String,
    developer: Option<String>,
    release_year: Option<i64>,
) -> CommandResult<GameSummary> {
    let game = blocking(&app, move |state| {
        LibraryRepository::new(&state.db).update_metadata(game_id, &MetadataPatch { title, developer, release_year })
    })
    .await?;
    events::library_changed(&app);
    Ok(game)
}

/// Looks up artwork for a game just added by the user, in the background (no key: no-op).
pub fn fetch_for_new_game(app: &AppHandle, game_id: i64) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        match state.artwork().fetch_missing_for_game(game_id) {
            Ok(report) if report.fetched > 0 => events::library_changed(&app),
            Ok(_) => {}
            Err(CoreError::Auth(_)) => tracing::warn!("artwork lookup skipped: the saved API key was refused"),
            Err(error) => tracing::warn!(game_id, %error, "artwork lookup failed"),
        }
        // Whatever the outcome, the UI stops showing its loading state for this game.
        events::artwork_fetched(&app, game_id);
    });
}

/// Opens SteamGridDB's API-key page in the default browser. The address is fixed here, never
/// supplied by the WebView.
#[tauri::command]
pub async fn open_artwork_key_page(app: AppHandle) -> CommandResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url("https://www.steamgriddb.com/profile/preferences/api", None::<&str>)
        .map_err(|error| CommandError::new(CommandErrorCode::Io, error.to_string()))
}
