//! Analyses a folder of old backups and proposes what to import. Read-only: nothing is written.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use walkdir::WalkDir;

use crate::clock::to_rfc3339;
use crate::error::{CoreError, CoreResult};
use crate::saves::importer::hints::find_hint;
use crate::saves::importer::layout::{find_variants, list_folder, variants_at};
use crate::saves::importer::model::{ImportPlan, PlannedGame, PlannedLayout, PlannedSnapshot, ScanMode};
use crate::saves::importer::noise::{is_noise, is_noise_directory};
use crate::saves::importer::titles::{clean_title, is_container_name, normalize_title};

/// Folder names that say nothing about the game when looking for a second title inside a backup.
const GENERIC_INNER_NAMES: &[&str] = &[
    "saved", "savegames", "saves", "save", "steam", "windows", "users", "documents", "appdata", "local", "roaming",
    "my games", "saved games", "profiles",
];
const MAX_ALT_TITLE_DEPTH: usize = 3;

/// Reads `root` and lists the games (and their snapshots) it would produce.
pub fn scan_folder(root: &Path, mode: ScanMode) -> CoreResult<ImportPlan> {
    if !root.is_dir() {
        return Err(CoreError::Validation(format!("{} is not a folder", root.display())));
    }
    let mut warnings = Vec::new();
    let mode = resolve_mode(root, mode);
    let mut games = Vec::new();
    match mode {
        ScanMode::GamesFolder | ScanMode::Auto => {
            let listing = list_folder(root, None);
            if !listing.files.is_empty() {
                warnings.push(format!("{} loose file(s) next to the game folders were ignored", listing.files.len()));
            }
            for folder in &listing.folders {
                collect_games(folder, root, None, &mut games);
            }
            let empty = empty_folder_names(root, &listing.folders);
            if !empty.is_empty() {
                warnings.push(format!("Folders with nothing to import were skipped: {}", empty.join(", ")));
            }
        }
        ScanMode::OneGame => {
            if let Some(game) = plan_game(root, ".".into(), None, None) {
                games.push(game);
            }
        }
    }
    mark_duplicates(&mut games);
    Ok(ImportPlan { source: root.to_string_lossy().into_owned(), mode, games, warnings })
}

/// Plans one game folder. `layout` forces the reading (one snapshot, or one per sub-folder); `None`
/// lets the folder's shape decide. Returns `None` for a folder with nothing worth keeping.
pub fn plan_game(
    folder: &Path,
    key: String,
    from_collection: Option<String>,
    layout: Option<PlannedLayout>,
) -> Option<PlannedGame> {
    let folder_name = folder.file_name()?.to_string_lossy().into_owned();
    let hint = find_hint(folder);
    let hint_file = hint.as_ref().map(|found| found.file_name.as_str());
    let hint_target = hint.as_ref().map(|found| found.target.clone());

    let variants = match layout {
        Some(PlannedLayout::Single) => None,
        Some(PlannedLayout::Variants) => find_variants(folder, hint_file).or_else(|| every_folder(folder, hint_file)),
        None => find_variants(folder, hint_file),
    };

    let mut skipped_files = 0;
    let mut empty_snapshots = Vec::new();
    let (layout, snapshots) = match variants {
        Some(variants) => {
            if let Some(parent) = variants.first().and_then(|(_, path)| path.parent()) {
                let kept: Vec<PathBuf> = variants.iter().map(|(_, path)| path.clone()).collect();
                empty_snapshots = empty_folder_names(parent, &kept);
            }
            let snapshots = variants
                .into_iter()
                .filter_map(|(label, root)| {
                    let measured = measure(&root, None);
                    skipped_files += measured.skipped;
                    measured.into_snapshot(Some(label), &root, hint_target.clone(), None)
                })
                .collect::<Vec<_>>();
            (PlannedLayout::Variants, snapshots)
        }
        None => {
            let measured = measure(folder, hint_file);
            skipped_files += measured.skipped;
            let snapshot = measured.into_snapshot(
                None,
                folder,
                hint_target,
                hint_file.map(str::to_owned),
            );
            (PlannedLayout::Single, snapshot.into_iter().collect())
        }
    };
    if snapshots.is_empty() {
        return None;
    }
    Some(PlannedGame {
        key,
        folder: folder.to_string_lossy().into_owned(),
        guessed_title: clean_title(&folder_name),
        suggested_title: None,
        alt_titles: alternative_titles(folder, &folder_name),
        folder_name,
        layout,
        snapshots,
        from_collection,
        duplicate_of: None,
        skipped_files,
        empty_snapshots,
    })
}

fn resolve_mode(root: &Path, mode: ScanMode) -> ScanMode {
    if mode != ScanMode::Auto {
        return mode;
    }
    let listing = list_folder(root, None);
    if !listing.files.is_empty() || variants_at(root, None).is_some() { ScanMode::OneGame } else { ScanMode::GamesFolder }
}

/// Plans `folder` as a game, or, if it only groups games (`save`, `My Games`), the games inside it.
fn collect_games(folder: &Path, scan_root: &Path, collection: Option<&str>, games: &mut Vec<PlannedGame>) {
    let name = folder.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if is_container_name(&name) {
        for child in list_folder(folder, None).folders {
            collect_games(&child, scan_root, Some(&name), games);
        }
        return;
    }
    let key = folder
        .strip_prefix(scan_root)
        .map_or_else(|_| name.clone(), |relative| relative.to_string_lossy().replace('\\', "/"));
    if let Some(game) = plan_game(folder, key, collection.map(str::to_owned), None) {
        games.push(game);
    }
}

fn every_folder(folder: &Path, ignored_file: Option<&str>) -> Option<Vec<(String, PathBuf)>> {
    let listing = list_folder(folder, ignored_file);
    let folders: Vec<(String, PathBuf)> = listing
        .folders
        .into_iter()
        .filter_map(|path| path.file_name().map(|name| (name.to_string_lossy().into_owned(), path.clone())))
        .collect();
    (folders.len() >= 2).then_some(folders)
}

/// Names found inside a backup that may be the real title (`DSR/NBGI/DARK SOULS REMASTERED`).
fn alternative_titles(folder: &Path, folder_name: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current = folder.to_path_buf();
    for _ in 0..MAX_ALT_TITLE_DEPTH {
        let listing = list_folder(&current, None);
        let [only] = listing.folders.as_slice() else { break };
        if let Some(name) = only.file_name().map(|n| n.to_string_lossy().into_owned()) {
            let generic = GENERIC_INNER_NAMES.contains(&name.to_lowercase().as_str());
            let is_new = normalize_title(&name) != normalize_title(folder_name);
            if !generic && is_new && name.chars().any(char::is_alphabetic) && name.contains(' ') {
                found.push(clean_title(&name));
            }
        }
        current = only.clone();
    }
    found
}

/// Flags rows that share a title. Their snapshots all join the same game, so each unnamed one is
/// named after its folder to tell them apart (`Skyrim Special Edition` and `Skyrim Special Edition GOG`).
fn mark_duplicates(games: &mut [PlannedGame]) {
    let mut first_seen: HashMap<String, usize> = HashMap::new();
    let mut shared: Vec<usize> = Vec::new();
    for index in 0..games.len() {
        let Some(game) = games.get(index) else { continue };
        let normalized = normalize_title(&game.guessed_title);
        match first_seen.get(&normalized).copied() {
            Some(first) => {
                let first_key = games.get(first).map(|first_game| first_game.key.clone());
                if let Some(game) = games.get_mut(index) {
                    game.duplicate_of = first_key;
                }
                shared.extend([first, index]);
            }
            None => {
                first_seen.insert(normalized, index);
            }
        }
    }
    for index in shared {
        if let Some(game) = games.get_mut(index) {
            let name = game.folder_name.clone();
            for snapshot in game.snapshots.iter_mut().filter(|snapshot| snapshot.label.is_none()) {
                snapshot.label = Some(name.clone());
            }
        }
    }
}

/// Names of the sub-folders of `parent` that are not in `kept` (they were left out for holding no file).
fn empty_folder_names(parent: &Path, kept: &[PathBuf]) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(parent) else { return Vec::new() };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .filter(|entry| !kept.contains(&entry.path()))
        .filter(|entry| !is_noise_directory(&entry.file_name().to_string_lossy()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[derive(Default)]
struct Measured {
    files: u32,
    bytes: u64,
    newest: Option<SystemTime>,
    skipped: u32,
}

impl Measured {
    fn into_snapshot(
        self,
        label: Option<String>,
        root: &Path,
        hint: Option<String>,
        hint_file: Option<String>,
    ) -> Option<PlannedSnapshot> {
        (self.files > 0).then(|| PlannedSnapshot {
            label,
            root: root.to_string_lossy().into_owned(),
            files: self.files,
            bytes: self.bytes,
            modified_at: self.newest.map(to_rfc3339),
            hint,
            hint_file,
        })
    }
}

/// Counts what importing `root` would keep, and what it would leave out as noise.
fn measure(root: &Path, hint_file: Option<&str>) -> Measured {
    let mut measured = Measured::default();
    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() || entry.path_is_symlink() {
            continue;
        }
        let relative = entry.path().strip_prefix(root).unwrap_or(entry.path());
        let is_hint_file = hint_file.is_some_and(|name| relative == Path::new(name));
        if is_noise(relative) || is_hint_file {
            measured.skipped += u32::from(!is_hint_file);
            continue;
        }
        let Ok(metadata) = entry.metadata() else { continue };
        measured.files += 1;
        measured.bytes += metadata.len();
        if let Ok(modified) = metadata.modified() {
            measured.newest = Some(measured.newest.map_or(modified, |newest| newest.max(modified)));
        }
    }
    measured
}
