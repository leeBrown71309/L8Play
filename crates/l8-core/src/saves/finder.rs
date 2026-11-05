//! "Find saves" for games Ludusavi does not know (layers 2 and 3 of detection).
//!
//! Layer 2 asks PCGamingWiki; layer 3 looks, in a bounded way, for folders named after the game in
//! the places games keep saves. Both only *propose* folders that exist on this PC, with the reason
//! they were proposed; the user confirms before anything is backed up.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rusqlite::params;
use serde::Serialize;
use ts_rs::TS;

use crate::artwork::http::HttpGet;
use crate::artwork::steamgriddb::normalize_title;
use crate::clock::{now_plus_seconds_rfc3339, now_rfc3339, to_rfc3339};
use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::library::{GameSummary, LibraryRepository};
use crate::platforms::Platform;
use crate::saves::locations::{broad_roots_from_environment, normalize_path, validate_user_folder};
use crate::saves::pcgamingwiki::{
    PathEnvironment, PcgwClient, candidates_from_wikitext, extract_windows_paths, page_url,
};
use crate::saves::repository::SaveLocationRepository;

const PCGW_SOURCE: &str = "pcgamingwiki";
const LICENCE_NOTE: &str = "CC BY-NC-SA 3.0, PCGamingWiki contributors";
const CACHE_SECONDS: i64 = 30 * 24 * 3600;
/// Marker row meaning "the wiki has nothing for this game" (so it is not asked again for 30 days).
const NO_RULES_MARKER: &str = "";

const MAX_DIRS_PER_ROOT: usize = 600;
const MAX_FILES_SCANNED: usize = 400;
const MAX_SCAN_DEPTH: usize = 3;
const SAVE_EXTENSIONS: [&str; 16] = [
    "sav", "save", "sl2", "dat", "bin", "json", "ini", "cfg", "xml", "profile", "slot", "sdt", "db", "sqlite", "rsav",
    "savx",
];
const CODE_EXTENSIONS: [&str; 3] = ["exe", "dll", "so"];
const SAVE_FOLDER_NAMES: [&str; 5] = ["save", "saves", "savedata", "savegames", "saved"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CandidateReason {
    /// Named by a PCGamingWiki "Game data" template.
    WikiTemplate,
    /// A folder whose name matches the game's title.
    FolderNameMatchesTitle,
    /// A folder whose name matches the install folder's name.
    FolderNameMatchesInstall,
    /// A save-named folder inside the game's install folder.
    InstallSaveFolder,
}

/// A folder proposed as a save location, never used before the user confirms it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SaveCandidate {
    pub path: String,
    pub reason: CandidateReason,
    /// The wiki's own path text, or the folder name that matched.
    pub evidence: String,
    /// Wiki page to read before accepting (PCGamingWiki candidates only).
    pub evidence_url: Option<String>,
    pub file_count: u32,
    #[ts(type = "number")]
    pub total_bytes: i64,
    pub newest_modified_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum WikiStatus {
    /// A page was found (or remembered from an earlier lookup).
    Found,
    NotFound,
    /// The wiki could not be reached; only local results are shown.
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FindSavesOutcome {
    pub candidates: Vec<SaveCandidate>,
    pub wiki: WikiStatus,
    pub wiki_page_url: Option<String>,
}

fn game_key(game: &GameSummary) -> String {
    match (game.platform, game.platform_game_id.as_deref()) {
        (Platform::Steam, Some(id)) => format!("steam:{id}"),
        _ => format!("title:{}", normalize_title(&game.title)),
    }
}

struct WikiLookup {
    status: WikiStatus,
    page_url: Option<String>,
    raw_paths: Vec<String>,
}

fn cached_rules(db: &Database, key: &str) -> CoreResult<Option<WikiLookup>> {
    let connection = db.connection()?;
    let mut statement = connection.prepare(
        "SELECT path_template, source_ref FROM save_rules WHERE game_key = ?1 AND source = ?2 AND created_at > ?3",
    )?;
    let cutoff = now_plus_seconds_rfc3339(-CACHE_SECONDS);
    let rows = statement
        .query_map(params![key, PCGW_SOURCE, cutoff], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if rows.is_empty() {
        return Ok(None);
    }
    let page_url = rows.iter().find_map(|(_, url)| url.clone());
    let raw_paths: Vec<String> =
        rows.into_iter().map(|(template, _)| template).filter(|t| t != NO_RULES_MARKER).collect();
    let status = if raw_paths.is_empty() { WikiStatus::NotFound } else { WikiStatus::Found };
    Ok(Some(WikiLookup { status, page_url, raw_paths }))
}

fn store_rules(db: &Database, key: &str, page_url: Option<&str>, raw_paths: &[String]) -> CoreResult<()> {
    let mut connection = db.connection()?;
    let tx = connection.transaction()?;
    tx.execute("DELETE FROM save_rules WHERE game_key = ?1 AND source = ?2", params![key, PCGW_SOURCE])?;
    let templates: Vec<&str> =
        if raw_paths.is_empty() { vec![NO_RULES_MARKER] } else { raw_paths.iter().map(String::as_str).collect() };
    for template in templates {
        tx.execute(
            "INSERT OR IGNORE INTO save_rules (game_key, path_template, source, source_ref, licence_note, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![key, template, PCGW_SOURCE, page_url, LICENCE_NOTE, now_rfc3339()],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn lookup_wiki(db: &Database, http: &dyn HttpGet, game: &GameSummary) -> CoreResult<WikiLookup> {
    let key = game_key(game);
    if let Some(cached) = cached_rules(db, &key)? {
        return Ok(cached);
    }
    let client = PcgwClient::new(http);
    let steam_id = (game.platform == Platform::Steam).then_some(game.platform_game_id.as_deref()).flatten();
    let page = match client.find_page(steam_id, &game.title) {
        Ok(page) => page,
        Err(CoreError::External(_) | CoreError::Busy(_)) => {
            return Ok(WikiLookup { status: WikiStatus::Offline, page_url: None, raw_paths: Vec::new() });
        }
        Err(other) => return Err(other),
    };
    let Some(page) = page else {
        store_rules(db, &key, None, &[])?;
        return Ok(WikiLookup { status: WikiStatus::NotFound, page_url: None, raw_paths: Vec::new() });
    };
    let url = page_url(&page);
    let wikitext = match client.wikitext(&page) {
        Ok(text) => text.unwrap_or_default(),
        Err(CoreError::External(_) | CoreError::Busy(_)) => {
            return Ok(WikiLookup { status: WikiStatus::Offline, page_url: Some(url), raw_paths: Vec::new() });
        }
        Err(other) => return Err(other),
    };
    let raw_paths: Vec<String> = extract_windows_paths(&wikitext).into_iter().map(|path| path.raw).collect();
    store_rules(db, &key, Some(&url), &raw_paths)?;
    let status = if raw_paths.is_empty() { WikiStatus::NotFound } else { WikiStatus::Found };
    Ok(WikiLookup { status, page_url: Some(url), raw_paths })
}

/// What a folder holds, read with hard limits so a huge folder cannot stall the UI.
struct FolderProfile {
    file_count: u32,
    total_bytes: i64,
    newest: Option<SystemTime>,
    save_like: bool,
    has_code: bool,
}

fn profile_folder(folder: &Path) -> FolderProfile {
    let mut profile = FolderProfile { file_count: 0, total_bytes: 0, newest: None, save_like: false, has_code: false };
    let mut scanned = 0usize;
    for entry in walkdir::WalkDir::new(folder).max_depth(MAX_SCAN_DEPTH).follow_links(false).into_iter().flatten() {
        scanned += 1;
        if scanned > MAX_FILES_SCANNED {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let extension = entry.path().extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
        profile.has_code |= CODE_EXTENSIONS.contains(&extension.as_str());
        profile.save_like |= SAVE_EXTENSIONS.contains(&extension.as_str());
        if let Ok(metadata) = entry.metadata() {
            profile.file_count += 1;
            profile.total_bytes += i64::try_from(metadata.len()).unwrap_or(i64::MAX);
            if let Ok(modified) = metadata.modified() {
                profile.newest = Some(profile.newest.map_or(modified, |current| current.max(modified)));
            }
        }
    }
    profile
}

fn candidate(path: &Path, reason: CandidateReason, evidence: String, evidence_url: Option<String>) -> SaveCandidate {
    let profile = profile_folder(path);
    SaveCandidate {
        path: path.to_string_lossy().into_owned(),
        reason,
        evidence,
        evidence_url,
        file_count: profile.file_count,
        total_bytes: profile.total_bytes,
        newest_modified_at: profile.newest.map(to_rfc3339),
    }
}

/// Names a folder is expected to have if it belongs to this game.
fn name_keys(title: &str, install_dir: Option<&Path>) -> (Vec<String>, Option<String>) {
    let mut titles = vec![normalize_title(title)];
    let short = title.split([':', '–', '—']).next().unwrap_or(title).split(" - ").next().unwrap_or(title);
    titles.push(normalize_title(short));
    titles.retain(|key| key.chars().count() >= 4);
    titles.dedup();
    let install = install_dir
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .map(normalize_title)
        .filter(|key| key.chars().count() >= 4);
    (titles, install)
}

fn child_dirs(parent: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(parent) else { return Vec::new() };
    entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .take(MAX_DIRS_PER_ROOT)
        .map(|entry| entry.path())
        .collect()
}

fn search_roots(env: &PathEnvironment, documents: &Path) -> Vec<PathBuf> {
    vec![
        documents.to_path_buf(),
        documents.join("My Games"),
        env.userprofile.join("Saved Games"),
        env.appdata.clone(),
        env.localappdata.clone(),
        env.userprofile.join("AppData").join("LocalLow"),
        env.programdata.clone(),
    ]
}

/// Folders named after the game in the usual save places, two levels deep at most.
fn discover_local(game: &GameSummary, env: &PathEnvironment, documents: &Path) -> Vec<SaveCandidate> {
    let install_dir = game.install_path.as_deref().map(PathBuf::from);
    let (title_keys, install_key) = name_keys(&game.title, install_dir.as_deref());
    let mut found = Vec::new();

    for root in search_roots(env, documents) {
        for first in child_dirs(&root) {
            // A publisher folder ("Roaming\FromSoftware") may hold the game folder one level down.
            for folder in std::iter::once(first.clone()).chain(child_dirs(&first)) {
                let Some(name) = folder.file_name().and_then(|n| n.to_str()) else { continue };
                let key = normalize_title(name);
                let reason = if title_keys
                    .iter()
                    .any(|wanted| key == *wanted || (wanted.len() >= 6 && key.contains(wanted.as_str())))
                {
                    Some(CandidateReason::FolderNameMatchesTitle)
                } else if install_key.as_deref().is_some_and(|wanted| key == wanted) {
                    Some(CandidateReason::FolderNameMatchesInstall)
                } else {
                    None
                };
                if let Some(reason) = reason
                    && let profile = profile_folder(&folder)
                    && profile.file_count > 0
                    && !profile.has_code
                    && profile.save_like
                {
                    found.push(candidate(&folder, reason, name.to_owned(), None));
                }
            }
        }
    }

    if let Some(install) = install_dir.as_deref() {
        for first in child_dirs(install) {
            for folder in std::iter::once(first.clone()).chain(child_dirs(&first)) {
                let Some(name) = folder.file_name().and_then(|n| n.to_str()) else { continue };
                if SAVE_FOLDER_NAMES.contains(&name.to_ascii_lowercase().as_str())
                    && profile_folder(&folder).file_count > 0
                {
                    found.push(candidate(&folder, CandidateReason::InstallSaveFolder, name.to_owned(), None));
                }
            }
        }
    }
    found
}

fn is_within(inner: &str, outer: &str) -> bool {
    inner == outer || inner.starts_with(&format!("{outer}\\"))
}

/// Proposes folders for a game. Existing registered locations and nested duplicates are dropped.
pub fn find_save_candidates(
    db: &Database,
    http: &dyn HttpGet,
    game_id: i64,
    documents: &Path,
    env: &PathEnvironment,
) -> CoreResult<FindSavesOutcome> {
    let game =
        LibraryRepository::new(db).summary(game_id)?.ok_or_else(|| CoreError::NotFound(format!("game {game_id}")))?;
    let install = game.install_path.as_deref().map(PathBuf::from);
    let broad = broad_roots_from_environment();
    let registered: Vec<String> =
        SaveLocationRepository::new(db).list(game_id)?.iter().map(|l| normalize_path(&l.path_buf())).collect();

    let wiki = lookup_wiki(db, http, &game)?;
    let mut all: Vec<SaveCandidate> = Vec::new();
    let wiki_text: String = wiki.raw_paths.iter().map(|raw| format!("{{{{Game data/saves|Windows|{raw}}}}}")).collect();
    for found in candidates_from_wikitext(&wiki_text, env) {
        all.push(candidate(&found.path, CandidateReason::WikiTemplate, found.template, wiki.page_url.clone()));
    }
    all.extend(discover_local(&game, env, documents));

    let mut kept: Vec<SaveCandidate> = Vec::new();
    for item in all {
        let path = PathBuf::from(&item.path);
        if validate_user_folder(&path, install.as_deref(), &broad).is_err() {
            continue;
        }
        let normalized = normalize_path(&path);
        let already = registered.iter().any(|known| is_within(&normalized, known) || is_within(known, &normalized));
        let duplicate = kept.iter().any(|other| {
            let other = normalize_path(Path::new(&other.path));
            is_within(&normalized, &other) || is_within(&other, &normalized)
        });
        if !already && !duplicate {
            kept.push(item);
        }
    }
    Ok(FindSavesOutcome { candidates: kept, wiki: wiki.status, wiki_page_url: wiki.page_url })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::artwork::http::HttpFailure;

    struct FakeHttp {
        answers: RefCell<Vec<Result<String, HttpFailure>>>,
        calls: RefCell<usize>,
    }

    impl HttpGet for FakeHttp {
        fn get_text(&self, _url: &str, _bearer: Option<&str>) -> Result<String, HttpFailure> {
            *self.calls.borrow_mut() += 1;
            self.answers.borrow_mut().remove(0)
        }

        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>, HttpFailure> {
            Err(HttpFailure::NotFound)
        }
    }

    fn env(root: &Path) -> PathEnvironment {
        PathEnvironment {
            appdata: root.join("AppData").join("Roaming"),
            localappdata: root.join("AppData").join("Local"),
            userprofile: root.to_path_buf(),
            programdata: root.join("ProgramData"),
            username: "ryu".into(),
            install_dir: None,
        }
    }

    fn add_game(db: &Database, title: &str, install: Option<&Path>) -> CoreResult<i64> {
        let connection = db.connection()?;
        connection.execute(
            "INSERT INTO games (platform_id, title, sort_title, slug, added_at, updated_at)
             VALUES ('standalone', ?1, lower(?1), lower(?1), 'x', 'x')",
            [title],
        )?;
        let id = connection.last_insert_rowid();
        if let Some(path) = install {
            connection.execute(
                "INSERT INTO installations (game_id, install_path, source, detected_at) VALUES (?1, ?2, 'manual', 'x')",
                params![id, path.to_string_lossy()],
            )?;
        }
        Ok(id)
    }

    fn write(path: &Path, bytes: &[u8]) -> CoreResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)?;
        Ok(())
    }

    fn offline_http() -> FakeHttp {
        FakeHttp { answers: RefCell::new(vec![Err(HttpFailure::Other("down".into()))]), calls: RefCell::new(0) }
    }

    #[test]
    fn local_discovery_finds_a_folder_named_after_the_game() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        let env = env(dir.path());
        let documents = dir.path().join("Documents");
        write(&env.appdata.join("Hollow Knight").join("user1.dat"), b"save")?;
        write(&env.appdata.join("Unrelated").join("user1.dat"), b"save")?;
        write(&env.localappdata.join("Hollow Knight Launcher").join("launcher.exe"), b"MZ")?;
        let db = Database::open_in_memory()?;
        let id = add_game(&db, "Hollow Knight", None)?;

        let outcome = find_save_candidates(&db, &offline_http(), id, &documents, &env)?;
        assert_eq!(outcome.wiki, WikiStatus::Offline);
        assert_eq!(outcome.candidates.len(), 1);
        let found = &outcome.candidates[0];
        assert_eq!(found.reason, CandidateReason::FolderNameMatchesTitle);
        assert!(found.path.ends_with("Hollow Knight"));
        assert_eq!(found.file_count, 1);
        Ok(())
    }

    #[test]
    fn publisher_subfolders_and_install_save_folders_are_found() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        let env = env(dir.path());
        let documents = dir.path().join("Documents");
        let install = dir.path().join("Games").join("Foo Quest");
        write(&env.appdata.join("Acme Studio").join("Foo Quest").join("slot1.sav"), b"s")?;
        write(&install.join("Saves").join("a.bin"), b"s")?;
        write(&install.join("foo.exe"), b"MZ")?;
        let db = Database::open_in_memory()?;
        let id = add_game(&db, "Foo Quest: Remastered", Some(&install))?;

        let outcome = find_save_candidates(&db, &offline_http(), id, &documents, &env)?;
        let reasons: Vec<CandidateReason> = outcome.candidates.iter().map(|c| c.reason).collect();
        assert!(reasons.contains(&CandidateReason::FolderNameMatchesTitle));
        assert!(reasons.contains(&CandidateReason::InstallSaveFolder));
        assert_eq!(outcome.candidates.len(), 2);
        Ok(())
    }

    #[test]
    fn already_registered_folders_are_not_proposed_again() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        let env = env(dir.path());
        let documents = dir.path().join("Documents");
        let folder = env.appdata.join("Hollow Knight");
        write(&folder.join("user1.dat"), b"save")?;
        let db = Database::open_in_memory()?;
        let id = add_game(&db, "Hollow Knight", None)?;
        SaveLocationRepository::new(&db).add_user_folder(id, &folder)?;
        let outcome = find_save_candidates(&db, &offline_http(), id, &documents, &env)?;
        assert!(outcome.candidates.is_empty());
        Ok(())
    }

    #[test]
    fn wiki_results_are_cached_and_merged_with_local_ones() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        let env = env(dir.path());
        let documents = dir.path().join("Documents");
        write(&env.appdata.join("Foo").join("slot.sav"), b"save")?;
        write(&documents.join("Foo Game").join("save.sav"), b"save")?;
        let wikitext = r"{{Game data/saves|Windows|{{p|appdata}}\Foo}}";
        let parse = format!(r#"{{"parse":{{"wikitext":{}}}}}"#, serde_json::to_string(wikitext).unwrap_or_default());
        let http = FakeHttp {
            answers: RefCell::new(vec![Ok(r#"{"query":{"pages":[{"title":"Foo Game"}]}}"#.into()), Ok(parse)]),
            calls: RefCell::new(0),
        };
        let db = Database::open_in_memory()?;
        let id = add_game(&db, "Foo Game", None)?;

        let first = find_save_candidates(&db, &http, id, &documents, &env)?;
        assert_eq!(first.wiki, WikiStatus::Found);
        assert_eq!(first.wiki_page_url.as_deref(), Some("https://www.pcgamingwiki.com/wiki/Foo_Game"));
        let reasons: Vec<CandidateReason> = first.candidates.iter().map(|c| c.reason).collect();
        assert_eq!(reasons, vec![CandidateReason::WikiTemplate, CandidateReason::FolderNameMatchesTitle]);
        assert_eq!(*http.calls.borrow(), 2);

        let second = find_save_candidates(&db, &http, id, &documents, &env)?;
        assert_eq!(second.candidates, first.candidates);
        assert_eq!(*http.calls.borrow(), 2, "the second search is answered from the local rules cache");
        Ok(())
    }

    #[test]
    fn a_game_the_wiki_does_not_know_is_remembered_as_such() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        let env = env(dir.path());
        let http = FakeHttp {
            answers: RefCell::new(vec![
                Ok(r#"{"query":{"pages":[{"title":"Nobody Knows","missing":true}]}}"#.into()),
                Ok(r#"["Nobody Knows",[],[],[]]"#.into()),
            ]),
            calls: RefCell::new(0),
        };
        let db = Database::open_in_memory()?;
        let id = add_game(&db, "Nobody Knows", None)?;
        let documents = dir.path().join("Documents");
        let first = find_save_candidates(&db, &http, id, &documents, &env)?;
        assert_eq!(first.wiki, WikiStatus::NotFound);
        let second = find_save_candidates(&db, &http, id, &documents, &env)?;
        assert_eq!(second.wiki, WikiStatus::NotFound);
        assert_eq!(*http.calls.borrow(), 2, "the second search is answered from the rules cache");
        Ok(())
    }
}
