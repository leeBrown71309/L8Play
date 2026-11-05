//! Artwork use cases: key management, candidate search, applying a choice, importing the user's
//! own image, resetting, and the automatic fetch for games without remote art.
//!
//! Source priority per kind: the user's choice > an automatically fetched SteamGridDB image >
//! Steam's public CDN (Steam games) > generated placeholder art (drawn by the UI).

use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

use crate::artwork::http::{HttpGet, MAX_IMAGE_BYTES, is_allowed_url};
use crate::artwork::kind::ArtworkKind;
use crate::artwork::secrets::{STEAMGRIDDB_KEY, SecretStore};
use crate::artwork::steamgriddb::{ArtworkCandidate, SgdbGame, SteamGridDb, exact_match};
use crate::artwork::store::{ArtworkSelection, ArtworkStore, NewAsset, PROVIDER_STEAMGRIDDB, PROVIDER_USER};
use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::library::{GameSummary, LibraryRepository};
use crate::platforms::Platform;

const EXTERNAL_ID_PROVIDER: &str = PROVIDER_STEAMGRIDDB;
/// "Nothing found" is asked again after a week; transient errors after an hour.
const NOT_FOUND_RETRY_SECONDS: i64 = 7 * 24 * 3600;
const ERROR_RETRY_SECONDS: i64 = 3600;

/// What an automatic fetch did, for the settings page and logs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArtworkFetchReport {
    pub fetched: u32,
    pub not_found: u32,
    pub failed: u32,
}

pub struct ArtworkService<'a> {
    db: &'a Database,
    store: ArtworkStore<'a>,
    secrets: &'a dyn SecretStore,
    http: &'a dyn HttpGet,
}

impl<'a> ArtworkService<'a> {
    pub fn new(
        db: &'a Database,
        root: impl Into<std::path::PathBuf>,
        secrets: &'a dyn SecretStore,
        http: &'a dyn HttpGet,
    ) -> Self {
        Self { db, store: ArtworkStore::new(db, root), secrets, http }
    }

    pub fn has_key(&self) -> CoreResult<bool> {
        Ok(self.secrets.get(STEAMGRIDDB_KEY)?.is_some())
    }

    /// Stores the key in the Credential Manager. The value is never returned or logged.
    pub fn save_key(&self, key: &str) -> CoreResult<()> {
        let key = key.trim();
        if key.len() < 16 || key.len() > 128 || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(CoreError::Validation("a SteamGridDB API key is 16 to 128 letters and digits".into()));
        }
        self.secrets.set(STEAMGRIDDB_KEY, key)
    }

    pub fn clear_key(&self) -> CoreResult<()> {
        self.secrets.delete(STEAMGRIDDB_KEY)
    }

    /// Asks SteamGridDB a trivial question to prove the saved key works.
    pub fn verify_key(&self) -> CoreResult<()> {
        let key = self.key()?;
        SteamGridDb::new(self.http, &key).search("Portal").map(|_| ())
    }

    fn key(&self) -> CoreResult<String> {
        self.secrets.get(STEAMGRIDDB_KEY)?.ok_or_else(|| CoreError::Auth("no SteamGridDB API key is saved".into()))
    }

    fn game(&self, game_id: i64) -> CoreResult<GameSummary> {
        LibraryRepository::new(self.db).summary(game_id)?.ok_or_else(|| CoreError::NotFound(format!("game {game_id}")))
    }

    pub fn selections(&self, game_id: i64) -> CoreResult<Vec<ArtworkSelection>> {
        self.store.selections(game_id)
    }

    /// Images SteamGridDB offers for a game. `query` overrides the title when it is not recognised.
    pub fn find_candidates(
        &self,
        game_id: i64,
        kind: ArtworkKind,
        query: Option<&str>,
    ) -> CoreResult<Vec<ArtworkCandidate>> {
        let key = self.key()?;
        let client = SteamGridDb::new(self.http, &key);
        let game = self.game(game_id)?;
        let query = query.map(str::trim).filter(|text| !text.is_empty());
        let Some(matched) = self.match_game(&client, &game, query, false)? else { return Ok(Vec::new()) };
        self.store.set_external_id(game_id, EXTERNAL_ID_PROVIDER, &matched.id.to_string())?;
        client.candidates(matched.id, kind)
    }

    /// Downloads the chosen image and makes it the user's selection for that kind.
    pub fn apply_candidate(&self, game_id: i64, candidate: &ArtworkCandidate) -> CoreResult<()> {
        // The URL comes from the WebView: only the provider's own hosts are ever fetched.
        if !is_allowed_url(&candidate.url) {
            return Err(CoreError::Validation("the image address is not from an allowed artwork host".into()));
        }
        self.game(game_id)?;
        let bytes = self.http.get_bytes(&candidate.url).map_err(CoreError::from)?;
        self.store.save(
            game_id,
            candidate.kind,
            &NewAsset {
                provider: PROVIDER_STEAMGRIDDB,
                provider_asset_id: Some(&candidate.id),
                source_url: Some(&candidate.url),
                bytes: &bytes,
                width: candidate.width,
                height: candidate.height,
                user_selected: true,
            },
        )?;
        Ok(())
    }

    /// Uses an image file from the user's disk.
    pub fn import_file(&self, game_id: i64, kind: ArtworkKind, path: &Path) -> CoreResult<()> {
        self.game(game_id)?;
        let metadata = std::fs::metadata(path)?;
        if !metadata.is_file() || metadata.len() > MAX_IMAGE_BYTES {
            return Err(CoreError::Validation("choose an image file smaller than 15 MB".into()));
        }
        let bytes = std::fs::read(path)?;
        self.store.save(
            game_id,
            kind,
            &NewAsset {
                provider: PROVIDER_USER,
                provider_asset_id: None,
                source_url: None,
                bytes: &bytes,
                width: None,
                height: None,
                user_selected: true,
            },
        )?;
        Ok(())
    }

    /// Back to the default source (Steam CDN or generated art) and allows a fresh automatic lookup.
    pub fn reset(&self, game_id: i64, kind: ArtworkKind) -> CoreResult<()> {
        self.store.reset(game_id, kind)?;
        self.store.clear_lookups(game_id)
    }

    /// Fetches missing cover, hero and logo for every game that has no remote art of its own
    /// (Epic and standalone). Steam games keep Steam's CDN art unless the user picks another image.
    /// Does nothing without a saved key; stops at the first rejected key or rate limit.
    pub fn fetch_missing(&self) -> CoreResult<ArtworkFetchReport> {
        let mut report = ArtworkFetchReport::default();
        if !self.has_key()? {
            return Ok(report);
        }
        for game in LibraryRepository::new(self.db).list()? {
            if game.platform == Platform::Steam {
                continue;
            }
            let outcome = self.fetch_game(&game)?;
            report.fetched += outcome.fetched;
            report.not_found += outcome.not_found;
            report.failed += outcome.failed;
        }
        Ok(report)
    }

    pub fn fetch_missing_for_game(&self, game_id: i64) -> CoreResult<ArtworkFetchReport> {
        if !self.has_key()? {
            return Ok(ArtworkFetchReport::default());
        }
        self.fetch_game(&self.game(game_id)?)
    }

    fn fetch_game(&self, game: &GameSummary) -> CoreResult<ArtworkFetchReport> {
        let mut report = ArtworkFetchReport::default();
        let have: Vec<ArtworkKind> = self.store.selections(game.id)?.into_iter().map(|s| s.kind).collect();
        let wanted: Vec<ArtworkKind> = ArtworkKind::AUTOMATIC
            .into_iter()
            .filter(|kind| !have.contains(kind))
            .filter(|kind| self.store.lookup_due(game.id, PROVIDER_STEAMGRIDDB, *kind).unwrap_or(false))
            .collect();
        if wanted.is_empty() {
            return Ok(report);
        }

        let key = self.key()?;
        let client = SteamGridDb::new(self.http, &key);
        let matched = match self.match_game(&client, game, None, true) {
            Ok(Some(matched)) => matched,
            Ok(None) => {
                for kind in wanted {
                    self.store.record_lookup(
                        game.id,
                        PROVIDER_STEAMGRIDDB,
                        kind,
                        "not_found",
                        Some(NOT_FOUND_RETRY_SECONDS),
                    )?;
                    report.not_found += 1;
                }
                return Ok(report);
            }
            Err(error @ (CoreError::Auth(_) | CoreError::Busy(_))) => return Err(error),
            Err(error) => {
                tracing::warn!(game_id = game.id, %error, "artwork match failed");
                report.failed += 1;
                return Ok(report);
            }
        };

        for kind in wanted {
            match self.fetch_best(&client, game.id, matched.id, kind) {
                Ok(true) => {
                    self.store.record_lookup(game.id, PROVIDER_STEAMGRIDDB, kind, "found", None)?;
                    report.fetched += 1;
                }
                Ok(false) => {
                    self.store.record_lookup(
                        game.id,
                        PROVIDER_STEAMGRIDDB,
                        kind,
                        "not_found",
                        Some(NOT_FOUND_RETRY_SECONDS),
                    )?;
                    report.not_found += 1;
                }
                Err(error @ (CoreError::Auth(_) | CoreError::Busy(_))) => return Err(error),
                Err(error) => {
                    tracing::warn!(game_id = game.id, kind = kind.as_db(), %error, "artwork fetch failed");
                    self.store.record_lookup(
                        game.id,
                        PROVIDER_STEAMGRIDDB,
                        kind,
                        "error",
                        Some(ERROR_RETRY_SECONDS),
                    )?;
                    report.failed += 1;
                }
            }
        }
        Ok(report)
    }

    /// Stores the top-scored candidate; `false` when the provider has none.
    fn fetch_best(&self, client: &SteamGridDb<'_>, game_id: i64, sgdb_id: i64, kind: ArtworkKind) -> CoreResult<bool> {
        let Some(best) = client.candidates(sgdb_id, kind)?.into_iter().next() else { return Ok(false) };
        let bytes = self.http.get_bytes(&best.url).map_err(CoreError::from)?;
        self.store.save(
            game_id,
            kind,
            &NewAsset {
                provider: PROVIDER_STEAMGRIDDB,
                provider_asset_id: Some(&best.id),
                source_url: Some(&best.url),
                bytes: &bytes,
                width: best.width,
                height: best.height,
                user_selected: false,
            },
        )?;
        Ok(true)
    }

    /// Finds the SteamGridDB entry of a game: the remembered id, the Steam app id, or a title
    /// search. `strict` (automatic mode) accepts only an exact title match; manual searches take the
    /// first result because the user can see and judge the pictures.
    fn match_game(
        &self,
        client: &SteamGridDb<'_>,
        game: &GameSummary,
        query: Option<&str>,
        strict: bool,
    ) -> CoreResult<Option<SgdbGame>> {
        if query.is_none() {
            if let Some(id) = self.store.external_id(game.id, EXTERNAL_ID_PROVIDER)?.and_then(|id| id.parse().ok()) {
                return Ok(Some(SgdbGame { id, name: String::new() }));
            }
            if game.platform == Platform::Steam
                && let Some(app_id) = game.platform_game_id.as_deref()
                && let Some(found) = client.game_by_steam_id(app_id)?
            {
                return Ok(Some(found));
            }
        }
        let term = query.unwrap_or(&game.title);
        let results = client.search(term)?;
        let chosen = if strict { exact_match(&results, term).cloned() } else { results.into_iter().next() };
        Ok(chosen)
    }
}
