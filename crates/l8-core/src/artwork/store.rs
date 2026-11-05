//! Disk cache and bookkeeping of artwork (`artwork_assets`, `artwork_lookups`).
//!
//! Files live in `<root>/<game id>/<kind>-<sha256 prefix>.<ext>` and are written through a
//! `.partial` file, so a crash never leaves a half-written image that looks valid. The cache is
//! re-creatable: clearing it only means images are fetched again.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use ts_rs::TS;

use crate::artwork::http::MAX_IMAGE_BYTES;
use crate::artwork::kind::ArtworkKind;
use crate::clock::{now_plus_seconds_rfc3339, now_rfc3339};
use crate::db::Database;
use crate::error::{CoreError, CoreResult};

pub const PROVIDER_STEAMGRIDDB: &str = "steamgriddb";
pub const PROVIDER_USER: &str = "user";

/// Where an active image of a game came from, for the Artwork tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArtworkSelection {
    pub kind: ArtworkKind,
    /// `steamgriddb` or `user`.
    pub provider: String,
    /// The user picked or uploaded it: automatic fetching never replaces it.
    pub user_selected: bool,
}

pub struct NewAsset<'a> {
    pub provider: &'a str,
    pub provider_asset_id: Option<&'a str>,
    pub source_url: Option<&'a str>,
    pub bytes: &'a [u8],
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub user_selected: bool,
}

/// Image formats the app stores, recognised by their first bytes (never by file name).
struct ImageType {
    mime: &'static str,
    extension: &'static str,
}

fn sniff_image(bytes: &[u8]) -> Option<ImageType> {
    let png = bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    let jpeg = bytes.starts_with(&[0xFF, 0xD8, 0xFF]);
    let webp = bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP";
    let ico = bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]);
    match (png, jpeg, webp, ico) {
        (true, ..) => Some(ImageType { mime: "image/png", extension: "png" }),
        (_, true, ..) => Some(ImageType { mime: "image/jpeg", extension: "jpg" }),
        (_, _, true, _) => Some(ImageType { mime: "image/webp", extension: "webp" }),
        (.., true) => Some(ImageType { mime: "image/vnd.microsoft.icon", extension: "ico" }),
        _ => None,
    }
}

pub struct ArtworkStore<'db> {
    db: &'db Database,
    root: PathBuf,
}

impl<'db> ArtworkStore<'db> {
    pub fn new(db: &'db Database, root: impl Into<PathBuf>) -> Self {
        Self { db, root: root.into() }
    }

    /// Validates, stores and activates an image, replacing the previous active one of that kind.
    pub fn save(&self, game_id: i64, kind: ArtworkKind, asset: &NewAsset<'_>) -> CoreResult<PathBuf> {
        if asset.bytes.is_empty() || asset.bytes.len() as u64 > MAX_IMAGE_BYTES {
            return Err(CoreError::Validation("the image is empty or larger than 15 MB".into()));
        }
        let image = sniff_image(asset.bytes)
            .ok_or_else(|| CoreError::Validation("the file is not a PNG, JPEG, WebP or ICO image".into()))?;
        let sha256 = hex::encode(Sha256::digest(asset.bytes));
        let folder = self.root.join(game_id.to_string());
        std::fs::create_dir_all(&folder)?;
        let path = folder.join(format!("{}-{}.{}", kind.as_db(), &sha256[..16], image.extension));
        if !path.exists() {
            let partial = path.with_extension("partial");
            std::fs::write(&partial, asset.bytes)?;
            std::fs::rename(&partial, &path)?;
        }

        let mut connection = self.db.connection()?;
        let tx = connection.transaction()?;
        let previous = active_paths(&tx, game_id, kind)?;
        tx.execute("DELETE FROM artwork_assets WHERE game_id = ?1 AND kind = ?2", params![game_id, kind.as_db()])?;
        tx.execute(
            "INSERT INTO artwork_assets (game_id, kind, provider, provider_asset_id, source_url, local_path,
                                         width, height, mime, sha256, is_active, is_user_selected, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, ?11, ?12)",
            params![
                game_id,
                kind.as_db(),
                asset.provider,
                asset.provider_asset_id,
                asset.source_url,
                path.to_string_lossy(),
                asset.width,
                asset.height,
                image.mime,
                sha256,
                asset.user_selected,
                now_rfc3339(),
            ],
        )?;
        tx.commit()?;
        drop(connection);
        remove_files(previous.iter().filter(|old| **old != path));
        Ok(path)
    }

    /// Removes the active image of a kind so the Steam CDN or generated art shows again.
    pub fn reset(&self, game_id: i64, kind: ArtworkKind) -> CoreResult<()> {
        let connection = self.db.connection()?;
        let previous = active_paths(&connection, game_id, kind)?;
        connection
            .execute("DELETE FROM artwork_assets WHERE game_id = ?1 AND kind = ?2", params![game_id, kind.as_db()])?;
        drop(connection);
        remove_files(previous.iter());
        Ok(())
    }

    pub fn selections(&self, game_id: i64) -> CoreResult<Vec<ArtworkSelection>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT kind, provider, is_user_selected FROM artwork_assets WHERE game_id = ?1 AND is_active = 1",
        )?;
        let rows = statement
            .query_map([game_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, bool>(2)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut selections = Vec::new();
        for (kind, provider, user_selected) in rows {
            selections.push(ArtworkSelection { kind: ArtworkKind::from_db(&kind)?, provider, user_selected });
        }
        selections.sort_by_key(|selection| selection.kind);
        Ok(selections)
    }

    /// Removes the rows of images whose file no longer exists, plus the "found" lookup that would stop them
    /// from being fetched again. Returns how many images were forgotten.
    pub fn prune_missing_files(&self) -> CoreResult<u32> {
        let connection = self.db.connection()?;
        let rows = {
            let mut statement =
                connection.prepare("SELECT id, game_id, kind, provider, local_path FROM artwork_assets")?;
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut forgotten = 0;
        for (id, game_id, kind, provider, path) in rows {
            if Path::new(&path).exists() {
                continue;
            }
            connection.execute("DELETE FROM artwork_assets WHERE id = ?1", [id])?;
            connection.execute(
                "DELETE FROM artwork_lookups WHERE game_id = ?1 AND provider = ?2 AND kind = ?3",
                params![game_id, provider, kind],
            )?;
            forgotten += 1;
        }
        Ok(forgotten)
    }

    /// Whether a lookup for this game, provider and kind may run now (never tried, or its retry
    /// date has passed). "Nothing found" is cached too, so the provider is not asked again daily.
    pub fn lookup_due(&self, game_id: i64, provider: &str, kind: ArtworkKind) -> CoreResult<bool> {
        let connection = self.db.connection()?;
        let next_retry: Option<Option<String>> = connection
            .query_row(
                "SELECT next_retry_at FROM artwork_lookups WHERE game_id = ?1 AND provider = ?2 AND kind = ?3",
                params![game_id, provider, kind.as_db()],
                |row| row.get(0),
            )
            .optional()?;
        Ok(match next_retry {
            None => true,
            Some(None) => false,
            Some(Some(at)) => at <= now_rfc3339(),
        })
    }

    /// `retry_in_seconds = None` means "found, no retry needed".
    pub fn record_lookup(
        &self,
        game_id: i64,
        provider: &str,
        kind: ArtworkKind,
        status: &str,
        retry_in_seconds: Option<i64>,
    ) -> CoreResult<()> {
        self.db.connection()?.execute(
            "INSERT INTO artwork_lookups (game_id, provider, kind, status, last_attempt_at, next_retry_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(game_id, provider, kind) DO UPDATE SET
               status = excluded.status, last_attempt_at = excluded.last_attempt_at,
               next_retry_at = excluded.next_retry_at",
            params![
                game_id,
                provider,
                kind.as_db(),
                status,
                now_rfc3339(),
                retry_in_seconds.map(now_plus_seconds_rfc3339)
            ],
        )?;
        Ok(())
    }

    /// Forgets cached "nothing found" answers of a game so a manual search is not blocked.
    pub fn clear_lookups(&self, game_id: i64) -> CoreResult<()> {
        self.db.connection()?.execute("DELETE FROM artwork_lookups WHERE game_id = ?1", [game_id])?;
        Ok(())
    }

    pub fn external_id(&self, game_id: i64, provider: &str) -> CoreResult<Option<String>> {
        Ok(self
            .db
            .connection()?
            .query_row(
                "SELECT external_id FROM game_external_ids WHERE game_id = ?1 AND provider = ?2",
                params![game_id, provider],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn set_external_id(&self, game_id: i64, provider: &str, external_id: &str) -> CoreResult<()> {
        self.db.connection()?.execute(
            "INSERT INTO game_external_ids (game_id, provider, external_id) VALUES (?1, ?2, ?3)
             ON CONFLICT(game_id, provider) DO UPDATE SET external_id = excluded.external_id",
            params![game_id, provider, external_id],
        )?;
        Ok(())
    }
}

/// Maps the path of an `l8art` request (`/<game id>/<kind>-<16 hex>.<ext>`) to a cached image and
/// its content type. Anything else (other shapes, `..`, other extensions) is refused, so the
/// protocol can only ever serve files this module wrote.
pub fn resolve_public_path(root: &Path, uri_path: &str) -> Option<(PathBuf, &'static str)> {
    let mut parts = uri_path.trim_start_matches('/').split('/');
    let (game, file) = (parts.next()?, parts.next()?);
    if parts.next().is_some() || game.is_empty() || !game.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (stem, extension) = file.rsplit_once('.')?;
    let (kind, hash) = stem.split_once('-')?;
    let content_type = match extension {
        "png" => "image/png",
        "jpg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        _ => return None,
    };
    let valid_hash = hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if ArtworkKind::from_db(kind).is_err() || !valid_hash {
        return None;
    }
    Some((root.join(game).join(file), content_type))
}

/// Local paths of the active images, per game and kind, for the library listing.
pub fn active_art_by_game(connection: &Connection) -> CoreResult<HashMap<i64, HashMap<ArtworkKind, String>>> {
    let mut statement =
        connection.prepare("SELECT game_id, kind, local_path FROM artwork_assets WHERE is_active = 1")?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut map: HashMap<i64, HashMap<ArtworkKind, String>> = HashMap::new();
    for (game_id, kind, path) in rows {
        // A file the user deleted by hand must not leave a broken image: fall back to the next source.
        if Path::new(&path).exists() {
            map.entry(game_id).or_default().insert(ArtworkKind::from_db(&kind)?, path);
        }
    }
    Ok(map)
}

fn active_paths(connection: &Connection, game_id: i64, kind: ArtworkKind) -> CoreResult<Vec<PathBuf>> {
    let mut statement = connection.prepare("SELECT local_path FROM artwork_assets WHERE game_id = ?1 AND kind = ?2")?;
    let paths = statement
        .query_map(params![game_id, kind.as_db()], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(paths.into_iter().map(PathBuf::from).collect())
}

/// Cache files are disposable: a failure to delete one is logged, not fatal.
fn remove_files<'a>(paths: impl Iterator<Item = &'a PathBuf>) {
    for path in paths {
        if let Err(error) = std::fs::remove_file(path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(path = %path.display(), %error, "could not delete a cached image");
        }
    }
}

#[cfg(test)]
pub(crate) mod test_images {
    /// Smallest byte strings that the sniffer accepts.
    pub const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];
    pub const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3];
}

#[cfg(test)]
mod tests {
    use super::test_images::{JPEG, PNG};
    use super::*;

    fn game(db: &Database) -> CoreResult<i64> {
        let connection = db.connection()?;
        connection.execute(
            "INSERT INTO games (platform_id, title, sort_title, slug, added_at, updated_at)
             VALUES ('standalone', 'Game', 'game', 'game', 'x', 'x')",
            [],
        )?;
        Ok(connection.last_insert_rowid())
    }

    fn asset(bytes: &[u8], user_selected: bool) -> NewAsset<'_> {
        NewAsset {
            provider: PROVIDER_USER,
            provider_asset_id: None,
            source_url: None,
            bytes,
            width: None,
            height: None,
            user_selected,
        }
    }

    #[test]
    fn images_deleted_from_the_cache_are_forgotten_so_they_are_fetched_again() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let id = game(&db)?;
        let dir = tempfile::tempdir()?;
        let store = ArtworkStore::new(&db, dir.path());
        let path = store.save(id, ArtworkKind::Cover, &asset(PNG, false))?;
        store.record_lookup(id, PROVIDER_USER, ArtworkKind::Cover, "found", None)?;
        assert_eq!(store.prune_missing_files()?, 0, "present files are kept");
        assert!(!store.lookup_due(id, PROVIDER_USER, ArtworkKind::Cover)?);

        std::fs::remove_file(&path)?;
        assert_eq!(store.prune_missing_files()?, 1);
        assert!(store.selections(id)?.is_empty());
        assert!(store.lookup_due(id, PROVIDER_USER, ArtworkKind::Cover)?, "the lookup is due again");
        Ok(())
    }

    #[test]
    fn the_public_path_resolver_only_accepts_cache_file_names() {
        let root = Path::new("C:\\cache");
        let ok = resolve_public_path(root, "/5/cover-36aebf0c3dee7a41.jpg");
        assert_eq!(ok.map(|(_, content_type)| content_type), Some("image/jpeg"));
        assert!(resolve_public_path(root, "/5/hero-4936e0510744ee58.png").is_some());
        for bad in [
            "/5/../6/cover-36aebf0c3dee7a41.jpg",
            "/..%2F/cover-36aebf0c3dee7a41.jpg",
            "/5/cover-36aebf0c3dee7a41.exe",
            "/5/cover-36AEBF0C3DEE7A41.jpg",
            "/5/cover-short.jpg",
            "/5/wallpaper-36aebf0c3dee7a41.jpg",
            "/x/cover-36aebf0c3dee7a41.jpg",
            "/5/6/cover-36aebf0c3dee7a41.jpg",
            "/cover-36aebf0c3dee7a41.jpg",
            "",
        ] {
            assert!(resolve_public_path(root, bad).is_none(), "{bad} must be refused");
        }
    }

    #[test]
    fn saves_and_replaces_the_active_image() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let id = game(&db)?;
        let dir = tempfile::tempdir()?;
        let store = ArtworkStore::new(&db, dir.path());

        let first = store.save(id, ArtworkKind::Cover, &asset(PNG, true))?;
        assert!(first.exists());
        let second = store.save(id, ArtworkKind::Cover, &asset(JPEG, false))?;
        assert!(second.exists());
        assert!(!first.exists(), "the replaced file is cleaned up");
        let connection = db.connection()?;
        let active = active_art_by_game(&connection)?;
        assert_eq!(active[&id][&ArtworkKind::Cover], second.to_string_lossy());
        Ok(())
    }

    #[test]
    fn saving_the_same_image_twice_keeps_the_file() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let id = game(&db)?;
        let dir = tempfile::tempdir()?;
        let store = ArtworkStore::new(&db, dir.path());
        let first = store.save(id, ArtworkKind::Hero, &asset(PNG, true))?;
        let again = store.save(id, ArtworkKind::Hero, &asset(PNG, true))?;
        assert_eq!(first, again);
        assert!(again.exists());
        Ok(())
    }

    #[test]
    fn rejects_non_images_and_oversized_files() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let id = game(&db)?;
        let dir = tempfile::tempdir()?;
        let store = ArtworkStore::new(&db, dir.path());
        assert!(matches!(
            store.save(id, ArtworkKind::Cover, &asset(b"MZ\x90\x00 not an image", true)),
            Err(CoreError::Validation(_))
        ));
        assert!(matches!(store.save(id, ArtworkKind::Cover, &asset(b"", true)), Err(CoreError::Validation(_))));
        assert!(
            !dir.path().join(id.to_string()).exists()
                || std::fs::read_dir(dir.path().join(id.to_string()))?.next().is_none()
        );
        Ok(())
    }

    #[test]
    fn reset_removes_the_row_and_the_file() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let id = game(&db)?;
        let dir = tempfile::tempdir()?;
        let store = ArtworkStore::new(&db, dir.path());
        let path = store.save(id, ArtworkKind::Logo, &asset(PNG, true))?;
        store.reset(id, ArtworkKind::Logo)?;
        assert!(!path.exists());
        assert!(store.selections(id)?.is_empty());
        Ok(())
    }

    #[test]
    fn a_missing_file_falls_back_instead_of_breaking_the_image() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let id = game(&db)?;
        let dir = tempfile::tempdir()?;
        let store = ArtworkStore::new(&db, dir.path());
        let path = store.save(id, ArtworkKind::Cover, &asset(PNG, true))?;
        std::fs::remove_file(path)?;
        let connection = db.connection()?;
        assert!(active_art_by_game(&connection)?.is_empty());
        Ok(())
    }

    #[test]
    fn lookups_are_cached_until_their_retry_date() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let id = game(&db)?;
        let dir = tempfile::tempdir()?;
        let store = ArtworkStore::new(&db, dir.path());
        assert!(store.lookup_due(id, PROVIDER_STEAMGRIDDB, ArtworkKind::Cover)?);
        store.record_lookup(id, PROVIDER_STEAMGRIDDB, ArtworkKind::Cover, "not_found", Some(3600))?;
        assert!(!store.lookup_due(id, PROVIDER_STEAMGRIDDB, ArtworkKind::Cover)?);
        store.record_lookup(id, PROVIDER_STEAMGRIDDB, ArtworkKind::Cover, "error", Some(-5))?;
        assert!(store.lookup_due(id, PROVIDER_STEAMGRIDDB, ArtworkKind::Cover)?);
        store.record_lookup(id, PROVIDER_STEAMGRIDDB, ArtworkKind::Cover, "found", None)?;
        assert!(!store.lookup_due(id, PROVIDER_STEAMGRIDDB, ArtworkKind::Cover)?);
        store.clear_lookups(id)?;
        assert!(store.lookup_due(id, PROVIDER_STEAMGRIDDB, ArtworkKind::Cover)?);
        Ok(())
    }
}
