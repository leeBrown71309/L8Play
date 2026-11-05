//! Detection pipeline for one game (layer 1: Ludusavi).

use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

use crate::db::Database;
use crate::error::CoreResult;
use crate::library::LibraryRepository;
use crate::platforms::Platform;
use crate::saves::locations::{broad_roots_from_environment, derive_locations};
use crate::saves::ludusavi::LudusaviCli;
use crate::saves::repository::{SaveLocation, SaveLocationRepository};

const LUDUSAVI_PROVIDER: &str = "ludusavi";
/// Marker stored when Ludusavi does not know a game, so automatic detection does not retry it.
const UNKNOWN_TITLE: &str = "";

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "status")]
#[ts(export)]
pub enum DetectionOutcome {
    /// Locations found (possibly none on disk yet if the game was never played).
    Found { manifest_title: String, locations: Vec<SaveLocation> },
    /// Ludusavi's manifest does not know this game; the user can add a folder by hand.
    Unknown { locations: Vec<SaveLocation> },
}

/// Runs detection and stores the result. User-added and user-rejected locations are preserved.
pub fn detect_save_locations(db: &Database, cli: &LudusaviCli, game_id: i64) -> CoreResult<DetectionOutcome> {
    let library = LibraryRepository::new(db);
    let target = library.launch_target(game_id)?;
    let summary = library.summary(game_id)?.ok_or_else(|| crate::CoreError::NotFound(format!("game {game_id}")))?;
    let repository = SaveLocationRepository::new(db);

    let title = match (target.platform, target.platform_game_id.as_deref()) {
        (Platform::Steam, Some(app_id)) => {
            cli.find_by_steam_id(app_id)?.map_or_else(|| cli.find_by_name(&summary.title), |found| Ok(Some(found)))?
        }
        _ => cli.find_by_name(&summary.title)?,
    };
    remember_title(db, game_id, title.as_deref().unwrap_or(UNKNOWN_TITLE))?;

    let Some(manifest_title) = title else {
        tracing::info!(game_id, title = %summary.title, "game unknown to Ludusavi");
        return Ok(DetectionOutcome::Unknown { locations: repository.list(game_id)? });
    };
    let files = cli.preview_files(&manifest_title)?;
    let derived = derive_locations(&files, target.install_path.as_deref(), &broad_roots_from_environment());
    repository.replace_detected(game_id, LUDUSAVI_PROVIDER, &derived)?;
    tracing::info!(game_id, manifest_title, files = files.len(), locations = derived.len(), "save locations detected");
    Ok(DetectionOutcome::Found { manifest_title, locations: repository.list(game_id)? })
}

fn remember_title(db: &Database, game_id: i64, title: &str) -> CoreResult<()> {
    db.connection()?.execute(
        "INSERT INTO game_external_ids (game_id, provider, external_id) VALUES (?1, ?2, ?3)
         ON CONFLICT(game_id, provider) DO UPDATE SET external_id = excluded.external_id",
        params![game_id, LUDUSAVI_PROVIDER, title],
    )?;
    Ok(())
}

/// Installed games that were never looked up in Ludusavi (for the background detection pass).
pub fn games_needing_detection(db: &Database) -> CoreResult<Vec<i64>> {
    let connection = db.connection()?;
    let mut statement = connection.prepare(
        "SELECT DISTINCT g.id FROM games g JOIN installations i ON i.game_id = g.id AND i.is_installed = 1
         WHERE NOT EXISTS (SELECT 1 FROM game_external_ids x WHERE x.game_id = g.id AND x.provider = ?1)",
    )?;
    let ids = statement.query_map([LUDUSAVI_PROVIDER], |row| row.get(0))?.collect::<Result<Vec<i64>, _>>()?;
    Ok(ids)
}

/// The Ludusavi title remembered for a game (`Some("")` = known to be unknown).
pub fn remembered_title(db: &Database, game_id: i64) -> CoreResult<Option<String>> {
    Ok(db
        .connection()?
        .query_row(
            "SELECT external_id FROM game_external_ids WHERE game_id = ?1 AND provider = ?2",
            params![game_id, LUDUSAVI_PROVIDER],
            |row| row.get(0),
        )
        .optional()?)
}
