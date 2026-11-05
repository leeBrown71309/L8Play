//! Save operations shared by commands and background work (detection, automatic snapshots).

use l8_core::saves::{SaveLocationRepository, detect_save_locations, games_needing_detection, remembered_title};
use l8_core::settings::SettingsRepository;
use l8_core::snapshots::{
    CreateOutcome, CreateRequest, RetentionPolicy, SnapshotEngine, SnapshotKind, SnapshotStorage,
};
use l8_core::{CoreError, CoreResult};
use tauri::AppHandle;

use crate::dto::AutoSnapshotEvent;
use crate::events;
use crate::state::AppState;

/// Takes an automatic snapshot (skipped when identical to the latest one), then applies retention.
/// Detects save locations first if the game was never looked up.
pub fn automatic_snapshot(state: &AppState, game_id: i64, kind: SnapshotKind) -> CoreResult<CreateOutcome> {
    let _guard = state.snapshot_lock()?;
    if SaveLocationRepository::new(&state.db).list(game_id)?.is_empty() {
        if remembered_title(&state.db, game_id)?.is_none() && state.ludusavi.is_available() {
            detect_save_locations(&state.db, &state.ludusavi, game_id)?;
        }
        if SaveLocationRepository::new(&state.db).list(game_id)?.is_empty() {
            return Ok(CreateOutcome::NothingToBackUp);
        }
    }
    let settings = SettingsRepository::new(&state.db).load()?;
    let engine = SnapshotEngine::new(&state.db, state.snapshot_root()?);
    let outcome = engine.create(&CreateRequest {
        game_id,
        name: String::new(),
        description: None,
        kind,
        pinned: false,
        skip_if_unchanged: true,
        storage: SnapshotStorage::Both,
    })?;
    if matches!(outcome, CreateOutcome::Created(_)) {
        let policy = RetentionPolicy { keep_local: settings.keep_automatic, keep_cloud: settings.keep_cloud };
        let report = engine.apply_retention(game_id, policy)?;
        if report.removed_local + report.removed_cloud > 0 {
            tracing::info!(
                game_id,
                removed_local = report.removed_local,
                removed_cloud = report.removed_cloud,
                "old automatic snapshots removed by retention"
            );
        }
    }
    Ok(outcome)
}

/// Runs an automatic snapshot and reports the result to the UI.
pub fn automatic_snapshot_and_notify(app: &AppHandle, state: &AppState, game_id: i64, kind: SnapshotKind) {
    let event = match automatic_snapshot(state, game_id, kind) {
        Ok(CreateOutcome::Created(summary)) => {
            crate::cloud_service::enqueue_upload(app, &summary.id);
            AutoSnapshotEvent::created(game_id, kind, summary.id)
        }
        Ok(CreateOutcome::Unchanged { .. }) => AutoSnapshotEvent::status(game_id, kind, "unchanged"),
        Ok(CreateOutcome::NothingToBackUp) => AutoSnapshotEvent::status(game_id, kind, "nothing"),
        Err(error) => {
            tracing::warn!(game_id, %error, "automatic snapshot failed");
            let status = if matches!(error, CoreError::Busy(_)) { "busy" } else { "failed" };
            AutoSnapshotEvent::failed(game_id, kind, status, error.to_string())
        }
    };
    if event.status == "failed" {
        crate::shell::notify_attention(app, crate::shell::Notice::SnapshotFailed);
    }
    events::saves_changed(app, game_id);
    events::auto_snapshot(app, event);
}

/// Background pass: looks up every installed game Ludusavi has not seen yet.
pub fn detect_new_games(app: &AppHandle, state: &AppState) {
    if !state.ludusavi.is_available() {
        tracing::warn!("Ludusavi sidecar missing; automatic save detection is disabled (run `bun run fetch:ludusavi`)");
        return;
    }
    let games = match games_needing_detection(&state.db) {
        Ok(games) => games,
        Err(error) => {
            tracing::warn!(%error, "could not list games needing detection");
            return;
        }
    };
    for game_id in games {
        match detect_save_locations(&state.db, &state.ludusavi, game_id) {
            Ok(_) => events::saves_changed(app, game_id),
            Err(error) => tracing::warn!(game_id, %error, "save detection failed"),
        }
    }
}
