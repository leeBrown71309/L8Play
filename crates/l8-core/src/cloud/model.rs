//! Cloud types shared with the frontend.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AccountStatus {
    Connected,
    /// The refresh token was revoked or expired: the user must sign in again.
    NeedsReauth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CloudAccountInfo {
    pub email: String,
    pub status: AccountStatus,
    pub connected_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CloudQuota {
    #[ts(type = "number")]
    pub used_bytes: i64,
    /// `None` for unlimited accounts.
    #[ts(type = "number | null")]
    pub limit_bytes: Option<i64>,
}

/// Everything the Cloud page shows at a glance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CloudStatus {
    /// Which Google client sign-in uses: the user's own, the one built into this app, or none yet.
    pub client_source: crate::cloud::client::ClientSource,
    pub account: Option<CloudAccountInfo>,
    pub quota: Option<CloudQuota>,
    pub queued: u32,
    pub running: u32,
    pub failed: u32,
    pub open_conflicts: u32,
    pub last_sync_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SyncDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SyncStatus {
    Queued,
    Running,
    Failed,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncOperationInfo {
    #[ts(type = "number")]
    pub id: i64,
    pub snapshot_id: String,
    pub game_title: String,
    pub snapshot_name: String,
    pub direction: SyncDirection,
    pub status: SyncStatus,
    #[ts(type = "number")]
    pub attempts: i64,
    #[ts(type = "number")]
    pub bytes_done: i64,
    #[ts(type = "number")]
    pub bytes_total: i64,
    pub last_error: Option<String>,
    pub next_attempt_at: Option<String>,
}

/// One side of a sync conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConflictSide {
    pub snapshot_id: String,
    pub name: String,
    pub created_at: String,
    pub device_name: String,
    #[ts(type = "number")]
    pub total_files: i64,
    #[ts(type = "number")]
    pub total_bytes: i64,
    #[ts(type = "number | null")]
    pub playtime_seconds: Option<i64>,
    /// The archive is on this PC (otherwise it must be downloaded first).
    pub local_copy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConflictInfo {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub game_id: i64,
    pub game_title: String,
    pub detected_at: String,
    /// The newest save made on this PC.
    pub local: ConflictSide,
    /// The newest save made on another device.
    pub remote: ConflictSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ConflictResolution {
    /// Keep this PC's save; the other device's copy stays in Drive, marked as a conflict copy.
    UseLocal,
    /// Restore the other device's save here (the current one is backed up first).
    UseRemote,
    /// Keep this PC's save and also download the other device's copy.
    KeepBoth,
}

impl ConflictResolution {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::UseLocal => "use_local",
            Self::UseRemote => "use_remote",
            Self::KeepBoth => "keep_both",
        }
    }
}

/// What one full synchronisation did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncReport {
    pub uploaded: u32,
    pub downloaded: u32,
    pub failed: u32,
    /// Snapshots found in Drive that this PC did not know.
    pub imported: u32,
    pub new_conflicts: u32,
    /// Drive snapshots of games that are not in this PC's library (left alone).
    pub skipped_unknown_games: u32,
}
