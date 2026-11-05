//! "Scan a folder for games": walks a folder the user picked, groups the executables it finds by
//! game folder, hides installers and helper programs, suggests the most likely main executable, and
//! leaves out everything already in the library so a rescan never proposes duplicates.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;
use walkdir::{DirEntry, WalkDir};

use crate::artwork::steamgriddb::normalize_title;
use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::saves::normalize_path;

const MAX_DEPTH: usize = 4;
const MAX_ENTRIES_PER_FOLDER: usize = 5_000;
const MAX_CANDIDATES: usize = 300;
const MAX_EXECUTABLES_PER_GAME: usize = 40;

/// Programs that are never the game itself (installers, redistributables, crash reporters, anti-cheat).
const JUNK_NAME_PARTS: &[&str] = &[
    "unins",
    "uninstall",
    "crashhandler",
    "crashreport",
    "crashpad",
    "vc_redist",
    "vcredist",
    "dxsetup",
    "dxwebsetup",
    "dotnet",
    "ndp4",
    "prereq",
    "easyanticheat",
    "beservice",
    "battleye",
    "installer",
    "setup",
    "redist",
    "oalinst",
    "notification_helper",
    "cefprocess",
    "helper",
    "crs-",
    "sumatra",
    "7za",
];

/// Folders that never hold a game's executable.
const JUNK_DIRS: &[&str] = &[
    "_commonredist",
    "commonredist",
    "redist",
    "redistributables",
    "directx",
    "vcredist",
    "__installer",
    "$recycle.bin",
    "system volume information",
    "node_modules",
];

/// Names that make an executable a less likely "play" target.
const SECONDARY_NAME_PARTS: &[&str] = &["launcher", "config", "settings", "editor", "server", "benchmark", "tool"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExecutableChoice {
    pub path: String,
    /// Path inside the game folder, for display.
    pub relative_path: String,
    #[ts(type = "number")]
    pub size_bytes: i64,
    /// The most likely main executable (at most one per game).
    pub suggested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameFolderCandidate {
    pub folder: String,
    pub suggested_title: String,
    /// Best guess first.
    pub executables: Vec<ExecutableChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FolderScanReport {
    /// Games not yet in the library.
    pub candidates: Vec<GameFolderCandidate>,
    /// Folders found but already in the library (hidden from the list).
    pub already_added: u32,
}

/// What the library already holds, to avoid proposing duplicates.
struct Known {
    folders: Vec<String>,
    executables: HashSet<String>,
    titles: HashSet<String>,
}

impl Known {
    fn load(db: &Database) -> CoreResult<Self> {
        let connection = db.connection()?;
        let mut folders = Vec::new();
        let mut executables = HashSet::new();
        let mut statement = connection.prepare("SELECT install_path, executable_path FROM installations")?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        for (folder, executable) in rows {
            folders.push(normalize_path(Path::new(&folder)));
            if let Some(executable) = executable {
                executables.insert(normalize_path(Path::new(&executable)));
            }
        }
        let mut statement = connection.prepare("SELECT title FROM games")?;
        let titles = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .map(|t| normalize_title(t))
            .collect();
        Ok(Self { folders, executables, titles })
    }

    fn contains(&self, folder: &Path, title: &str, executables: &[ExecutableChoice]) -> bool {
        let candidate = normalize_path(folder);
        let nested = |outer: &str, inner: &str| inner == outer || inner.starts_with(&format!("{outer}\\"));
        self.folders.iter().any(|known| nested(known, &candidate) || nested(&candidate, known))
            || executables.iter().any(|exe| self.executables.contains(&normalize_path(Path::new(&exe.path))))
            || self.titles.contains(&normalize_title(title))
    }
}

fn is_junk_executable(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    JUNK_NAME_PARTS.iter().any(|part| lower.contains(part))
}

fn is_junk_dir(entry: &DirEntry) -> bool {
    entry.file_type().is_dir()
        && entry.depth() > 0
        && entry.file_name().to_str().is_some_and(|name| JUNK_DIRS.contains(&name.to_lowercase().as_str()))
}

fn is_executable_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
        && entry.path().extension().is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        && entry.file_name().to_str().is_some_and(|name| !is_junk_executable(name))
}

/// Higher is more likely to be the game's main executable.
fn score(stem: &str, folder_key: &str, depth: usize, size: u64) -> i64 {
    let key = normalize_title(stem);
    let mut points = 0i64;
    if !key.is_empty() && key == folder_key {
        points += 100;
    } else if key.len() >= 3 && (folder_key.contains(&key) || key.contains(folder_key)) && !folder_key.is_empty() {
        points += 60;
    }
    let lower = stem.to_lowercase();
    if SECONDARY_NAME_PARTS.iter().any(|part| lower.contains(part)) {
        points -= 40;
    }
    points -= 10 * i64::try_from(depth).unwrap_or(0);
    // Among equals, the shorter name wins ("game" over "game-launcher-helper").
    points -= i64::try_from(stem.len() / 6).unwrap_or(0);
    // Bigger programs are more likely the game: up to 20 points, one per doubling above 1 MB.
    let megabytes = (size / (1024 * 1024)).max(1);
    points += i64::from(megabytes.ilog2()).min(20);
    points
}

/// Executables of one game folder, best guess first, with the suggestion flagged.
fn executables_of(folder: &Path) -> Vec<ExecutableChoice> {
    let folder_key = folder.file_name().and_then(|name| name.to_str()).map(normalize_title).unwrap_or_default();
    let mut found: Vec<(i64, ExecutableChoice)> = Vec::new();
    let walker =
        WalkDir::new(folder).max_depth(MAX_DEPTH).follow_links(false).into_iter().filter_entry(|e| !is_junk_dir(e));
    for entry in walker.flatten().take(MAX_ENTRIES_PER_FOLDER) {
        if !is_executable_file(&entry) {
            continue;
        }
        let size = entry.metadata().map_or(0, |metadata| metadata.len());
        let stem = entry.path().file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
        let relative = entry.path().strip_prefix(folder).unwrap_or(entry.path()).to_string_lossy().into_owned();
        found.push((
            score(stem, &folder_key, entry.depth().saturating_sub(1), size),
            ExecutableChoice {
                path: entry.path().to_string_lossy().into_owned(),
                relative_path: relative,
                size_bytes: i64::try_from(size).unwrap_or(i64::MAX),
                suggested: false,
            },
        ));
    }
    found.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.relative_path.len().cmp(&b.1.relative_path.len()))
            .then_with(|| a.1.relative_path.cmp(&b.1.relative_path))
    });
    found.truncate(MAX_EXECUTABLES_PER_GAME);
    let mut executables: Vec<ExecutableChoice> = found.into_iter().map(|(_, exe)| exe).collect();
    if let Some(first) = executables.first_mut() {
        first.suggested = true;
    }
    executables
}

/// "the_last_of_us" and "Hollow.Knight" become readable titles; names with spaces are kept.
fn title_from_folder(folder: &Path) -> String {
    let name = folder.file_name().and_then(|name| name.to_str()).unwrap_or("Game");
    let readable = if name.contains(' ') { name.to_owned() } else { name.replace(['_', '.'], " ") };
    let collapsed = readable.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() { "Game".to_owned() } else { collapsed }
}

fn child_folders(root: &Path) -> CoreResult<Vec<PathBuf>> {
    let mut folders: Vec<PathBuf> = std::fs::read_dir(root)?
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| !JUNK_DIRS.contains(&name.to_lowercase().as_str()) && !name.starts_with('.'))
        })
        .map(|entry| entry.path())
        .collect();
    folders.sort_by_key(|path| path.to_string_lossy().to_lowercase());
    Ok(folders)
}

fn has_direct_executable(folder: &Path) -> bool {
    std::fs::read_dir(folder).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_file())
                && entry.path().extension().is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
                && entry.file_name().to_str().is_some_and(|name| !is_junk_executable(name))
        })
    })
}

/// Scans `root` for games not yet in the library. If `root` itself holds an executable it is one
/// game; otherwise each sub-folder is a game of its own.
pub fn scan_game_folder(db: &Database, root: &Path) -> CoreResult<FolderScanReport> {
    if !root.is_dir() {
        return Err(CoreError::Validation(format!("{} is not a folder", root.display())));
    }
    let known = Known::load(db)?;
    let game_folders = if has_direct_executable(root) { vec![root.to_path_buf()] } else { child_folders(root)? };

    let mut report = FolderScanReport { candidates: Vec::new(), already_added: 0 };
    for folder in game_folders.into_iter().take(MAX_CANDIDATES * 2) {
        let executables = executables_of(&folder);
        if executables.is_empty() {
            continue;
        }
        let title = title_from_folder(&folder);
        if known.contains(&folder, &title, &executables) {
            report.already_added += 1;
        } else if report.candidates.len() < MAX_CANDIDATES {
            report.candidates.push(GameFolderCandidate {
                folder: folder.to_string_lossy().into_owned(),
                suggested_title: title,
                executables,
            });
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::LibraryRepository;

    fn touch(path: &Path, size: usize) -> CoreResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, vec![b'x'; size])?;
        Ok(())
    }

    /// A library folder shaped like the real one: several executables per game.
    fn library(root: &Path) -> CoreResult<()> {
        touch(&root.join("The Last of Us").join("tlou-i.exe"), 5_000_000)?;
        touch(&root.join("The Last of Us").join("launcher.exe"), 500_000)?;
        touch(&root.join("The Last of Us").join("unins000.exe"), 900_000)?;
        touch(&root.join("The Last of Us").join("_CommonRedist").join("vcredist_x64.exe"), 9_000_000)?;
        touch(&root.join("Moonlighter").join("Moonlighter.exe"), 20_000_000)?;
        touch(&root.join("Moonlighter").join("data").join("UnityCrashHandler64.exe"), 1_000_000)?;
        touch(&root.join("Hollow_Knight").join("bin").join("hollow_knight.exe"), 30_000_000)?;
        touch(&root.join("Only Data").join("readme.txt"), 10)?;
        Ok(())
    }

    #[test]
    fn groups_executables_per_game_folder_and_hides_helpers() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        library(dir.path())?;
        let db = Database::open_in_memory()?;
        let report = scan_game_folder(&db, dir.path())?;
        let titles: Vec<&str> = report.candidates.iter().map(|c| c.suggested_title.as_str()).collect();
        assert_eq!(
            titles,
            vec!["Hollow Knight", "Moonlighter", "The Last of Us"],
            "folders without executables are skipped"
        );

        let tlou = &report.candidates[2];
        let names: Vec<&str> = tlou.executables.iter().map(|e| e.relative_path.as_str()).collect();
        assert_eq!(names, vec!["tlou-i.exe", "launcher.exe"], "uninstaller and redistributables are hidden");
        assert!(tlou.executables[0].suggested && !tlou.executables[1].suggested);

        let moonlighter = &report.candidates[1];
        assert_eq!(moonlighter.executables.len(), 1, "the crash handler is hidden");
        let hollow = &report.candidates[0];
        assert_eq!(hollow.executables[0].relative_path, "bin\\hollow_knight.exe");
        Ok(())
    }

    #[test]
    fn a_folder_that_holds_an_executable_is_a_single_game() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        library(dir.path())?;
        let db = Database::open_in_memory()?;
        let report = scan_game_folder(&db, &dir.path().join("Moonlighter"))?;
        assert_eq!(report.candidates.len(), 1);
        assert_eq!(report.candidates[0].suggested_title, "Moonlighter");
        Ok(())
    }

    #[test]
    fn games_already_in_the_library_are_not_proposed_again() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        library(dir.path())?;
        let db = Database::open_in_memory()?;
        let repository = LibraryRepository::new(&db);
        let first = scan_game_folder(&db, dir.path())?;
        assert_eq!(first.candidates.len(), 3);

        // The user adds Moonlighter with its game folder, and Hollow Knight by its executable alone.
        let moonlighter = &first.candidates[1];
        repository.add_standalone_in(
            Some(Path::new(&moonlighter.folder)),
            Path::new(&moonlighter.executables[0].path),
            Some("Moonlighter"),
        )?;
        let hollow = &first.candidates[0];
        repository.add_standalone(Path::new(&hollow.executables[0].path), Some("Whatever name"))?;

        let second = scan_game_folder(&db, dir.path())?;
        let titles: Vec<&str> = second.candidates.iter().map(|c| c.suggested_title.as_str()).collect();
        assert_eq!(titles, vec!["The Last of Us"]);
        assert_eq!(second.already_added, 2);

        // A new game appears later: only that one is proposed.
        touch(&dir.path().join("New Game").join("new_game.exe"), 3_000_000)?;
        let third = scan_game_folder(&db, dir.path())?;
        let titles: Vec<&str> = third.candidates.iter().map(|c| c.suggested_title.as_str()).collect();
        assert_eq!(titles, vec!["New Game", "The Last of Us"]);
        Ok(())
    }

    #[test]
    fn a_game_with_the_same_title_is_not_proposed_even_from_another_folder() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        library(dir.path())?;
        let db = Database::open_in_memory()?;
        let elsewhere = tempfile::tempdir()?;
        touch(&elsewhere.path().join("moonlighter.exe"), 3_000_000)?;
        LibraryRepository::new(&db).add_standalone(&elsewhere.path().join("moonlighter.exe"), Some("Moonlighter"))?;
        let report = scan_game_folder(&db, dir.path())?;
        assert!(report.candidates.iter().all(|c| c.suggested_title != "Moonlighter"));
        assert_eq!(report.already_added, 1);
        Ok(())
    }

    #[test]
    fn adding_inside_a_folder_validates_and_stays_idempotent() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        library(dir.path())?;
        let db = Database::open_in_memory()?;
        let repository = LibraryRepository::new(&db);
        let folder = dir.path().join("The Last of Us");
        let exe = folder.join("tlou-i.exe");
        let added = repository.add_standalone_in(Some(&folder), &exe, Some("The Last of Us Part I"))?;
        assert!(added.created);
        assert_eq!(added.game.install_path.as_deref(), Some(folder.to_string_lossy().as_ref()));
        let again = repository.add_standalone_in(Some(&folder), &exe, Some("Other title"))?;
        assert!(!again.created);
        assert_eq!(again.game.id, added.game.id);
        let outside = dir.path().join("Moonlighter").join("Moonlighter.exe");
        assert!(matches!(repository.add_standalone_in(Some(&folder), &outside, None), Err(CoreError::Validation(_))));
        Ok(())
    }

    #[test]
    fn titles_are_made_readable() {
        assert_eq!(title_from_folder(Path::new("D:\\Games\\the_last_of_us")), "the last of us");
        assert_eq!(title_from_folder(Path::new("D:\\Games\\Hollow.Knight")), "Hollow Knight");
        assert_eq!(title_from_folder(Path::new("D:\\Games\\Elden Ring")), "Elden Ring");
    }

    #[test]
    fn scoring_prefers_the_exe_named_like_the_folder() {
        let folder = normalize_title("Moonlighter");
        assert!(score("Moonlighter", &folder, 0, 20_000_000) > score("launcher", &folder, 0, 20_000_000));
        assert!(score("Moonlighter", &folder, 0, 1_000_000) > score("other", &folder, 0, 90_000_000));
    }

    /// Reads a real folder: `L8_SCAN_DIR=D:\\Workspace\\Games cargo test -p l8-core live_scan -- --ignored --nocapture`.
    #[test]
    #[ignore = "needs a real folder in L8_SCAN_DIR"]
    fn live_scan_of_a_real_folder() -> CoreResult<()> {
        let Some(dir) = std::env::var_os("L8_SCAN_DIR") else { return Ok(()) };
        let db = Database::open_in_memory()?;
        let report = scan_game_folder(&db, Path::new(&dir))?;
        for candidate in &report.candidates {
            println!("{} ({})", candidate.suggested_title, candidate.folder);
            for exe in &candidate.executables {
                println!(
                    "   {} {} ({} bytes)",
                    if exe.suggested { "*" } else { " " },
                    exe.relative_path,
                    exe.size_bytes
                );
            }
        }
        Ok(())
    }
}
