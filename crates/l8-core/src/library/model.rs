//! Library types shared with the frontend (exported to TypeScript).

use std::path::PathBuf;

use serde::Serialize;
use ts_rs::TS;

use crate::library::ArtworkUrls;
use crate::platforms::Platform;
use crate::platforms::steam_owned::OwnedLibraryStatus;

/// A game as listed in the library grid, rows, rails and dashboard.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameSummary {
    #[ts(type = "number")]
    pub id: i64,
    pub title: String,
    pub platform: Platform,
    pub platform_game_id: Option<String>,
    pub installed: bool,
    /// Saves-only game: imported from an older backup, never launched or installed by L8Play.
    pub archived: bool,
    pub favorite: bool,
    pub last_played_at: Option<String>,
    #[ts(type = "number")]
    pub playtime_seconds: i64,
    #[ts(type = "number | null")]
    pub size_bytes: Option<i64>,
    pub install_path: Option<String>,
    pub artwork: ArtworkUrls,
}

/// Everything the game details page shows.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameDetail {
    pub game: GameSummary,
    pub developer: Option<String>,
    #[ts(type = "number | null")]
    pub release_year: Option<i64>,
    pub executable_path: Option<String>,
    pub added_at: String,
    #[ts(type = "number")]
    pub session_count: i64,
}

/// Outcome of one platform during a scan.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlatformScan {
    pub platform: Platform,
    /// `None` when the launcher is not installed on this PC.
    pub found: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScanReport {
    pub added: u32,
    pub updated: u32,
    pub uninstalled: u32,
    pub total: u32,
    pub platforms: Vec<PlatformScan>,
    pub warnings: Vec<String>,
    /// Outcome of listing the Steam games the account owns but has not installed.
    pub owned_library: OwnedLibraryStatus,
    /// Owned, not installed Steam games added to the library by that lookup.
    pub owned_added: u32,
}

/// What the launcher needs to start a game.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchTarget {
    pub game_id: i64,
    pub platform: Platform,
    pub platform_game_id: Option<String>,
    pub installed: bool,
    pub executable_path: Option<PathBuf>,
    pub install_path: Option<PathBuf>,
}

/// An installed game whose processes the session monitor looks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchedGame {
    pub game_id: i64,
    pub install_path: PathBuf,
}
