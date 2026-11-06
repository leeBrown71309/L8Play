//! Event names shared with the frontend (`src/lib/tauri/events.ts` mirrors them).

use tauri::{AppHandle, Emitter};

use crate::dto::SessionEvent;

/// The library changed (scan, manual game, favourite, playtime): refetch game lists.
pub const LIBRARY_CHANGED: &str = "library://changed";
/// A game started or stopped running.
pub const GAME_SESSION: &str = "game://session";

/// The background artwork lookup for a game just added by the user ended (payload: the game id).
pub const ARTWORK_FETCHED: &str = "artwork://fetched";

pub fn artwork_fetched(app: &AppHandle, game_id: i64) {
    if let Err(error) = app.emit(ARTWORK_FETCHED, game_id) {
        tracing::warn!(%error, "could not emit artwork lookup result");
    }
}

pub fn library_changed(app: &AppHandle) {
    if let Err(error) = app.emit(LIBRARY_CHANGED, ()) {
        tracing::warn!(%error, "could not emit library change");
    }
}

pub fn game_session(app: &AppHandle, event: SessionEvent) {
    if let Err(error) = app.emit(GAME_SESSION, event) {
        tracing::warn!(%error, "could not emit session event");
    }
}

/// Progress of an import of old backups (payload: `ImportProgress`).
pub const IMPORT_PROGRESS: &str = "import://progress";

/// Save locations or snapshots of a game changed: refetch them.
pub const SAVES_CHANGED: &str = "saves://changed";
/// An automatic snapshot finished (created, skipped or failed).
pub const AUTO_SNAPSHOT: &str = "snapshot://auto";

/// Cloud state changed (upload finished, import, conflict, sign-in needed).
pub const CLOUD_CHANGED: &str = "cloud://changed";

pub fn cloud_changed(app: &AppHandle, new_conflicts: u32) {
    if let Err(error) = app.emit(CLOUD_CHANGED, crate::dto::CloudChangedEvent { new_conflicts }) {
        tracing::warn!(%error, "could not emit cloud change");
    }
    if new_conflicts > 0 {
        crate::shell::notify_attention(app, crate::shell::Notice::ConflictFound);
    }
}

pub fn saves_changed(app: &AppHandle, game_id: i64) {
    if let Err(error) = app.emit(SAVES_CHANGED, crate::dto::SavesChangedEvent { game_id }) {
        tracing::warn!(%error, "could not emit saves change");
    }
}

pub fn auto_snapshot(app: &AppHandle, event: crate::dto::AutoSnapshotEvent) {
    if let Err(error) = app.emit(AUTO_SNAPSHOT, event) {
        tracing::warn!(%error, "could not emit automatic snapshot result");
    }
}
