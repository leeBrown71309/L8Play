//! Service-level tests with a fake HTTP layer and an in-memory secret store.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use crate::artwork::http::{HttpFailure, HttpGet};
use crate::artwork::kind::ArtworkKind;
use crate::artwork::secrets::{MemorySecretStore, STEAMGRIDDB_KEY, SecretStore};
use crate::artwork::service::ArtworkService;
use crate::artwork::steamgriddb::ArtworkCandidate;
use crate::artwork::store::test_images::PNG;
use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::library::{LibraryRepository, MetadataPatch};

const KEY: &str = "0123456789abcdef0123456789abcdef";

/// Routes by URL fragment and counts calls, so tests can assert what was (not) requested.
#[derive(Default)]
struct FakeHttp {
    texts: HashMap<&'static str, Result<String, HttpFailure>>,
    images: HashMap<&'static str, Vec<u8>>,
    calls: RefCell<Vec<String>>,
}

impl HttpGet for FakeHttp {
    fn get_text(&self, url: &str, bearer: Option<&str>) -> Result<String, HttpFailure> {
        assert_eq!(bearer, Some(KEY), "every API call carries the saved key");
        self.calls.borrow_mut().push(url.to_owned());
        self.texts
            .iter()
            .find(|(fragment, _)| url.contains(*fragment))
            .map_or(Err(HttpFailure::NotFound), |(_, answer)| answer.clone())
    }

    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, HttpFailure> {
        self.calls.borrow_mut().push(url.to_owned());
        self.images.get(url).cloned().ok_or(HttpFailure::NotFound)
    }
}

fn standalone_game(db: &Database, title: &str) -> CoreResult<i64> {
    let connection = db.connection()?;
    connection.execute(
        "INSERT INTO games (platform_id, title, sort_title, slug, added_at, updated_at)
         VALUES ('standalone', ?1, lower(?1), lower(?1), 'x', 'x')",
        [title],
    )?;
    Ok(connection.last_insert_rowid())
}

fn assets_body(url: &str) -> String {
    format!(
        r#"{{"success":true,"data":[{{"id":42,"url":"{url}","thumb":"{url}","width":600,"height":900,
        "style":"alternate","mime":"image/png","author":{{"name":"Ann"}}}}]}}"#
    )
}

const SEARCH_HIT: &str = r#"{"success":true,"data":[{"id":7,"name":"Hollow Knight"}]}"#;
const COVER_URL: &str = "https://cdn2.steamgriddb.com/grid/cover.png";

fn store_with_key() -> CoreResult<MemorySecretStore> {
    let secrets = MemorySecretStore::default();
    secrets.set(STEAMGRIDDB_KEY, KEY)?;
    Ok(secrets)
}

#[test]
fn key_validation_and_storage() -> CoreResult<()> {
    let db = Database::open_in_memory()?;
    let dir = tempfile::tempdir()?;
    let secrets = MemorySecretStore::default();
    let http = FakeHttp::default();
    let service = ArtworkService::new(&db, dir.path(), &secrets, &http);

    assert!(!service.has_key()?);
    assert!(matches!(service.save_key("short"), Err(CoreError::Validation(_))));
    assert!(matches!(service.save_key("0123456789abcdef 0123456789abcdef"), Err(CoreError::Validation(_))));
    service.save_key(&format!("  {KEY}  "))?;
    assert!(service.has_key()?);
    assert_eq!(secrets.get(STEAMGRIDDB_KEY)?.as_deref(), Some(KEY));
    service.clear_key()?;
    assert!(!service.has_key()?);
    assert!(matches!(service.find_candidates(1, ArtworkKind::Cover, None), Err(CoreError::Auth(_))));
    Ok(())
}

#[test]
fn finds_candidates_and_applies_one_as_the_users_choice() -> CoreResult<()> {
    let db = Database::open_in_memory()?;
    let id = standalone_game(&db, "Hollow Knight")?;
    let dir = tempfile::tempdir()?;
    let secrets = store_with_key()?;
    let mut http = FakeHttp::default();
    http.texts.insert("/search/autocomplete/", Ok(SEARCH_HIT.into()));
    http.texts.insert("/grids/game/7", Ok(assets_body(COVER_URL)));
    http.images.insert(COVER_URL, PNG.to_vec());
    let service = ArtworkService::new(&db, dir.path(), &secrets, &http);

    let found = service.find_candidates(id, ArtworkKind::Cover, None)?;
    assert_eq!(found.len(), 1);
    service.apply_candidate(id, &found[0])?;

    let selections = service.selections(id)?;
    assert_eq!(selections.len(), 1);
    assert!(selections[0].user_selected);
    let game = LibraryRepository::new(&db).summary(id)?.ok_or(CoreError::NotFound("game".into()))?;
    let cover = game.artwork.cover.ok_or(CoreError::NotFound("cover".into()))?;
    assert!(Path::new(&cover).exists(), "the library lists the cached file");
    Ok(())
}

#[test]
fn refuses_candidate_urls_outside_the_artwork_hosts() -> CoreResult<()> {
    let db = Database::open_in_memory()?;
    let id = standalone_game(&db, "Game")?;
    let dir = tempfile::tempdir()?;
    let secrets = store_with_key()?;
    let http = FakeHttp::default();
    let service = ArtworkService::new(&db, dir.path(), &secrets, &http);
    let candidate = ArtworkCandidate {
        id: "1".into(),
        kind: ArtworkKind::Cover,
        url: "https://evil.example/a.png".into(),
        thumb_url: "https://evil.example/a.png".into(),
        width: None,
        height: None,
        style: None,
        author: None,
        mime: None,
    };
    assert!(matches!(service.apply_candidate(id, &candidate), Err(CoreError::Validation(_))));
    assert!(http.calls.borrow().is_empty(), "nothing was requested");
    Ok(())
}

#[test]
fn imports_the_users_own_image_and_rejects_other_files() -> CoreResult<()> {
    let db = Database::open_in_memory()?;
    let id = standalone_game(&db, "Game")?;
    let dir = tempfile::tempdir()?;
    let secrets = MemorySecretStore::default();
    let http = FakeHttp::default();
    let service = ArtworkService::new(&db, dir.path().join("cache"), &secrets, &http);

    let image = dir.path().join("cover.png");
    std::fs::write(&image, PNG)?;
    service.import_file(id, ArtworkKind::Cover, &image)?;
    assert_eq!(service.selections(id)?[0].provider, "user");

    let text = dir.path().join("notes.png");
    std::fs::write(&text, b"just text pretending to be a png")?;
    assert!(matches!(service.import_file(id, ArtworkKind::Cover, &text), Err(CoreError::Validation(_))));
    assert!(matches!(service.import_file(id, ArtworkKind::Cover, dir.path()), Err(CoreError::Validation(_))));

    service.reset(id, ArtworkKind::Cover)?;
    assert!(service.selections(id)?.is_empty());
    Ok(())
}

#[test]
fn automatic_fetch_fills_gaps_and_remembers_misses() -> CoreResult<()> {
    let db = Database::open_in_memory()?;
    let id = standalone_game(&db, "Hollow Knight")?;
    let dir = tempfile::tempdir()?;
    let secrets = store_with_key()?;
    let mut http = FakeHttp::default();
    http.texts.insert("/search/autocomplete/", Ok(SEARCH_HIT.into()));
    http.texts.insert("/grids/game/7", Ok(assets_body(COVER_URL)));
    http.images.insert(COVER_URL, PNG.to_vec());
    let service = ArtworkService::new(&db, dir.path(), &secrets, &http);

    let first = service.fetch_missing()?;
    assert_eq!((first.fetched, first.not_found, first.failed), (1, 2, 0), "cover found; hero and logo missing");
    let selections = service.selections(id)?;
    assert_eq!(selections.len(), 1);
    assert!(!selections[0].user_selected, "automatic images never count as the user's choice");

    let calls_after_first = http.calls.borrow().len();
    let second = service.fetch_missing()?;
    assert_eq!((second.fetched, second.not_found, second.failed), (0, 0, 0));
    assert_eq!(http.calls.borrow().len(), calls_after_first, "cached answers prevent new requests");
    Ok(())
}

#[test]
fn automatic_fetch_refuses_to_guess_a_game() -> CoreResult<()> {
    let db = Database::open_in_memory()?;
    standalone_game(&db, "Hollow Knight Silksong Demo")?;
    let dir = tempfile::tempdir()?;
    let secrets = store_with_key()?;
    let mut http = FakeHttp::default();
    http.texts.insert("/search/autocomplete/", Ok(SEARCH_HIT.into()));
    let service = ArtworkService::new(&db, dir.path(), &secrets, &http);
    let report = service.fetch_missing()?;
    assert_eq!((report.fetched, report.not_found), (0, 3), "no exact title match, nothing downloaded");
    assert!(http.calls.borrow().iter().all(|url| url.contains("/search/autocomplete/")));
    Ok(())
}

#[test]
fn automatic_fetch_never_replaces_the_users_choice_and_does_nothing_without_a_key() -> CoreResult<()> {
    let db = Database::open_in_memory()?;
    let id = standalone_game(&db, "Hollow Knight")?;
    let dir = tempfile::tempdir()?;
    let image = dir.path().join("mine.png");
    std::fs::write(&image, PNG)?;
    let none = MemorySecretStore::default();
    let http = FakeHttp::default();
    ArtworkService::new(&db, dir.path().join("cache"), &none, &http).import_file(id, ArtworkKind::Cover, &image)?;
    assert_eq!(ArtworkService::new(&db, dir.path().join("cache"), &none, &http).fetch_missing()?.fetched, 0);
    assert!(http.calls.borrow().is_empty());

    let secrets = store_with_key()?;
    let mut http = FakeHttp::default();
    http.texts.insert("/search/autocomplete/", Ok(SEARCH_HIT.into()));
    let service = ArtworkService::new(&db, dir.path().join("cache"), &secrets, &http);
    service.fetch_missing()?;
    assert!(http.calls.borrow().iter().all(|url| !url.contains("/grids/")), "the user's cover is never searched again");
    Ok(())
}

#[test]
fn a_rejected_key_stops_the_automatic_fetch() -> CoreResult<()> {
    let db = Database::open_in_memory()?;
    standalone_game(&db, "Hollow Knight")?;
    let dir = tempfile::tempdir()?;
    let secrets = store_with_key()?;
    let mut http = FakeHttp::default();
    http.texts.insert("/search/autocomplete/", Err(HttpFailure::Unauthorized));
    let service = ArtworkService::new(&db, dir.path(), &secrets, &http);
    assert!(matches!(service.fetch_missing(), Err(CoreError::Auth(_))));
    Ok(())
}

#[test]
fn metadata_edits_validate_and_survive_scans() -> CoreResult<()> {
    let db = Database::open_in_memory()?;
    let id = standalone_game(&db, "Tlou I")?;
    let repository = LibraryRepository::new(&db);
    let updated = repository.update_metadata(
        id,
        &MetadataPatch {
            title: "  The Last of Us Part I ".into(),
            developer: Some("Naughty Dog".into()),
            release_year: Some(2022),
        },
    )?;
    assert_eq!(updated.title, "The Last of Us Part I");
    let detail = repository.detail(id)?.ok_or(CoreError::NotFound("detail".into()))?;
    assert_eq!(detail.developer.as_deref(), Some("Naughty Dog"));
    assert_eq!(detail.release_year, Some(2022));

    for bad in [
        MetadataPatch { title: "   ".into(), developer: None, release_year: None },
        MetadataPatch { title: "x".repeat(121), developer: None, release_year: None },
        MetadataPatch { title: "ok".into(), developer: None, release_year: Some(1900) },
    ] {
        assert!(matches!(repository.update_metadata(id, &bad), Err(CoreError::Validation(_))));
    }
    assert!(matches!(
        repository.update_metadata(999, &MetadataPatch { title: "x".into(), developer: None, release_year: None }),
        Err(CoreError::NotFound(_))
    ));
    Ok(())
}
