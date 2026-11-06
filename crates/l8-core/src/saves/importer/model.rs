//! Types exchanged with the frontend by the import flow.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What the chosen folder contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ScanMode {
    /// Decide from the folder's shape.
    #[default]
    Auto,
    /// A folder that holds one sub-folder per game (`Save Games/<game>/…`).
    GamesFolder,
    /// The folder is the backup of a single game.
    OneGame,
}

/// How one game's backup is organised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PlannedLayout {
    /// The whole folder is one snapshot.
    Single,
    /// Each sub-folder is a named snapshot (`after ending A`, `before final choice`…).
    Variants,
}

/// One snapshot that importing a game will create.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlannedSnapshot {
    /// Name of the snapshot (the sub-folder it comes from); `None` for a single unnamed backup.
    pub label: Option<String>,
    /// The folder whose content becomes the snapshot.
    pub root: String,
    #[ts(type = "number")]
    pub files: u32,
    #[ts(type = "number")]
    pub bytes: u64,
    /// Date of the newest file, which dates the snapshot.
    pub modified_at: Option<String>,
    /// Suggested restore folder read from a hint file, e.g. `%LOCALAPPDATA%\Game\Saved\SaveGames`.
    pub hint: Option<String>,
    /// The hint file inside `root` that must not be imported as a save.
    pub hint_file: Option<String>,
}

/// One game found in the chosen folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlannedGame {
    /// Stable id of the row: the folder's path relative to the scanned folder.
    pub key: String,
    pub folder: String,
    pub folder_name: String,
    pub guessed_title: String,
    /// The game's name in the Ludusavi database when one of the guessed names matched it.
    pub suggested_title: Option<String>,
    /// Other names found inside the backup (`DARK SOULS REMASTERED` for a folder called `DSR`).
    pub alt_titles: Vec<String>,
    pub layout: PlannedLayout,
    pub snapshots: Vec<PlannedSnapshot>,
    /// The collection folder it was found in (`save`, `My Games`…), if any.
    pub from_collection: Option<String>,
    /// Key of an earlier row with the same title: both would become snapshots of the same game.
    pub duplicate_of: Option<String>,
    /// Logs, crash dumps and screenshots left out.
    #[ts(type = "number")]
    pub skipped_files: u32,
    /// Sub-folders that would have been snapshots but hold no file (so nothing is imported for them).
    pub empty_snapshots: Vec<String>,
}

/// Everything found in the chosen folder, before anything is imported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportPlan {
    pub source: String,
    /// How the folder was read (`Auto` is resolved to one of the other two).
    pub mode: ScanMode,
    pub games: Vec<PlannedGame>,
    /// Loose files next to the game folders that were ignored, and similar remarks.
    pub warnings: Vec<String>,
}

/// One game to import, as confirmed by the user in the review step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportGameRequest {
    pub title: String,
    pub snapshots: Vec<PlannedSnapshot>,
}

/// What happened to one game.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportedGame {
    #[ts(type = "number | null")]
    pub game_id: Option<i64>,
    pub title: String,
    /// `false` when the title matched a game already in the library.
    pub created: bool,
    pub imported: u32,
    /// Ids of the snapshots created (they are queued for upload to Drive afterwards).
    pub snapshot_ids: Vec<String>,
    /// Snapshots skipped because the game already has exactly those files.
    pub duplicates: u32,
    pub errors: Vec<String>,
}

/// The outcome of an import, game by game.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportReport {
    pub games: Vec<ImportedGame>,
}

/// Progress event sent while games are imported.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportProgress {
    #[ts(type = "number")]
    pub done: u32,
    #[ts(type = "number")]
    pub total: u32,
    pub current_title: String,
}
