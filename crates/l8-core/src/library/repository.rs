//! SQLite access for games, installations, library roots and play sessions.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::artwork::active_art_by_game;
use crate::clock::{now_rfc3339, seconds_between};
use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::library::slug::{slugify, sort_title};
use crate::library::{ArtworkUrls, GameDetail, GameSummary, LaunchTarget, WatchedGame};
use crate::platforms::steam_owned::OwnedGame;
use crate::platforms::{Discovery, Platform};
use crate::saves::normalize_path;

/// Counts produced by merging one platform's discovery.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MergeCounts {
    pub added: u32,
    pub updated: u32,
    pub uninstalled: u32,
}

/// Summary columns, in the order [`summary_from_row`] reads them.
const SUMMARY_SELECT: &str = "
    SELECT g.id, g.title, g.platform_id, g.platform_game_id, g.is_favorite, g.last_played_at,
           g.playtime_seconds, i.size_bytes, i.install_path, COALESCE(i.is_installed, 0), g.is_archived
    FROM games g
    LEFT JOIN installations i ON i.id = (
        SELECT id FROM installations WHERE game_id = g.id ORDER BY is_installed DESC, detected_at DESC LIMIT 1
    )";

const MAX_TEXT: usize = 120;

/// Fields the user can correct on the Properties tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataPatch {
    pub title: String,
    pub developer: Option<String>,
    pub release_year: Option<i64>,
}

/// Result of adding a game by hand.
#[derive(Debug, Clone, PartialEq)]
pub struct AddedGame {
    pub game: GameSummary,
    /// `false` when the executable was already in the library.
    pub created: bool,
}

pub struct LibraryRepository<'db> {
    db: &'db Database,
}

impl<'db> LibraryRepository<'db> {
    pub fn new(db: &'db Database) -> Self {
        Self { db }
    }

    /// Upserts every discovered game of `platform`, then marks the platform's games that were not
    /// seen as uninstalled. Runs in one transaction: a failed merge changes nothing.
    pub fn merge_discovery(&self, platform: Platform, discovery: &Discovery) -> CoreResult<MergeCounts> {
        let mut connection = self.db.connection()?;
        let tx = connection.transaction()?;
        let now = now_rfc3339();
        let mut counts = MergeCounts::default();
        let mut seen = HashSet::new();

        for root in &discovery.library_paths {
            let kind = library_root_kind(platform);
            tx.execute(
                "INSERT INTO library_roots (platform_id, path, kind, added_by) VALUES (?1, ?2, ?3, 'detected')
                 ON CONFLICT(path) DO NOTHING",
                params![platform.as_db_id(), root.to_string_lossy(), kind],
            )?;
        }

        for game in &discovery.games {
            seen.insert(game.platform_game_id.clone());
            let existing: Option<(i64, bool)> = tx
                .query_row(
                    "SELECT id, user_edited FROM games WHERE platform_id = ?1 AND platform_game_id = ?2",
                    params![platform.as_db_id(), game.platform_game_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            // A game the user added by hand before its launcher was detected keeps its identity
            // (art, saves, snapshots): only its platform changes.
            let existing = match existing {
                Some(found) => Some(found),
                None => adopt_standalone(&tx, platform, game)?,
            };
            let game_id = match existing {
                Some((id, user_edited)) => {
                    if !user_edited {
                        tx.execute(
                            "UPDATE games SET title = ?2, sort_title = ?3, updated_at = ?4 WHERE id = ?1 AND title <> ?2",
                            params![id, game.title, sort_title(&game.title), now],
                        )?;
                    }
                    counts.updated += 1;
                    id
                }
                None => {
                    let slug = unique_slug(&tx, &game.title)?;
                    tx.execute(
                        "INSERT INTO games (platform_id, platform_game_id, title, sort_title, slug, added_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                        params![platform.as_db_id(), game.platform_game_id, game.title, sort_title(&game.title), slug, now],
                    )?;
                    counts.added += 1;
                    tx.last_insert_rowid()
                }
            };
            // Launcher-known activity only ever moves forward; L8 sessions may already be newer.
            tx.execute(
                "UPDATE games SET
                    last_played_at = CASE WHEN ?2 IS NOT NULL AND (last_played_at IS NULL OR ?2 > last_played_at) THEN ?2 ELSE last_played_at END,
                    playtime_seconds = MAX(playtime_seconds, COALESCE(?3, 0))
                 WHERE id = ?1",
                params![game_id, game.last_played_at, game.playtime_seconds],
            )?;
            upsert_installation(
                &tx,
                game_id,
                &game.install_path,
                game.executable_path.as_deref(),
                game.size_bytes,
                installation_source(platform),
                &now,
            )?;
        }

        counts.uninstalled = mark_unseen_uninstalled(&tx, platform, &seen)?;
        tx.commit()?;
        Ok(counts)
    }

    /// Adds the Steam games the account owns but has not installed (no installation row, so they list
    /// as "not installed"). Existing games are never removed or renamed; a game with no recorded
    /// playtime takes the account's. Returns how many games were created.
    pub fn merge_owned_steam(&self, owned: &[OwnedGame]) -> CoreResult<u32> {
        let mut connection = self.db.connection()?;
        let tx = connection.transaction()?;
        let now = now_rfc3339();
        let platform = Platform::Steam.as_db_id();
        let mut added = 0;
        for game in owned {
            let existing: Option<i64> = tx
                .query_row(
                    "SELECT id FROM games WHERE platform_id = ?1 AND platform_game_id = ?2",
                    params![platform, game.app_id],
                    |row| row.get(0),
                )
                .optional()?;
            let game_id = match existing {
                Some(id) => id,
                None => {
                    let slug = unique_slug(&tx, &game.title)?;
                    tx.execute(
                        "INSERT INTO games (platform_id, platform_game_id, title, sort_title, slug, added_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                        params![platform, game.app_id, game.title, sort_title(&game.title), slug, now],
                    )?;
                    added += 1;
                    tx.last_insert_rowid()
                }
            };
            if let Some(seconds) = game.playtime_seconds {
                tx.execute(
                    "UPDATE games SET playtime_seconds = ?2 WHERE id = ?1 AND playtime_seconds = 0",
                    params![game_id, seconds],
                )?;
            }
        }
        tx.commit()?;
        Ok(added)
    }

    /// Adds a game by its executable. Re-adding the same executable returns the existing game.
    pub fn add_standalone(&self, executable: &Path, title: Option<&str>) -> CoreResult<GameSummary> {
        Ok(self.add_standalone_in(None, executable, title)?.game)
    }

    /// Adds a game started by `executable`. `folder` is the game's own folder (used for save
    /// detection and to recognise the running game); when absent, the executable's folder is used.
    /// An executable that is already in the library returns the existing game (`created: false`).
    pub fn add_standalone_in(
        &self,
        folder: Option<&Path>,
        executable: &Path,
        title: Option<&str>,
    ) -> CoreResult<AddedGame> {
        let executable = validate_executable(executable)?;
        let install_path = match folder {
            Some(folder) => {
                let inside = normalize_path(&executable).starts_with(&format!("{}\\", normalize_path(folder)));
                if !folder.is_dir() || !inside {
                    return Err(CoreError::Validation("the executable must be inside the game folder".into()));
                }
                folder
            }
            None => executable.parent().ok_or_else(|| CoreError::Validation("the executable has no folder".into()))?,
        };
        let exe_text = executable.to_string_lossy().to_string();
        let existing: Option<i64> = self
            .db
            .connection()?
            .query_row(
                "SELECT game_id FROM installations WHERE executable_path = ?1 COLLATE NOCASE",
                [&exe_text],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(game_id) = existing {
            let game = self.summary(game_id)?.ok_or_else(|| CoreError::NotFound(format!("game {game_id}")))?;
            return Ok(AddedGame { game, created: false });
        }

        let title = title
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map_or_else(|| title_from_executable(&executable), str::to_owned);
        let game_id = {
            let mut connection = self.db.connection()?;
            let tx = connection.transaction()?;
            let now = now_rfc3339();
            let slug = unique_slug(&tx, &title)?;
            tx.execute(
                "INSERT INTO games (platform_id, platform_game_id, title, sort_title, slug, user_edited, added_at, updated_at)
                 VALUES ('standalone', NULL, ?1, ?2, ?3, 1, ?4, ?4)",
                params![title, sort_title(&title), slug, now],
            )?;
            let game_id = tx.last_insert_rowid();
            upsert_installation(&tx, game_id, install_path, Some(&executable), None, "manual", &now)?;
            tx.commit()?;
            game_id
        };
        let game = self.summary(game_id)?.ok_or_else(|| CoreError::NotFound(format!("game {game_id}")))?;
        Ok(AddedGame { game, created: true })
    }

    /// Finds the game called `title` (ignoring case), or creates a saves-only game for it.
    ///
    /// An existing game of that name, installed or not, is reused: the imported snapshots simply join
    /// its history. Only a new title creates an archived game (no installation, never rescanned).
    pub fn find_or_create_archived(&self, title: &str) -> CoreResult<AddedGame> {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > MAX_TEXT {
            return Err(CoreError::Validation(format!("the title must be 1 to {MAX_TEXT} characters")));
        }
        let existing: Option<i64> = self
            .db
            .connection()?
            .query_row("SELECT id FROM games WHERE title = ?1 COLLATE NOCASE ORDER BY is_archived, id LIMIT 1", [title], |row| {
                row.get(0)
            })
            .optional()?;
        if let Some(game_id) = existing {
            let game = self.summary(game_id)?.ok_or_else(|| CoreError::NotFound(format!("game {game_id}")))?;
            return Ok(AddedGame { game, created: false });
        }
        let game_id = {
            let mut connection = self.db.connection()?;
            let tx = connection.transaction()?;
            let now = now_rfc3339();
            let slug = unique_slug(&tx, title)?;
            tx.execute(
                "INSERT INTO games (platform_id, platform_game_id, title, sort_title, slug, user_edited, is_archived, added_at, updated_at)
                 VALUES ('standalone', NULL, ?1, ?2, ?3, 1, 1, ?4, ?4)",
                params![title, sort_title(title), slug, now],
            )?;
            let game_id = tx.last_insert_rowid();
            tx.commit()?;
            game_id
        };
        let game = self.summary(game_id)?.ok_or_else(|| CoreError::NotFound(format!("game {game_id}")))?;
        Ok(AddedGame { game, created: true })
    }

    /// Games of the library, other than saves-only ones, that carry the same title as the saves-only
    /// game `archived_id` (ignoring case, accents and spacing): the game was installed since the saves
    /// were imported, and its snapshots can be joined to it.
    pub fn link_candidates(&self, archived_id: i64) -> CoreResult<Vec<GameSummary>> {
        let Some(archived) = self.summary(archived_id)?.filter(|game| game.archived) else { return Ok(Vec::new()) };
        let wanted = crate::saves::importer::normalize_title(&archived.title);
        Ok(self
            .list()?
            .into_iter()
            .filter(|game| !game.archived && game.id != archived_id)
            .filter(|game| crate::saves::importer::normalize_title(&game.title) == wanted)
            .collect())
    }

    /// Joins a saves-only game to the real game it turned out to be: every snapshot (and what hangs off
    /// them) moves to `target_id`, missing artwork and ids are inherited, and the saves-only row is
    /// removed. Snapshot archives stay where they are, so nothing is copied or lost.
    pub fn merge_archived_into(&self, archived_id: i64, target_id: i64) -> CoreResult<GameSummary> {
        let mut connection = self.db.connection()?;
        let tx = connection.transaction()?;
        let is_archived: Option<bool> = tx
            .query_row("SELECT is_archived FROM games WHERE id = ?1", [archived_id], |row| row.get(0))
            .optional()?;
        if is_archived != Some(true) {
            return Err(CoreError::Validation("only a saves-only game can be joined to another game".into()));
        }
        let target_archived: Option<bool> = tx
            .query_row("SELECT is_archived FROM games WHERE id = ?1", [target_id], |row| row.get(0))
            .optional()?;
        match target_archived {
            None => return Err(CoreError::NotFound(format!("game {target_id}"))),
            Some(true) => return Err(CoreError::Validation("choose a game that is not saves-only".into())),
            Some(false) => {}
        }
        tx.execute("UPDATE snapshots SET game_id = ?2 WHERE game_id = ?1", params![archived_id, target_id])?;
        tx.execute("UPDATE sync_conflicts SET game_id = ?2 WHERE game_id = ?1", params![archived_id, target_id])?;
        tx.execute("UPDATE OR IGNORE save_locations SET game_id = ?2 WHERE game_id = ?1", params![archived_id, target_id])?;
        tx.execute(
            "INSERT OR IGNORE INTO game_external_ids (game_id, provider, external_id)
             SELECT ?2, provider, external_id FROM game_external_ids WHERE game_id = ?1",
            params![archived_id, target_id],
        )?;
        tx.execute(
            "UPDATE artwork_assets SET game_id = ?2 WHERE game_id = ?1
               AND kind NOT IN (SELECT kind FROM artwork_assets WHERE game_id = ?2 AND is_active = 1)",
            params![archived_id, target_id],
        )?;
        tx.execute(
            "UPDATE games SET is_favorite = MAX(is_favorite, (SELECT is_favorite FROM games WHERE id = ?1)),
                              updated_at = ?3 WHERE id = ?2",
            params![archived_id, target_id, now_rfc3339()],
        )?;
        // Everything that mattered has moved; what is left (sessions, lookups, spare artwork) goes with the row.
        tx.execute("DELETE FROM games WHERE id = ?1", [archived_id])?;
        tx.commit()?;
        tracing::info!(archived_id, target_id, "saves-only game joined to its installed game");
        drop(connection);
        self.summary(target_id)?.ok_or_else(|| CoreError::NotFound(format!("game {target_id}")))
    }

    pub fn list(&self) -> CoreResult<Vec<GameSummary>> {
        let connection = self.db.connection()?;
        let mut statement =
            connection.prepare(&format!("{SUMMARY_SELECT} WHERE g.is_hidden = 0 ORDER BY g.sort_title"))?;
        let mut games = statement.query_map([], summary_from_row)?.collect::<Result<Vec<_>, _>>()?;
        let local = active_art_by_game(&connection)?;
        for game in &mut games {
            if let Some(art) = local.get(&game.id) {
                game.artwork.overlay(art);
            }
        }
        Ok(games)
    }

    pub fn summary(&self, game_id: i64) -> CoreResult<Option<GameSummary>> {
        let connection = self.db.connection()?;
        let mut game = connection
            .query_row(&format!("{SUMMARY_SELECT} WHERE g.id = ?1"), [game_id], summary_from_row)
            .optional()?;
        if let Some(game) = &mut game
            && let Some(art) = active_art_by_game(&connection)?.get(&game_id)
        {
            game.artwork.overlay(art);
        }
        Ok(game)
    }

    pub fn detail(&self, game_id: i64) -> CoreResult<Option<GameDetail>> {
        let Some(game) = self.summary(game_id)? else { return Ok(None) };
        let connection = self.db.connection()?;
        let (added_at, developer, release_year, executable_path): (String, Option<String>, Option<i64>, Option<String>) =
            connection.query_row(
            "SELECT g.added_at, g.developer, g.release_year,
                    (SELECT executable_path FROM installations WHERE game_id = g.id ORDER BY is_installed DESC, detected_at DESC LIMIT 1)
             FROM games g WHERE g.id = ?1",
            [game_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        let session_count =
            connection
                .query_row("SELECT COUNT(*) FROM game_sessions WHERE game_id = ?1", [game_id], |row| row.get(0))?;
        Ok(Some(GameDetail { game, developer, release_year, executable_path, added_at, session_count }))
    }

    pub fn set_favorite(&self, game_id: i64, favorite: bool) -> CoreResult<GameSummary> {
        let changed = self.db.connection()?.execute(
            "UPDATE games SET is_favorite = ?2, updated_at = ?3 WHERE id = ?1",
            params![game_id, favorite, now_rfc3339()],
        )?;
        if changed == 0 {
            return Err(CoreError::NotFound(format!("game {game_id}")));
        }
        self.summary(game_id)?.ok_or_else(|| CoreError::NotFound(format!("game {game_id}")))
    }

    /// Saves the user's corrections. The slug (snapshot folder name) never changes, and
    /// `user_edited` stops later scans from overwriting the title.
    pub fn update_metadata(&self, game_id: i64, patch: &MetadataPatch) -> CoreResult<GameSummary> {
        let title = patch.title.trim();
        if title.is_empty() || title.chars().count() > MAX_TEXT {
            return Err(CoreError::Validation(format!("the title must be 1 to {MAX_TEXT} characters")));
        }
        let developer = patch.developer.as_deref().map(str::trim).filter(|text| !text.is_empty());
        if developer.is_some_and(|text| text.chars().count() > MAX_TEXT) {
            return Err(CoreError::Validation(format!("the developer must be at most {MAX_TEXT} characters")));
        }
        if patch.release_year.is_some_and(|year| !(1970..=2100).contains(&year)) {
            return Err(CoreError::Validation("the release year must be between 1970 and 2100".into()));
        }
        let changed = self.db.connection()?.execute(
            "UPDATE games SET title = ?2, sort_title = ?3, developer = ?4, release_year = ?5, user_edited = 1,
                              updated_at = ?6 WHERE id = ?1",
            params![game_id, title, sort_title(title), developer, patch.release_year, now_rfc3339()],
        )?;
        if changed == 0 {
            return Err(CoreError::NotFound(format!("game {game_id}")));
        }
        self.summary(game_id)?.ok_or_else(|| CoreError::NotFound(format!("game {game_id}")))
    }

    pub fn launch_target(&self, game_id: i64) -> CoreResult<LaunchTarget> {
        let connection = self.db.connection()?;
        connection
            .query_row(
                "SELECT g.platform_id, g.platform_game_id, COALESCE(i.is_installed, 0), i.executable_path, i.install_path
                 FROM games g
                 LEFT JOIN installations i ON i.id = (
                     SELECT id FROM installations WHERE game_id = g.id ORDER BY is_installed DESC, detected_at DESC LIMIT 1)
                 WHERE g.id = ?1",
                [game_id],
                |row| {
                    let platform: String = row.get(0)?;
                    Ok((platform, row.get::<_, Option<String>>(1)?, row.get::<_, bool>(2)?, row.get::<_, Option<String>>(3)?, row.get::<_, Option<String>>(4)?))
                },
            )
            .optional()?
            .map(|(platform, platform_game_id, installed, executable, install)| LaunchTarget {
                game_id,
                platform: Platform::from_db_id(&platform).unwrap_or(Platform::Standalone),
                platform_game_id,
                installed,
                executable_path: executable.map(PathBuf::from),
                install_path: install.map(PathBuf::from),
            })
            .ok_or_else(|| CoreError::NotFound(format!("game {game_id}")))
    }

    /// Installed games and their folders, for the process monitor.
    pub fn watched_games(&self) -> CoreResult<Vec<WatchedGame>> {
        let connection = self.db.connection()?;
        let mut statement =
            connection.prepare("SELECT game_id, install_path FROM installations WHERE is_installed = 1")?;
        let games = statement
            .query_map([], |row| {
                Ok(WatchedGame { game_id: row.get(0)?, install_path: PathBuf::from(row.get::<_, String>(1)?) })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(games)
    }

    pub fn start_session(&self, game_id: i64, launched_by_app: bool) -> CoreResult<i64> {
        let connection = self.db.connection()?;
        connection.execute(
            "INSERT INTO game_sessions (game_id, started_at, launched_by_app) VALUES (?1, ?2, ?3)",
            params![game_id, now_rfc3339(), launched_by_app],
        )?;
        Ok(connection.last_insert_rowid())
    }

    /// Closes a session and adds its duration to the game's playtime. Returns the duration.
    pub fn end_session(&self, session_id: i64) -> CoreResult<i64> {
        let mut connection = self.db.connection()?;
        let tx = connection.transaction()?;
        let now = now_rfc3339();
        let (game_id, started_at): (i64, String) = tx.query_row(
            "SELECT game_id, started_at FROM game_sessions WHERE id = ?1 AND ended_at IS NULL",
            [session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let duration = seconds_between(&started_at, &now);
        tx.execute("UPDATE game_sessions SET ended_at = ?2 WHERE id = ?1", params![session_id, now])?;
        tx.execute(
            "UPDATE games SET playtime_seconds = playtime_seconds + ?2, last_played_at = ?3, updated_at = ?3 WHERE id = ?1",
            params![game_id, duration, now],
        )?;
        tx.commit()?;
        Ok(duration)
    }

    /// Sessions left open by a crash or power loss are closed without adding playtime.
    pub fn close_dangling_sessions(&self) -> CoreResult<usize> {
        Ok(self
            .db
            .connection()?
            .execute("UPDATE game_sessions SET ended_at = started_at WHERE ended_at IS NULL", [])?)
    }
}

fn summary_from_row(row: &Row<'_>) -> rusqlite::Result<GameSummary> {
    let platform_id: String = row.get(2)?;
    let platform = Platform::from_db_id(&platform_id).unwrap_or(Platform::Standalone);
    let platform_game_id: Option<String> = row.get(3)?;
    Ok(GameSummary {
        id: row.get(0)?,
        title: row.get(1)?,
        artwork: ArtworkUrls::for_game(platform, platform_game_id.as_deref()),
        platform,
        platform_game_id,
        favorite: row.get(4)?,
        last_played_at: row.get(5)?,
        playtime_seconds: row.get(6)?,
        size_bytes: row.get(7)?,
        install_path: row.get(8)?,
        installed: row.get(9)?,
        archived: row.get(10)?,
    })
}

fn upsert_installation(
    tx: &Connection,
    game_id: i64,
    install_path: &Path,
    executable: Option<&Path>,
    size_bytes: Option<u64>,
    source: &str,
    now: &str,
) -> CoreResult<()> {
    let path = install_path.to_string_lossy();
    let size = size_bytes.and_then(|bytes| i64::try_from(bytes).ok());
    tx.execute(
        "INSERT INTO installations (game_id, install_path, executable_path, size_bytes, is_installed, source, detected_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6)
         ON CONFLICT(game_id, install_path) DO UPDATE SET
            executable_path = COALESCE(excluded.executable_path, executable_path),
            size_bytes = COALESCE(excluded.size_bytes, size_bytes),
            is_installed = 1, source = excluded.source, detected_at = excluded.detected_at",
        params![game_id, path, executable.map(|exe| exe.to_string_lossy().to_string()), size, source, now],
    )?;
    // A game moved to another library keeps one installed location.
    tx.execute(
        "UPDATE installations SET is_installed = 0 WHERE game_id = ?1 AND install_path <> ?2",
        params![game_id, path],
    )?;
    Ok(())
}

/// `library_roots.kind` for a platform's detected folders.
fn library_root_kind(platform: Platform) -> &'static str {
    match platform {
        Platform::Steam => "steam_library",
        Platform::Epic => "epic_manifests",
        Platform::Xbox | Platform::Gog | Platform::Standalone => "scan_folder",
    }
}

/// `installations.source` for a platform's detected installs.
fn installation_source(platform: Platform) -> &'static str {
    match platform {
        Platform::Steam => "steam_manifest",
        Platform::Epic => "epic_manifest",
        Platform::Xbox => "xbox_manifest",
        Platform::Gog => "gog_registry",
        Platform::Standalone => "manual",
    }
}

/// Finds a standalone game that is the same install as a discovered Xbox/GOG game (same folder, or
/// its executable inside that folder) and moves it to `platform`. Returns its `(id, user_edited)`.
fn adopt_standalone(
    tx: &Connection,
    platform: Platform,
    game: &crate::platforms::DiscoveredGame,
) -> CoreResult<Option<(i64, bool)>> {
    if !matches!(platform, Platform::Xbox | Platform::Gog) {
        return Ok(None);
    }
    let folder = normalize_path(&game.install_path);
    let inside = |path: &str| path == folder || path.starts_with(&format!("{folder}\\"));
    let executable = game.executable_path.as_deref().map(normalize_path);
    let mut statement = tx.prepare(
        "SELECT g.id, g.user_edited, i.install_path, i.executable_path
         FROM games g JOIN installations i ON i.game_id = g.id
         WHERE g.platform_id = 'standalone' ORDER BY g.id",
    )?;
    let rows: Vec<(i64, bool, String, Option<String>)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let found = rows.into_iter().find(|(_, _, install_path, exe)| {
        let exe = exe.as_deref().map(|text| normalize_path(Path::new(text)));
        inside(&normalize_path(Path::new(install_path)))
            || exe.as_deref().is_some_and(inside)
            || (executable.is_some() && exe == executable)
    });
    let Some((id, user_edited, _, _)) = found else { return Ok(None) };
    tx.execute(
        "UPDATE games SET platform_id = ?2, platform_game_id = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, platform.as_db_id(), game.platform_game_id, now_rfc3339()],
    )?;
    tracing::info!(game_id = id, platform = platform.as_db_id(), title = %game.title, "standalone game reclassified");
    Ok(Some((id, user_edited)))
}

fn mark_unseen_uninstalled(tx: &Connection, platform: Platform, seen: &HashSet<String>) -> CoreResult<u32> {
    let mut statement = tx.prepare(
        "SELECT g.id, g.platform_game_id FROM games g JOIN installations i ON i.game_id = g.id
         WHERE g.platform_id = ?1 AND i.is_installed = 1",
    )?;
    let installed: Vec<(i64, Option<String>)> =
        statement.query_map([platform.as_db_id()], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<Result<_, _>>()?;
    let mut count = 0;
    for (game_id, platform_game_id) in installed {
        if platform_game_id.is_some_and(|id| !seen.contains(&id)) {
            tx.execute("UPDATE installations SET is_installed = 0 WHERE game_id = ?1", [game_id])?;
            count += 1;
        }
    }
    Ok(count)
}

fn unique_slug(tx: &Connection, title: &str) -> CoreResult<String> {
    let base = slugify(title);
    let mut candidate = base.clone();
    let mut suffix = 2;
    while tx.query_row("SELECT 1 FROM games WHERE slug = ?1", [&candidate], |_| Ok(())).optional()?.is_some() {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    Ok(candidate)
}

/// Accepts only an existing `.exe` file; returns its canonical path.
fn validate_executable(path: &Path) -> CoreResult<PathBuf> {
    let is_exe = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("exe"));
    if !is_exe {
        return Err(CoreError::Validation("choose a Windows executable (.exe)".into()));
    }
    if !path.is_file() {
        return Err(CoreError::Validation(format!("{} does not exist", path.display())));
    }
    Ok(dunce_canonicalize(path))
}

/// Canonical path without the `\\?\` prefix Windows adds, so paths stay readable and comparable.
fn dunce_canonicalize(path: &Path) -> PathBuf {
    match std::fs::canonicalize(path) {
        Ok(canonical) => {
            let text = canonical.to_string_lossy();
            text.strip_prefix(r"\\?\").map_or(canonical.clone(), PathBuf::from)
        }
        Err(_) => path.to_path_buf(),
    }
}

/// `D:\Games\HollowKnight\hollow_knight.exe` → `"Hollow Knight"` (from the file name).
fn title_from_executable(executable: &Path) -> String {
    let stem = executable.file_stem().map(|stem| stem.to_string_lossy().to_string()).unwrap_or_default();
    let mut words = String::new();
    let mut previous_lower = false;
    for c in stem.chars() {
        if c == '_' || c == '-' || c == '.' {
            words.push(' ');
            previous_lower = false;
        } else {
            if c.is_uppercase() && previous_lower {
                words.push(' ');
            }
            words.push(c);
            previous_lower = c.is_lowercase();
        }
    }
    let title = words
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map(|first| first.to_uppercase().chain(chars).collect::<String>()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ");
    if title.is_empty() { "Game".to_owned() } else { title }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platforms::DiscoveredGame;

    fn steam_game(id: &str, title: &str) -> DiscoveredGame {
        DiscoveredGame {
            platform_game_id: id.into(),
            title: title.into(),
            install_path: PathBuf::from(format!(r"D:\SteamLibrary\steamapps\common\{title}")),
            executable_path: None,
            size_bytes: Some(1_000),
            last_played_at: None,
            playtime_seconds: None,
        }
    }

    fn discovery(games: Vec<DiscoveredGame>) -> Discovery {
        Discovery { games, library_paths: vec![PathBuf::from(r"D:\SteamLibrary")], warnings: vec![] }
    }

    #[test]
    fn merge_adds_updates_and_marks_uninstalled() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let repo = LibraryRepository::new(&db);
        let first = repo.merge_discovery(
            Platform::Steam,
            &discovery(vec![steam_game("814380", "Sekiro"), steam_game("582010", "MHW")]),
        )?;
        assert_eq!(first, MergeCounts { added: 2, updated: 0, uninstalled: 0 });

        let second = repo.merge_discovery(Platform::Steam, &discovery(vec![steam_game("814380", "Sekiro")]))?;
        assert_eq!(second, MergeCounts { added: 0, updated: 1, uninstalled: 1 });

        let games = repo.list()?;
        assert_eq!(games.len(), 2, "uninstalled games are kept");
        let mhw = games.iter().find(|game| game.title == "MHW").ok_or(CoreError::NotFound("MHW".into()))?;
        assert!(!mhw.installed);
        assert!(mhw.artwork.cover.is_some());
        Ok(())
    }

    fn owned(id: &str, title: &str, seconds: Option<i64>) -> OwnedGame {
        OwnedGame { app_id: id.into(), title: title.into(), playtime_seconds: seconds }
    }

    #[test]
    fn owned_games_are_listed_as_not_installed_and_never_disturb_installed_ones()
    -> Result<(), Box<dyn std::error::Error>> {
        let db = Database::open_in_memory()?;
        let repo = LibraryRepository::new(&db);
        repo.merge_discovery(Platform::Steam, &discovery(vec![steam_game("814380", "Sekiro")]))?;
        repo.update_metadata(
            repo.list()?[0].id,
            &MetadataPatch { title: "My Sekiro".into(), developer: None, release_year: None },
        )?;

        let added = repo.merge_owned_steam(&[
            owned("814380", "Sekiro: Shadows Die Twice", Some(100)),
            owned("1245620", "ELDEN RING", Some(7200)),
            owned("1245620", "ELDEN RING", Some(7200)),
            owned("367520", "Hollow Knight", None),
        ])?;
        assert_eq!(added, 2, "the installed game and the duplicate are not added again");

        let games = repo.list()?;
        assert_eq!(games.len(), 3);
        let sekiro = games.iter().find(|game| game.platform_game_id.as_deref() == Some("814380")).ok_or("sekiro")?;
        assert!(sekiro.installed, "an installed game stays installed");
        assert_eq!(sekiro.title, "My Sekiro", "titles are never overwritten");
        let elden = games.iter().find(|game| game.platform_game_id.as_deref() == Some("1245620")).ok_or("elden")?;
        assert!(!elden.installed);
        assert_eq!((elden.platform, elden.playtime_seconds), (Platform::Steam, 7200));
        assert!(elden.install_path.is_none());

        // A later pass adds nothing, keeps existing playtime, and never deletes a game that left the list.
        let again = repo.merge_owned_steam(&[owned("1245620", "ELDEN RING", Some(99_999))])?;
        assert_eq!(again, 0);
        assert_eq!(repo.summary(elden.id)?.ok_or("kept")?.playtime_seconds, 7200);
        assert_eq!(repo.list()?.len(), 3);
        Ok(())
    }

    #[test]
    fn an_owned_game_flips_to_installed_through_the_normal_scan() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let repo = LibraryRepository::new(&db);
        repo.merge_owned_steam(&[owned("1245620", "ELDEN RING", None)])?;
        let counts = repo.merge_discovery(Platform::Steam, &discovery(vec![steam_game("1245620", "ELDEN RING")]))?;
        assert_eq!((counts.added, counts.updated), (0, 1), "same game, not a duplicate");
        let games = repo.list()?;
        assert_eq!(games.len(), 1);
        assert!(games[0].installed);
        // Uninstalling it later marks it not installed again.
        let gone = repo.merge_discovery(Platform::Steam, &discovery(Vec::new()))?;
        assert_eq!(gone.uninstalled, 1);
        assert!(!repo.list()?[0].installed);
        Ok(())
    }

    #[test]
    fn launcher_activity_only_moves_forward() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let repo = LibraryRepository::new(&db);
        let mut game = steam_game("814380", "Sekiro");
        game.last_played_at = Some("2026-11-04T21:12:00Z".into());
        game.playtime_seconds = Some(3_600);
        repo.merge_discovery(Platform::Steam, &discovery(vec![game.clone()]))?;
        game.last_played_at = Some("2026-10-01T10:00:00Z".into());
        game.playtime_seconds = Some(60);
        repo.merge_discovery(Platform::Steam, &discovery(vec![game]))?;
        let summary = &repo.list()?[0];
        assert_eq!(summary.last_played_at.as_deref(), Some("2026-11-04T21:12:00Z"));
        assert_eq!(summary.playtime_seconds, 3_600);
        Ok(())
    }

    #[test]
    fn same_titles_get_distinct_slugs() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let repo = LibraryRepository::new(&db);
        repo.merge_discovery(Platform::Steam, &discovery(vec![steam_game("1", "Doom"), steam_game("2", "Doom")]))?;
        let slugs: Vec<String> = db
            .connection()?
            .prepare("SELECT slug FROM games ORDER BY id")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        assert_eq!(slugs, vec!["doom", "doom-2"]);
        Ok(())
    }

    #[test]
    fn standalone_games_are_validated_and_deduplicated() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let exe = dir.path().join("hollow_knight.exe");
        std::fs::write(&exe, b"MZ")?;
        let db = Database::open_in_memory()?;
        let repo = LibraryRepository::new(&db);

        let added = repo.add_standalone(&exe, None)?;
        assert_eq!(added.title, "Hollow Knight");
        assert_eq!(added.platform, Platform::Standalone);
        assert!(added.installed);
        assert_eq!(repo.add_standalone(&exe, Some("Other name"))?.id, added.id, "same executable, same game");

        assert!(matches!(repo.add_standalone(&dir.path().join("readme.txt"), None), Err(CoreError::Validation(_))));
        assert!(matches!(repo.add_standalone(&dir.path().join("missing.exe"), None), Err(CoreError::Validation(_))));
        Ok(())
    }

    fn launcher_game(id: &str, title: &str, folder: &Path, exe: &Path) -> DiscoveredGame {
        DiscoveredGame {
            platform_game_id: id.into(),
            title: title.into(),
            install_path: folder.to_path_buf(),
            executable_path: Some(exe.to_path_buf()),
            size_bytes: None,
            last_played_at: None,
            playtime_seconds: None,
        }
    }

    /// The Xbox app / GOG scan must turn a hand-added game into a game of that platform, not a copy.
    #[test]
    fn a_hand_added_game_is_reclassified_by_its_launcher() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let game = root.path().join("Minecraft Dungeons II");
        let sibling = root.path().join("Minecraft Dungeons II Beta");
        for folder in [&game, &sibling] {
            std::fs::create_dir_all(folder.join("Content"))?;
            std::fs::write(folder.join("Content").join("Dungeons.exe"), b"MZ")?;
        }
        let exe = game.join("Content").join("Dungeons.exe");
        let db = Database::open_in_memory()?;
        let repo = LibraryRepository::new(&db);
        let added = repo.add_standalone_in(Some(&game), &exe, Some("My Dungeons"))?.game;
        let other = repo.add_standalone(&sibling.join("Content").join("Dungeons.exe"), Some("Beta"))?;

        let found = Discovery {
            games: vec![launcher_game("Pkg_x!App", "Minecraft Dungeons II", &game, &exe)],
            ..Discovery::default()
        };
        let counts = repo.merge_discovery(Platform::Xbox, &found)?;
        assert_eq!((counts.added, counts.updated), (0, 1), "no duplicate");
        let games = repo.list()?;
        assert_eq!(games.len(), 2);
        let moved = games.iter().find(|g| g.id == added.id).ok_or("the game is still there")?;
        assert_eq!(moved.platform, Platform::Xbox);
        assert_eq!(moved.platform_game_id.as_deref(), Some("Pkg_x!App"));
        assert_eq!(moved.title, "My Dungeons", "a title the user chose is kept");
        let untouched = games.iter().find(|g| g.id == other.id).ok_or("the sibling is still there")?;
        assert_eq!(
            untouched.platform,
            Platform::Standalone,
            "a folder that only shares a name prefix is not the same game"
        );

        let again = repo.merge_discovery(Platform::Xbox, &found)?;
        assert_eq!((again.added, again.updated), (0, 1), "a second scan changes nothing");
        // The package disappears from the Xbox app: the game is kept, marked not installed.
        let gone = repo.merge_discovery(Platform::Xbox, &Discovery::default())?;
        assert_eq!(gone.uninstalled, 1);
        assert!(!repo.summary(added.id)?.ok_or("kept")?.installed);
        Ok(())
    }

    #[test]
    fn gog_games_are_found_by_their_executable_too() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let folder = root.path().join("Moonlighter");
        std::fs::create_dir_all(&folder)?;
        let exe = folder.join("Moonlighter.exe");
        std::fs::write(&exe, b"MZ")?;
        let db = Database::open_in_memory()?;
        let repo = LibraryRepository::new(&db);
        let added = repo.add_standalone_in(Some(&folder), &exe, None)?.game;
        // GOG reports a different folder text for the same install (an ancestor-free, executable-only match).
        let found = Discovery {
            games: vec![launcher_game("2147483047", "Moonlighter", &folder, &exe)],
            ..Discovery::default()
        };
        repo.merge_discovery(Platform::Gog, &found)?;
        let games = repo.list()?;
        assert_eq!(games.len(), 1);
        assert_eq!((games[0].id, games[0].platform), (added.id, Platform::Gog));
        Ok(())
    }

    #[test]
    fn sessions_add_playtime() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let repo = LibraryRepository::new(&db);
        repo.merge_discovery(Platform::Steam, &discovery(vec![steam_game("814380", "Sekiro")]))?;
        let game_id = repo.list()?[0].id;
        db.connection()?.execute(
            "INSERT INTO game_sessions (game_id, started_at, launched_by_app) VALUES (?1, '2000-01-01T00:00:00Z', 1)",
            [game_id],
        )?;
        let session_id = db.connection()?.last_insert_rowid();
        assert!(repo.end_session(session_id)? > 0);
        let game = repo.summary(game_id)?.ok_or(CoreError::NotFound("game".into()))?;
        assert!(game.playtime_seconds > 0);
        assert!(game.last_played_at.is_some());
        Ok(())
    }

    #[test]
    fn favorites_and_missing_games() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let repo = LibraryRepository::new(&db);
        repo.merge_discovery(Platform::Steam, &discovery(vec![steam_game("814380", "Sekiro")]))?;
        let game_id = repo.list()?[0].id;
        assert!(repo.set_favorite(game_id, true)?.favorite);
        assert!(matches!(repo.set_favorite(9_999, true), Err(CoreError::NotFound(_))));
        Ok(())
    }

    #[test]
    fn executable_names_become_titles() {
        assert_eq!(title_from_executable(Path::new(r"C:\g\hollow_knight.exe")), "Hollow Knight");
        assert_eq!(title_from_executable(Path::new(r"C:\g\StardewValley.exe")), "Stardew Valley");
        assert_eq!(title_from_executable(Path::new(r"C:\g\celeste.exe")), "Celeste");
    }
}
