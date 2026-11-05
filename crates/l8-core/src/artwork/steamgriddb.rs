//! SteamGridDB API v2 client (user's own free API key). Endpoints used:
//! `GET /games/steam/{appId}`, `GET /search/autocomplete/{term}` and
//! `GET /{grids|heroes|logos|icons}/game/{id}`. Responses are `{ success, data }` envelopes.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::artwork::http::{HttpFailure, HttpGet, percent_encode};
use crate::artwork::kind::ArtworkKind;
use crate::error::{CoreError, CoreResult};

const API: &str = "https://www.steamgriddb.com/api/v2";

/// How many candidates the picker shows per kind.
const MAX_CANDIDATES: usize = 24;

/// Image types the app can display (animated WebM is skipped).
const ACCEPTED_MIMES: [&str; 4] = ["image/png", "image/jpeg", "image/webp", "image/vnd.microsoft.icon"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SgdbGame {
    pub id: i64,
    pub name: String,
}

/// One image offered by the provider, shown in the Artwork tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArtworkCandidate {
    pub id: String,
    pub kind: ArtworkKind,
    /// Full-size image (downloaded when chosen).
    pub url: String,
    /// Small preview for the picker.
    pub thumb_url: String,
    #[ts(type = "number | null")]
    pub width: Option<i64>,
    #[ts(type = "number | null")]
    pub height: Option<i64>,
    pub style: Option<String>,
    pub author: Option<String>,
    pub mime: Option<String>,
}

#[derive(Deserialize)]
struct Envelope<T> {
    #[serde(default = "Vec::new")]
    errors: Vec<String>,
    success: bool,
    data: Option<T>,
}

#[derive(Deserialize)]
struct GameJson {
    id: i64,
    name: String,
}

#[derive(Deserialize)]
struct AssetJson {
    id: i64,
    url: String,
    #[serde(default)]
    thumb: Option<String>,
    width: Option<i64>,
    height: Option<i64>,
    style: Option<String>,
    mime: Option<String>,
    author: Option<AuthorJson>,
}

#[derive(Deserialize)]
struct AuthorJson {
    name: Option<String>,
}

pub struct SteamGridDb<'a> {
    http: &'a dyn HttpGet,
    key: &'a str,
}

impl<'a> SteamGridDb<'a> {
    pub fn new(http: &'a dyn HttpGet, key: &'a str) -> Self {
        Self { http, key }
    }

    fn fetch<T: for<'de> Deserialize<'de>>(&self, path_and_query: &str) -> Result<Option<T>, HttpFailure> {
        let body = self.http.get_text(&format!("{API}{path_and_query}"), Some(self.key))?;
        let envelope: Envelope<T> = serde_json::from_str(&body)
            .map_err(|error| HttpFailure::Other(format!("unexpected answer from SteamGridDB: {error}")))?;
        if !envelope.success {
            return Err(HttpFailure::Other(format!("SteamGridDB reported: {}", envelope.errors.join("; "))));
        }
        Ok(envelope.data)
    }

    /// The SteamGridDB entry of a Steam app; `None` when the app is unknown.
    pub fn game_by_steam_id(&self, app_id: &str) -> CoreResult<Option<SgdbGame>> {
        if app_id.is_empty() || !app_id.chars().all(|c| c.is_ascii_digit()) {
            return Err(CoreError::Validation("a Steam app id is digits only".into()));
        }
        match self.fetch::<GameJson>(&format!("/games/steam/{app_id}")) {
            Ok(game) => Ok(game.map(|game| SgdbGame { id: game.id, name: game.name })),
            Err(HttpFailure::NotFound) => Ok(None),
            Err(other) => Err(other.into()),
        }
    }

    /// Games matching a title, best match first.
    pub fn search(&self, title: &str) -> CoreResult<Vec<SgdbGame>> {
        let term = title.trim();
        if term.is_empty() {
            return Ok(Vec::new());
        }
        match self.fetch::<Vec<GameJson>>(&format!("/search/autocomplete/{}", percent_encode(term))) {
            Ok(games) => {
                Ok(games.unwrap_or_default().into_iter().map(|g| SgdbGame { id: g.id, name: g.name }).collect())
            }
            Err(HttpFailure::NotFound) => Ok(Vec::new()),
            Err(other) => Err(other.into()),
        }
    }

    /// Images of one kind for a game, best-scored first (the API sorts them).
    pub fn candidates(&self, game_id: i64, kind: ArtworkKind) -> CoreResult<Vec<ArtworkCandidate>> {
        let path = format!(
            "/{}/game/{game_id}?nsfw=false&humor=false{}",
            kind.steamgriddb_segment(),
            kind.steamgriddb_filters()
        );
        let assets = match self.fetch::<Vec<AssetJson>>(&path) {
            Ok(assets) => assets.unwrap_or_default(),
            Err(HttpFailure::NotFound) => Vec::new(),
            Err(other) => return Err(other.into()),
        };
        Ok(assets
            .into_iter()
            .filter(|asset| asset.mime.as_deref().is_none_or(|mime| ACCEPTED_MIMES.contains(&mime)))
            .take(MAX_CANDIDATES)
            .map(|asset| ArtworkCandidate {
                id: asset.id.to_string(),
                kind,
                thumb_url: asset.thumb.unwrap_or_else(|| asset.url.clone()),
                url: asset.url,
                width: asset.width,
                height: asset.height,
                style: asset.style,
                author: asset.author.and_then(|author| author.name),
                mime: asset.mime,
            })
            .collect())
    }
}

/// Picks the entry whose normalized name equals the title, if any; the automatic matcher refuses
/// to guess when nothing matches exactly.
pub fn exact_match<'a>(games: &'a [SgdbGame], title: &str) -> Option<&'a SgdbGame> {
    let wanted = normalize_title(title);
    games.iter().find(|game| normalize_title(&game.name) == wanted)
}

/// Lower-case alphanumerics only, so "Hades II", "hades ii" and "Hades: II" compare equal.
pub fn normalize_title(title: &str) -> String {
    title.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    /// Answers each request with the next queued body and records the URLs asked.
    struct FakeHttp {
        answers: RefCell<Vec<Result<String, HttpFailure>>>,
        urls: RefCell<Vec<String>>,
    }

    impl FakeHttp {
        fn new(answers: Vec<Result<String, HttpFailure>>) -> Self {
            Self { answers: RefCell::new(answers), urls: RefCell::new(Vec::new()) }
        }
    }

    impl HttpGet for FakeHttp {
        fn get_text(&self, url: &str, _bearer: Option<&str>) -> Result<String, HttpFailure> {
            self.urls.borrow_mut().push(url.to_owned());
            self.answers.borrow_mut().remove(0)
        }

        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>, HttpFailure> {
            Err(HttpFailure::NotFound)
        }
    }

    #[test]
    fn parses_a_steam_lookup() -> CoreResult<()> {
        let http = FakeHttp::new(vec![Ok(r#"{"success":true,"data":{"id":5254,"name":"Sekiro"}}"#.into())]);
        let game = SteamGridDb::new(&http, "k").game_by_steam_id("814380")?;
        assert_eq!(game, Some(SgdbGame { id: 5254, name: "Sekiro".into() }));
        assert_eq!(http.urls.borrow()[0], "https://www.steamgriddb.com/api/v2/games/steam/814380");
        Ok(())
    }

    #[test]
    fn unknown_steam_app_is_none_and_bad_ids_are_refused() -> CoreResult<()> {
        let http = FakeHttp::new(vec![Err(HttpFailure::NotFound)]);
        assert_eq!(SteamGridDb::new(&http, "k").game_by_steam_id("1")?, None);
        assert!(matches!(SteamGridDb::new(&http, "k").game_by_steam_id("../x"), Err(CoreError::Validation(_))));
        Ok(())
    }

    #[test]
    fn search_encodes_the_title() -> CoreResult<()> {
        let http = FakeHttp::new(vec![Ok(r#"{"success":true,"data":[{"id":1,"name":"Hades II"}]}"#.into())]);
        let games = SteamGridDb::new(&http, "k").search("Hades II/?")?;
        assert_eq!(games.len(), 1);
        assert_eq!(http.urls.borrow()[0], "https://www.steamgriddb.com/api/v2/search/autocomplete/Hades%20II%2F%3F");
        Ok(())
    }

    #[test]
    fn candidates_skip_unsupported_types_and_keep_metadata() -> CoreResult<()> {
        let body = r#"{"success":true,"data":[
            {"id":10,"url":"https://cdn2.steamgriddb.com/grid/a.png","thumb":"https://cdn2.steamgriddb.com/thumb/a.jpg",
             "width":600,"height":900,"style":"alternate","mime":"image/png","author":{"name":"Ann"}},
            {"id":11,"url":"https://cdn2.steamgriddb.com/grid/b.webm","thumb":"https://cdn2.steamgriddb.com/thumb/b.jpg",
             "width":600,"height":900,"style":"alternate","mime":"video/webm","author":{"name":"Bob"}}]}"#;
        let http = FakeHttp::new(vec![Ok(body.into())]);
        let found = SteamGridDb::new(&http, "k").candidates(7, ArtworkKind::Cover)?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "10");
        assert_eq!(found[0].author.as_deref(), Some("Ann"));
        assert!(http.urls.borrow()[0].contains("/grids/game/7?nsfw=false&humor=false&dimensions=600x900"));
        Ok(())
    }

    #[test]
    fn a_failed_envelope_is_an_error() {
        let http = FakeHttp::new(vec![Ok(r#"{"success":false,"errors":["Game not found"]}"#.into())]);
        assert!(matches!(SteamGridDb::new(&http, "k").search("x"), Err(CoreError::External(_))));
    }

    #[test]
    fn rejected_keys_surface_as_auth_errors() {
        let http = FakeHttp::new(vec![Err(HttpFailure::Unauthorized)]);
        assert!(matches!(SteamGridDb::new(&http, "bad").search("x"), Err(CoreError::Auth(_))));
    }

    #[test]
    fn matching_ignores_case_and_punctuation() {
        let games = vec![SgdbGame { id: 1, name: "Hades: II".into() }, SgdbGame { id: 2, name: "Hades".into() }];
        assert_eq!(exact_match(&games, "hades ii").map(|g| g.id), Some(1));
        assert_eq!(exact_match(&games, "Hades III"), None);
    }
}
