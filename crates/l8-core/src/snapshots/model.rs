//! Snapshot types shared with the frontend.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SnapshotKind {
    Manual,
    AutoExit,
    AutoLaunch,
    Scheduled,
    Safety,
    ConflictCopy,
}

impl SnapshotKind {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::AutoExit => "auto_exit",
            Self::AutoLaunch => "auto_launch",
            Self::Scheduled => "scheduled",
            Self::Safety => "safety",
            Self::ConflictCopy => "conflict_copy",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "auto_exit" => Self::AutoExit,
            "auto_launch" => Self::AutoLaunch,
            "scheduled" => Self::Scheduled,
            "safety" => Self::Safety,
            "conflict_copy" => Self::ConflictCopy,
            _ => Self::Manual,
        }
    }

    /// Automatic snapshots are the only ones retention may remove.
    pub fn is_automatic(self) -> bool {
        matches!(self, Self::AutoExit | Self::AutoLaunch | Self::Scheduled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SnapshotStatus {
    Ready,
    Corrupt,
    MissingLocal,
}

impl SnapshotStatus {
    pub fn from_db(value: &str, archive_exists: bool) -> Self {
        match value {
            "corrupt" => Self::Corrupt,
            _ if !archive_exists => Self::MissingLocal,
            _ => Self::Ready,
        }
    }
}

/// Where a snapshot is meant to live, chosen when it is created.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SnapshotStorage {
    /// This PC, and Google Drive once it is connected.
    #[default]
    Both,
    /// This PC only: never uploaded.
    Local,
    /// Google Drive only: the local archive is removed once Drive confirms the upload.
    Cloud,
}

impl SnapshotStorage {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Local => "local",
            Self::Cloud => "cloud",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "local" => Self::Local,
            "cloud" => Self::Cloud,
            _ => Self::Both,
        }
    }
}

/// Where a snapshot stands in the cloud (Google Drive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CloudState {
    /// Local only.
    None,
    Queued,
    Uploading,
    Failed,
    /// A verified copy is in Drive.
    Synced,
}

impl CloudState {
    pub fn from_db(value: &str) -> Self {
        match value {
            "synced" => Self::Synced,
            "uploading" => Self::Uploading,
            "queued" => Self::Queued,
            "failed" => Self::Failed,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SnapshotSummary {
    pub id: String,
    #[ts(type = "number")]
    pub game_id: i64,
    pub game_title: String,
    /// Empty for automatic snapshots: the UI shows a translated label for the kind.
    pub name: String,
    pub description: Option<String>,
    pub kind: SnapshotKind,
    pub status: SnapshotStatus,
    pub created_at: String,
    pub verified_at: Option<String>,
    pub archive_path: Option<String>,
    #[ts(type = "number")]
    pub archive_size_bytes: i64,
    #[ts(type = "number")]
    pub total_files: i64,
    #[ts(type = "number")]
    pub total_bytes: i64,
    pub archive_sha256: String,
    #[ts(type = "number | null")]
    pub playtime_seconds: Option<i64>,
    pub pinned: bool,
    pub device_name: String,
    pub cloud: CloudState,
}

/// What a restore did, for the result message.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RestoreReport {
    pub restored_snapshot_id: String,
    /// The automatic safety copy of the save that was replaced (`None` if it was identical to an
    /// existing snapshot or there was nothing to protect).
    pub safety_snapshot_id: Option<String>,
    #[ts(type = "number")]
    pub files_restored: i64,
}

/// The save on disk right now, compared with the latest snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CurrentSaveState {
    #[ts(type = "number")]
    pub total_files: i64,
    #[ts(type = "number")]
    pub total_bytes: i64,
    pub last_modified_at: Option<String>,
    pub latest_snapshot_id: Option<String>,
    /// `true` when the current files are byte-identical to the latest snapshot.
    pub matches_latest: bool,
}

/// Per-game protection figures for the library, dashboard and rails.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameSaveStatus {
    #[ts(type = "number")]
    pub game_id: i64,
    #[ts(type = "number")]
    pub location_count: i64,
    #[ts(type = "number")]
    pub snapshot_count: i64,
    pub last_snapshot_at: Option<String>,
    /// Ludusavi looked the game up and does not know it.
    pub unknown_to_ludusavi: bool,
}

/// Outcome of a snapshot request.
#[derive(Debug, Clone, PartialEq)]
pub enum CreateOutcome {
    Created(Box<SnapshotSummary>),
    /// The save is identical to the latest snapshot; nothing new was written.
    Unchanged {
        latest_snapshot_id: String,
    },
    /// The game has save locations but no file in them yet.
    NothingToBackUp,
}
