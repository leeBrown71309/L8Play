//! Persistence of save locations (`save_locations` table).

use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

use crate::clock::now_rfc3339;
use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::saves::locations::{DerivedLocation, LocationKind, normalize_path};

/// A save location as shown in the Saves tab.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SaveLocation {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub game_id: i64,
    pub path: String,
    pub kind: LocationKind,
    /// `ludusavi` (detected) or `user` (added by hand).
    pub source: String,
    /// The path exists right now.
    pub exists: bool,
}

impl SaveLocation {
    pub fn path_buf(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }
}

pub struct SaveLocationRepository<'db> {
    db: &'db Database,
}

impl<'db> SaveLocationRepository<'db> {
    pub fn new(db: &'db Database) -> Self {
        Self { db }
    }

    /// Active (non-rejected) locations of a game.
    pub fn list(&self, game_id: i64) -> CoreResult<Vec<SaveLocation>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, game_id, resolved_path, kind, source FROM save_locations
             WHERE game_id = ?1 AND status <> 'rejected' ORDER BY resolved_path",
        )?;
        let locations = statement
            .query_map([game_id], |row| {
                let path: String = row.get(2)?;
                Ok(SaveLocation {
                    id: row.get(0)?,
                    game_id: row.get(1)?,
                    exists: Path::new(&path).exists(),
                    path,
                    kind: LocationKind::from_db(&row.get::<_, String>(3)?),
                    source: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(locations)
    }

    /// Replaces the detected locations of a game. User-added locations and user rejections are kept:
    /// a path the user rejected is never re-added by detection.
    pub fn replace_detected(&self, game_id: i64, source: &str, detected: &[DerivedLocation]) -> CoreResult<()> {
        let mut connection = self.db.connection()?;
        let tx = connection.transaction()?;
        let now = now_rfc3339();
        tx.execute(
            "DELETE FROM save_locations WHERE game_id = ?1 AND source = ?2 AND status <> 'rejected'",
            params![game_id, source],
        )?;
        for location in detected {
            let path = location.path.to_string_lossy();
            let already: Option<i64> = tx
                .query_row(
                    "SELECT id FROM save_locations WHERE game_id = ?1 AND resolved_path = ?2 COLLATE NOCASE",
                    params![game_id, path],
                    |row| row.get(0),
                )
                .optional()?;
            if already.is_some() {
                continue;
            }
            tx.execute(
                "INSERT INTO save_locations (game_id, path_template, resolved_path, kind, source, confidence, status, confirmed_by_user, detected_at, last_verified_at)
                 VALUES (?1, ?2, ?2, ?3, ?4, 0.9, 'verified', 0, ?5, ?5)",
                params![game_id, path, location.kind.as_db(), source, now],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Adds a folder chosen by the user (already validated by the caller).
    pub fn add_user_folder(&self, game_id: i64, folder: &Path) -> CoreResult<()> {
        let path = folder.to_string_lossy();
        let connection = self.db.connection()?;
        let existing: Option<i64> = connection
            .query_row(
                "SELECT id FROM save_locations WHERE game_id = ?1 AND resolved_path = ?2 COLLATE NOCASE",
                params![game_id, path],
                |row| row.get(0),
            )
            .optional()?;
        match existing {
            Some(id) => {
                connection.execute(
                    "UPDATE save_locations SET status = 'verified', confirmed_by_user = 1 WHERE id = ?1",
                    [id],
                )?;
            }
            None => {
                connection.execute(
                    "INSERT INTO save_locations (game_id, path_template, resolved_path, kind, source, confidence, status, confirmed_by_user, detected_at, last_verified_at)
                     VALUES (?1, ?2, ?2, 'directory', 'user', 1.0, 'verified', 1, ?3, ?3)",
                    params![game_id, path, now_rfc3339()],
                )?;
            }
        }
        Ok(())
    }

    /// Removes a location from backups. Detected ones are remembered as rejected.
    pub fn reject(&self, game_id: i64, location_id: i64) -> CoreResult<()> {
        let connection = self.db.connection()?;
        let source: Option<String> = connection
            .query_row(
                "SELECT source FROM save_locations WHERE id = ?1 AND game_id = ?2",
                params![location_id, game_id],
                |row| row.get(0),
            )
            .optional()?;
        match source.as_deref() {
            None => Err(CoreError::NotFound(format!("save location {location_id}"))),
            Some("user") => {
                connection.execute("DELETE FROM save_locations WHERE id = ?1", [location_id])?;
                Ok(())
            }
            Some(_) => {
                connection.execute("UPDATE save_locations SET status = 'rejected' WHERE id = ?1", [location_id])?;
                Ok(())
            }
        }
    }

    /// True if `path` equals one of the game's active locations (used to authorise restores).
    pub fn is_registered(&self, game_id: i64, path: &Path) -> CoreResult<bool> {
        let target = normalize_path(path);
        Ok(self.list(game_id)?.iter().any(|location| normalize_path(&location.path_buf()) == target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::LibraryRepository;
    use crate::platforms::{DiscoveredGame, Discovery, Platform};

    fn one_game(db: &Database) -> CoreResult<i64> {
        let repo = LibraryRepository::new(db);
        repo.merge_discovery(
            Platform::Steam,
            &Discovery {
                games: vec![DiscoveredGame {
                    platform_game_id: "1".into(),
                    title: "Game".into(),
                    install_path: PathBuf::from(r"D:\Game"),
                    executable_path: None,
                    size_bytes: None,
                    last_played_at: None,
                    playtime_seconds: None,
                }],
                ..Discovery::default()
            },
        )?;
        Ok(repo.list()?[0].id)
    }

    #[test]
    fn detection_respects_user_choices() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let game_id = one_game(&db)?;
        let repo = SaveLocationRepository::new(&db);
        let detected = vec![
            DerivedLocation { path: PathBuf::from(r"C:\Saves\A"), kind: LocationKind::Directory },
            DerivedLocation { path: PathBuf::from(r"C:\Saves\B"), kind: LocationKind::Directory },
        ];
        repo.replace_detected(game_id, "ludusavi", &detected)?;
        let b = repo
            .list(game_id)?
            .into_iter()
            .find(|location| location.path.ends_with('B'))
            .ok_or(CoreError::NotFound("B".into()))?;
        repo.reject(game_id, b.id)?;
        repo.replace_detected(game_id, "ludusavi", &detected)?;
        let paths: Vec<String> = repo.list(game_id)?.into_iter().map(|location| location.path).collect();
        assert_eq!(paths, vec![r"C:\Saves\A".to_owned()], "a rejected location is not re-added");

        repo.add_user_folder(game_id, Path::new(r"C:\Mine"))?;
        repo.replace_detected(game_id, "ludusavi", &[])?;
        let remaining: Vec<String> = repo.list(game_id)?.into_iter().map(|location| location.source).collect();
        assert_eq!(remaining, vec!["user".to_owned()], "user folders survive re-detection");
        assert!(repo.is_registered(game_id, Path::new(r"c:\mine\"))?);
        Ok(())
    }
}
