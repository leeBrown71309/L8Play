//! Looks for the saves of games that are no longer installed, on this PC.
//!
//! After a game is uninstalled its saves usually stay behind (`AppData`, `Documents`, `Saved Games`,
//! the Steam `userdata` folder…). Ludusavi's database knows where thousands of games keep theirs, so
//! one preview run lists every game that still has files here. Games that are installed (they are
//! in the library already) are left out; the rest becomes an [`ImportPlan`] that goes through the
//! same review and import steps as a backup folder. Read-only: nothing is copied until the user confirms.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::db::Database;
use crate::error::CoreResult;
use crate::library::LibraryRepository;
use crate::saves::importer::model::{ImportPlan, PlannedGame, PlannedLayout, ScanMode};
use crate::saves::importer::scan::measure;
use crate::saves::importer::titles::normalize_title;
use crate::saves::locations::{LocationKind, broad_roots_from_environment, derive_locations};
use crate::saves::{LudusaviCli, remembered_title};

/// Scans this PC and proposes the leftover saves of games that are not installed.
pub fn scan_pc_for_saves(db: &Database, cli: &LudusaviCli) -> CoreResult<ImportPlan> {
    let found = cli.preview_all()?;
    let installed = installed_titles(db)?;
    let broad_roots = broad_roots_from_environment();

    let mut games = Vec::new();
    let mut warnings = Vec::new();
    let mut file_only = 0u32;
    for (title, files) in found {
        if installed.contains(&normalize_title(&title)) {
            continue;
        }
        let locations = derive_locations(&files, None, &broad_roots);
        let folders: Vec<PathBuf> = locations
            .iter()
            .filter(|location| location.kind == LocationKind::Directory)
            .map(|location| location.path.clone())
            .collect();
        if folders.len() < locations.len() {
            file_only += 1;
        }
        if let Some(game) = plan_orphan(&title, &folders) {
            games.push(game);
        }
    }
    games.sort_by_key(|game| game.guessed_title.to_lowercase());
    if file_only > 0 {
        warnings.push(format!(
            "{file_only} game(s) keep a save as a loose file in a shared folder; only their dedicated save folders are listed."
        ));
    }
    Ok(ImportPlan { source: "this PC".into(), mode: ScanMode::GamesFolder, games, warnings })
}

/// Normalized titles of every installed game, plus the database titles remembered for them.
fn installed_titles(db: &Database) -> CoreResult<HashSet<String>> {
    let mut titles = HashSet::new();
    for game in LibraryRepository::new(db).list()?.into_iter().filter(|game| game.installed) {
        titles.insert(normalize_title(&game.title));
        if let Some(remembered) = remembered_title(db, game.id)?.filter(|title| !title.is_empty()) {
            titles.insert(normalize_title(&remembered));
        }
    }
    Ok(titles)
}

/// One planned game: one snapshot per save folder, restorable to where the files were found.
fn plan_orphan(title: &str, folders: &[PathBuf]) -> Option<PlannedGame> {
    let single = folders.len() == 1;
    let mut snapshots = Vec::new();
    let mut skipped_files = 0u32;
    for folder in folders {
        let measured = measure(folder, None);
        skipped_files += measured.skipped;
        let label = (!single).then(|| folder_label(folder));
        if let Some(snapshot) = measured.into_snapshot(label, folder, Some(folder.to_string_lossy().into_owned()), None)
        {
            snapshots.push(snapshot);
        }
    }
    let first = folders.first()?;
    (!snapshots.is_empty()).then(|| PlannedGame {
        key: title.to_owned(),
        folder: first.to_string_lossy().into_owned(),
        folder_name: title.to_owned(),
        guessed_title: title.to_owned(),
        suggested_title: Some(title.to_owned()),
        alt_titles: Vec::new(),
        layout: if single { PlannedLayout::Single } else { PlannedLayout::Variants },
        snapshots,
        from_collection: None,
        duplicate_of: None,
        skipped_files,
        empty_snapshots: Vec::new(),
    })
}

/// A short name for a save folder, from its last two path components.
fn folder_label(folder: &Path) -> String {
    let parts: Vec<String> = folder.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect();
    parts[parts.len().saturating_sub(2)..].join("\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_save_folder_becomes_a_restorable_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let folder = dir.path().join("Hades");
        std::fs::create_dir_all(&folder)?;
        std::fs::write(folder.join("Profile1.sav"), b"data")?;
        std::fs::write(folder.join("crash.log"), b"noise")?;

        let game = plan_orphan("Hades", std::slice::from_ref(&folder)).ok_or("no plan")?;
        assert_eq!(game.layout, PlannedLayout::Single);
        assert_eq!(game.snapshots.len(), 1);
        assert_eq!(game.snapshots[0].files, 1, "logs are left out");
        assert_eq!(game.snapshots[0].hint.as_deref(), Some(folder.to_string_lossy().as_ref()));
        assert!(game.snapshots[0].label.is_none());
        Ok(())
    }

    #[test]
    fn several_folders_become_named_snapshots_and_empty_ones_are_dropped() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let one = dir.path().join("a").join("Saves");
        let two = dir.path().join("b").join("Config");
        let empty = dir.path().join("c");
        for folder in [&one, &two, &empty] {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(one.join("s.sav"), b"1")?;
        std::fs::write(two.join("c.ini"), b"2")?;

        let game = plan_orphan("Game", &[one, two, empty]).ok_or("no plan")?;
        assert_eq!(game.layout, PlannedLayout::Variants);
        assert_eq!(game.snapshots.len(), 2);
        assert!(game.snapshots.iter().all(|snapshot| snapshot.label.is_some()));
        Ok(())
    }

    #[test]
    fn nothing_to_import_gives_no_plan() {
        assert!(plan_orphan("Game", &[]).is_none());
        assert!(plan_orphan("Game", &[PathBuf::from(r"Z:\does\not\exist")]).is_none());
    }
}
