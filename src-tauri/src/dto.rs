//! Data sent to the frontend that has no home in `l8-core`. Exported to `src/lib/bindings`.

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppInfo {
    pub version: String,
    pub database_path: String,
    pub schema_version: u32,
    /// The bundled Ludusavi sidecar was found next to the app.
    pub ludusavi_available: bool,
}

/// Payload of the `game://session` event.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionEvent {
    #[ts(type = "number")]
    pub game_id: i64,
    pub running: bool,
    /// Set when a session ends.
    #[ts(type = "number | null")]
    pub duration_seconds: Option<i64>,
}

/// Result of a manual "Create snapshot".
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "status")]
#[ts(export)]
pub enum CreateSnapshotResult {
    Created {
        snapshot: Box<l8_core::snapshots::SnapshotSummary>,
    },
    /// Identical to the latest snapshot (only reported for automatic snapshots).
    Unchanged {
        #[serde(rename = "latestSnapshotId")]
        latest_snapshot_id: String,
    },
    /// The save locations hold no file yet.
    NothingToBackUp,
}

impl From<l8_core::snapshots::CreateOutcome> for CreateSnapshotResult {
    fn from(outcome: l8_core::snapshots::CreateOutcome) -> Self {
        match outcome {
            l8_core::snapshots::CreateOutcome::Created(snapshot) => Self::Created { snapshot },
            l8_core::snapshots::CreateOutcome::Unchanged { latest_snapshot_id } => {
                Self::Unchanged { latest_snapshot_id }
            }
            l8_core::snapshots::CreateOutcome::NothingToBackUp => Self::NothingToBackUp,
        }
    }
}

/// Payload of the `saves://changed` event.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavesChangedEvent {
    #[ts(type = "number")]
    pub game_id: i64,
}

/// Payload of the `snapshot://auto` event (automatic snapshot outcome, for a toast).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AutoSnapshotEvent {
    #[ts(type = "number")]
    pub game_id: i64,
    pub kind: l8_core::snapshots::SnapshotKind,
    /// `created`, `unchanged`, `nothing`, `busy` or `failed`.
    pub status: String,
    pub snapshot_id: Option<String>,
    pub message: Option<String>,
}

impl AutoSnapshotEvent {
    pub fn created(game_id: i64, kind: l8_core::snapshots::SnapshotKind, snapshot_id: String) -> Self {
        Self { game_id, kind, status: "created".into(), snapshot_id: Some(snapshot_id), message: None }
    }

    pub fn status(game_id: i64, kind: l8_core::snapshots::SnapshotKind, status: &str) -> Self {
        Self { game_id, kind, status: status.into(), snapshot_id: None, message: None }
    }

    pub fn failed(game_id: i64, kind: l8_core::snapshots::SnapshotKind, status: &str, message: String) -> Self {
        Self { game_id, kind, status: status.into(), snapshot_id: None, message: Some(message) }
    }
}

/// Payload of the `cloud://changed` event.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CloudChangedEvent {
    pub new_conflicts: u32,
}

/// One game chosen in the "scan a folder" dialog.
#[derive(Debug, Clone, serde::Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NewGameInput {
    /// The game's own folder.
    pub folder: String,
    /// The executable the user picked to launch it.
    pub executable: String,
    pub title: String,
}

/// What adding several games at once did.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddGamesReport {
    pub added: Vec<l8_core::library::GameSummary>,
    /// Games that were already in the library.
    pub skipped: u32,
    /// Games that could not be added (folder or executable no longer valid).
    pub failed: u32,
}
