//! The importer on folder trees shaped like real, hand-made backup folders.

use std::fs;
use std::path::Path;

use crate::db::Database;
use crate::library::LibraryRepository;
use crate::saves::importer::{
    ImportGameRequest, PlannedGame, PlannedLayout, ScanMode, import_games, scan_folder,
};
use crate::snapshots::{SnapshotEngine, SnapshotRepository, SnapshotStorage};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write(root: &Path, relative: &str, content: &str) -> TestResult {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// A folder of backups with the shapes found in a real "Save Games" folder.
fn sample_backup_folder() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    // Named snapshots one level down.
    for (name, content) in [("after ending A", "a"), ("after ending B", "b"), ("before final choice", "c")] {
        write(root, &format!("Stellar Blade/{name}/SaveGames/76561199774717161/slot1.sav"), content)?;
        write(root, &format!("Stellar Blade/{name}/SaveGames/Backup/slot1.bak"), content)?;
        write(root, &format!("Stellar Blade/{name}/SaveGames/StellarBladeSetting.sav"), "settings")?;
    }
    // Named snapshots two levels down, with a hint file and differently shaped variants.
    write(root, "The First Berserker Khazan/location.txt", r"Type %LOCALAPPDATA%\The First Berserker Khazan\Saved\SaveGames in the box and press Enter.")?;
    write(root, "The First Berserker Khazan/Saved/1er fin CHAOS 3 PHASES/76561199875893403/slot.sav", "1")?;
    write(root, "The First Berserker Khazan/Saved/2eme fin/direct.sav", "2")?;
    write(root, "The First Berserker Khazan/Saved/BEFORE ANY CHOICE/76561199875893403/slot.sav", "3")?;
    // One plain save plus a setting file next to it.
    write(root, "EldenRing/76561197960267366/ER0000.sav", "elden")?;
    write(root, "EldenRing/GraphicsConfig.xml", "<xml/>")?;
    // Timestamped slots are not named snapshots.
    write(root, "outward/SaveGames/76561198988209285/Save_eCjgz0F4YUuqOxInfRGCJA/20231225013034/a.dat", "1")?;
    write(root, "outward/SaveGames/76561198988209285/Save_eCjgz0F4YUuqOxInfRGCJA/20231225013601/a.dat", "2")?;
    // Noise is left out.
    write(root, "God of War/slot.save", "gow")?;
    write(root, "God of War/ScreenShot-2023-4-10_20-29-32.png", "png")?;
    write(root, "God of War/GodOfWar.log", "log")?;
    // A collection folder repeating a game that also sits at the top level.
    write(root, "save/My Games/DarkSoulsII/0110000100001337/DS2.sl2", "ds2")?;
    write(root, "save/NBGI/DARK SOULS REMASTERED/DRAKS0005.sl2", "dsr")?;
    write(root, "DarkSoulsII/0110000100001337/DS2.sl2", "ds2")?;
    Ok(dir)
}

fn find<'a>(games: &'a [PlannedGame], title: &str) -> Result<&'a PlannedGame, String> {
    games.iter().find(|game| game.guessed_title == title).ok_or_else(|| format!("no game titled {title}"))
}

#[test]
fn sub_folders_that_are_named_snapshots_are_detected_at_any_depth() -> TestResult {
    let dir = sample_backup_folder()?;
    let plan = scan_folder(dir.path(), ScanMode::Auto)?;
    assert_eq!(plan.mode, ScanMode::GamesFolder);

    let stellar = find(&plan.games, "Stellar Blade")?;
    assert_eq!(stellar.layout, PlannedLayout::Variants);
    let labels: Vec<_> = stellar.snapshots.iter().filter_map(|s| s.label.as_deref()).collect();
    assert_eq!(labels, ["after ending A", "after ending B", "before final choice"]);

    let khazan = find(&plan.games, "The First Berserker Khazan")?;
    assert_eq!(khazan.layout, PlannedLayout::Variants);
    assert_eq!(khazan.snapshots.len(), 3);
    for snapshot in &khazan.snapshots {
        assert_eq!(
            snapshot.hint.as_deref(),
            Some(r"%LOCALAPPDATA%\The First Berserker Khazan\Saved\SaveGames")
        );
    }
    Ok(())
}

#[test]
fn identifiers_and_settings_do_not_create_fake_snapshots() -> TestResult {
    let dir = sample_backup_folder()?;
    let plan = scan_folder(dir.path(), ScanMode::Auto)?;

    let elden = find(&plan.games, "Elden Ring")?;
    assert_eq!((elden.layout, elden.snapshots.len()), (PlannedLayout::Single, 1));
    let outward = find(&plan.games, "Outward")?;
    assert_eq!((outward.layout, outward.snapshots.len()), (PlannedLayout::Single, 1));
    assert_eq!(outward.snapshots[0].files, 2);
    Ok(())
}

#[test]
fn noise_is_counted_but_not_kept() -> TestResult {
    let dir = sample_backup_folder()?;
    let plan = scan_folder(dir.path(), ScanMode::Auto)?;
    let game = find(&plan.games, "God of War")?;
    assert_eq!(game.snapshots[0].files, 1);
    assert_eq!(game.skipped_files, 2);
    Ok(())
}

#[test]
fn collection_folders_are_expanded_and_duplicates_flagged() -> TestResult {
    let dir = sample_backup_folder()?;
    let plan = scan_folder(dir.path(), ScanMode::Auto)?;

    let from_collection: Vec<_> = plan.games.iter().filter(|game| game.from_collection.is_some()).collect();
    assert_eq!(from_collection.len(), 2);
    assert!(plan.games.iter().all(|game| game.guessed_title != "save"));

    let titles: Vec<_> = plan.games.iter().filter(|game| game.guessed_title == "Dark Souls II").collect();
    assert_eq!(titles.len(), 2);
    assert_eq!(titles.iter().filter(|game| game.duplicate_of.is_some()).count(), 1);

    let remastered = plan.games.iter().find(|game| game.folder_name == "NBGI").ok_or("NBGI not found")?;
    assert!(remastered.alt_titles.iter().any(|title| title == "DARK SOULS REMASTERED"));
    Ok(())
}

#[test]
fn a_folder_of_snapshots_alone_is_read_as_one_game() -> TestResult {
    let dir = sample_backup_folder()?;
    let plan = scan_folder(&dir.path().join("Stellar Blade"), ScanMode::Auto)?;
    assert_eq!(plan.mode, ScanMode::OneGame);
    assert_eq!(plan.games.len(), 1);
    assert_eq!(plan.games[0].guessed_title, "Stellar Blade");
    assert_eq!(plan.games[0].snapshots.len(), 3);
    Ok(())
}

#[test]
fn forcing_the_layout_overrides_the_guess() -> TestResult {
    let dir = sample_backup_folder()?;
    let folder = dir.path().join("Stellar Blade");
    let single = super::plan_game(&folder, "Stellar Blade".into(), None, Some(PlannedLayout::Single))
        .ok_or("nothing planned")?;
    assert_eq!((single.layout, single.snapshots.len()), (PlannedLayout::Single, 1));
    assert_eq!(single.snapshots[0].files, 9);
    Ok(())
}

#[test]
fn importing_creates_named_imported_snapshots_once() -> TestResult {
    let dir = sample_backup_folder()?;
    let snapshots_root = tempfile::tempdir()?;
    let db = Database::open_in_memory()?;
    let plan = scan_folder(dir.path(), ScanMode::Auto)?;
    let stellar = find(&plan.games, "Stellar Blade")?;
    let requests = [ImportGameRequest { title: "Stellar Blade".into(), snapshots: stellar.snapshots.clone() }];

    let mut events = Vec::new();
    let report = import_games(&db, snapshots_root.path().to_path_buf(), &requests, SnapshotStorage::Local, &mut |p| {
        events.push(p.done);
    });
    assert_eq!(events, [0, 1]);
    let imported = &report.games[0];
    assert!(imported.errors.is_empty(), "{:?}", imported.errors);
    assert_eq!((imported.created, imported.imported, imported.duplicates), (true, 3, 0));

    let game_id = imported.game_id.ok_or("no game id")?;
    let game = LibraryRepository::new(&db).summary(game_id)?.ok_or("game missing")?;
    assert!(game.archived && !game.installed);

    let repository = SnapshotRepository::new(&db);
    let snapshots = repository.list(Some(game_id))?;
    assert_eq!(snapshots.len(), 3);
    assert!(snapshots.iter().all(|snapshot| snapshot.imported));
    let mut labels: Vec<_> = snapshots.iter().filter_map(|snapshot| snapshot.label.clone()).collect();
    labels.sort();
    assert_eq!(labels, ["after ending A", "after ending B", "before final choice"]);

    // Every imported archive passes the same verification as a regular snapshot.
    let engine = SnapshotEngine::new(&db, snapshots_root.path().to_path_buf());
    for snapshot in &snapshots {
        engine.verify(&snapshot.id)?;
    }
    // Where it belongs is unknown until the user chooses: restoring explains that.
    let refused = engine.restore(&snapshots[0].id);
    assert!(refused.is_err());

    // Importing the same folder again adds nothing and creates no second game.
    let again = import_games(&db, snapshots_root.path().to_path_buf(), &requests, SnapshotStorage::Local, &mut |_| {});
    assert_eq!((again.games[0].created, again.games[0].imported, again.games[0].duplicates), (false, 0, 3));
    assert_eq!(repository.list(Some(game_id))?.len(), 3);
    Ok(())
}

#[test]
fn the_source_folder_is_never_modified() -> TestResult {
    let dir = sample_backup_folder()?;
    let snapshots_root = tempfile::tempdir()?;
    let db = Database::open_in_memory()?;
    let plan = scan_folder(dir.path(), ScanMode::Auto)?;
    let requests: Vec<_> = plan
        .games
        .iter()
        .map(|game| ImportGameRequest { title: game.guessed_title.clone(), snapshots: game.snapshots.clone() })
        .collect();
    let before = count_files(dir.path());
    let report = import_games(&db, snapshots_root.path().to_path_buf(), &requests, SnapshotStorage::Local, &mut |_| {});
    assert!(report.games.iter().all(|game| game.errors.is_empty()), "{report:?}");
    assert_eq!(count_files(dir.path()), before);
    Ok(())
}

fn count_files(root: &Path) -> usize {
    walkdir::WalkDir::new(root).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_file()).count()
}

/// Prints how a real backup folder is read. Run with
/// `L8_IMPORT_SAMPLE=C:\path\to\SaveGames cargo test -p l8-core real_backup_folder -- --ignored --nocapture`.
#[test]
#[ignore = "needs a real backup folder"]
fn real_backup_folder() -> TestResult {
    let Ok(path) = std::env::var("L8_IMPORT_SAMPLE") else { return Ok(()) };
    let plan = scan_folder(Path::new(&path), ScanMode::Auto)?;
    println!("mode {:?}, {} games, warnings {:?}", plan.mode, plan.games.len(), plan.warnings);
    for game in &plan.games {
        let labels: Vec<_> = game.snapshots.iter().map(|s| s.label.clone().unwrap_or_else(|| "-".into())).collect();
        println!(
            "{:<45} -> {:<40} {:?} {} snap {:?} skipped {} dup {:?} alt {:?}",
            game.folder_name, game.guessed_title, game.layout, game.snapshots.len(), labels, game.skipped_files,
            game.duplicate_of, game.alt_titles
        );
    }
    Ok(())
}

/// Imports the "Stellar Blade" variants into a fresh database and returns what the tests need.
struct Imported {
    _source: tempfile::TempDir,
    snapshots_root: tempfile::TempDir,
    db: Database,
    game_id: i64,
    snapshot_ids: Vec<String>,
}

fn import_stellar_blade() -> Result<Imported, Box<dyn std::error::Error>> {
    let source = sample_backup_folder()?;
    let snapshots_root = tempfile::tempdir()?;
    let db = Database::open_in_memory()?;
    let plan = scan_folder(source.path(), ScanMode::Auto)?;
    let stellar = find(&plan.games, "Stellar Blade")?;
    let requests = [ImportGameRequest { title: "Stellar Blade".into(), snapshots: stellar.snapshots.clone() }];
    let report = import_games(&db, snapshots_root.path().to_path_buf(), &requests, SnapshotStorage::Local, &mut |_| {});
    let game = report.games.first().ok_or("no game")?;
    Ok(Imported {
        game_id: game.game_id.ok_or("no id")?,
        snapshot_ids: game.snapshot_ids.clone(),
        _source: source,
        snapshots_root,
        db,
    })
}

#[test]
fn an_imported_snapshot_is_restored_over_a_chosen_folder_without_losing_anything() -> TestResult {
    let imported = import_stellar_blade()?;
    let engine = SnapshotEngine::new(&imported.db, imported.snapshots_root.path().to_path_buf());
    let repository = SnapshotRepository::new(&imported.db);
    let snapshot = repository
        .list(Some(imported.game_id))?
        .into_iter()
        .find(|snapshot| snapshot.label.as_deref() == Some("after ending B"))
        .ok_or("missing snapshot")?;

    // The game was reinstalled and already has a save, plus an unrelated file that must survive.
    let target_parent = tempfile::tempdir()?;
    let target = target_parent.path().join("Saved");
    write(&target, "SaveGames/76561199774717161/slot1.sav", "current save")?;
    write(&target, "SaveGames/notes.txt", "mine")?;

    let report = engine.restore_imported(&snapshot.id, &target)?;
    assert_eq!(report.files_restored, 3);
    assert!(report.safety_snapshot_id.is_some(), "the overwritten save is kept");
    assert_eq!(fs::read_to_string(target.join("SaveGames/76561199774717161/slot1.sav"))?, "b");
    assert_eq!(fs::read_to_string(target.join("SaveGames/notes.txt"))?, "mine");
    let leftovers = fs::read_dir(target_parent.path())?.filter_map(Result::ok).count();
    assert_eq!(leftovers, 1, "no temporary folder is left next to the save");

    // The folder became the game's save location, so the safety copy restores like any other snapshot.
    let safety_id = report.safety_snapshot_id.ok_or("no safety copy")?;
    engine.restore(&safety_id)?;
    assert_eq!(fs::read_to_string(target.join("SaveGames/76561199774717161/slot1.sav"))?, "current save");
    Ok(())
}

#[test]
fn restore_targets_come_from_the_game_save_folders() -> TestResult {
    use crate::saves::{DerivedLocation, LocationKind, SaveLocationRepository};

    let imported = import_stellar_blade()?;
    let engine = SnapshotEngine::new(&imported.db, imported.snapshots_root.path().to_path_buf());
    let id = imported.snapshot_ids.first().ok_or("no snapshot")?;
    assert!(engine.restore_targets(id)?.is_empty(), "nothing is known before the game is detected");

    // The files start with `SaveGames/`, so they belong in the parent of a folder called SaveGames.
    let root = tempfile::tempdir()?;
    let save_games = root.path().join("SB").join("Saved").join("SaveGames");
    fs::create_dir_all(&save_games)?;
    SaveLocationRepository::new(&imported.db).replace_detected(
        imported.game_id,
        "ludusavi",
        &[DerivedLocation { path: save_games, kind: LocationKind::Directory }],
    )?;
    let targets = engine.restore_targets(id)?;
    assert_eq!(targets.len(), 1);
    assert!(targets[0].path.ends_with("Saved"), "{}", targets[0].path);
    Ok(())
}

#[test]
fn a_regular_restore_still_refuses_an_imported_snapshot_with_a_clear_reason() -> TestResult {
    let imported = import_stellar_blade()?;
    let engine = SnapshotEngine::new(&imported.db, imported.snapshots_root.path().to_path_buf());
    let id = imported.snapshot_ids.first().ok_or("no snapshot")?;
    let error = engine.restore(id).err().ok_or("expected a refusal")?;
    assert!(error.to_string().contains("choose where to restore"), "{error}");
    Ok(())
}

#[test]
fn a_saves_only_game_is_joined_to_the_installed_game_it_turns_out_to_be() -> TestResult {
    use crate::platforms::{DiscoveredGame, Discovery, Platform};

    let imported = import_stellar_blade()?;
    let library = LibraryRepository::new(&imported.db);
    assert!(library.link_candidates(imported.game_id)?.is_empty(), "nothing to join to yet");

    // The game is installed later and found by a normal library scan.
    library.merge_discovery(
        Platform::Steam,
        &Discovery {
            games: vec![DiscoveredGame {
                platform_game_id: "3489700".into(),
                title: "STELLAR BLADE".into(),
                install_path: std::path::PathBuf::from(r"D:\Steam\Stellar Blade"),
                executable_path: None,
                size_bytes: None,
                last_played_at: None,
                playtime_seconds: None,
            }],
            ..Discovery::default()
        },
    )?;
    let candidates = library.link_candidates(imported.game_id)?;
    assert_eq!(candidates.len(), 1);
    let installed = candidates[0].clone();
    assert!(installed.installed && !installed.archived);

    let merged = library.merge_archived_into(imported.game_id, installed.id)?;
    assert_eq!(merged.id, installed.id);
    assert!(library.summary(imported.game_id)?.is_none(), "the saves-only row is gone");

    let snapshots = SnapshotRepository::new(&imported.db).list(Some(installed.id))?;
    assert_eq!(snapshots.len(), 3);
    assert!(snapshots.iter().all(|snapshot| snapshot.imported && snapshot.game_title == installed.title));
    // Archives did not move: they still verify.
    let engine = SnapshotEngine::new(&imported.db, imported.snapshots_root.path().to_path_buf());
    for snapshot in &snapshots {
        engine.verify(&snapshot.id)?;
    }
    // A game that is not saves-only cannot be joined to anything.
    assert!(library.merge_archived_into(installed.id, installed.id).is_err());
    Ok(())
}

/// Imports a real backup folder into a throwaway database and snapshot folder, then verifies every
/// archive and checks the source was left untouched. Run with
/// `L8_IMPORT_SAMPLE=C:\path\to\SaveGames cargo test -p l8-core real_backup_import -- --ignored --nocapture`.
#[test]
#[ignore = "needs a real backup folder"]
fn real_backup_import() -> TestResult {
    let Ok(path) = std::env::var("L8_IMPORT_SAMPLE") else { return Ok(()) };
    let source = Path::new(&path);
    let before = count_files(source);
    let plan = scan_folder(source, ScanMode::Auto)?;
    let requests: Vec<_> = plan
        .games
        .iter()
        .map(|game| ImportGameRequest {
            title: game.suggested_title.clone().unwrap_or_else(|| game.guessed_title.clone()),
            snapshots: game.snapshots.clone(),
        })
        .collect();
    let snapshots_root = tempfile::tempdir()?;
    let db = Database::open_in_memory()?;
    let started = std::time::Instant::now();
    let report = import_games(&db, snapshots_root.path().to_path_buf(), &requests, SnapshotStorage::Local, &mut |_| {});
    let games = report.games.len();
    let created = report.games.iter().filter(|game| game.created).count();
    let imported: u32 = report.games.iter().map(|game| game.imported).sum();
    let duplicates: u32 = report.games.iter().map(|game| game.duplicates).sum();
    let errors: Vec<_> = report.games.iter().flat_map(|game| game.errors.clone()).collect();
    println!("{games} rows -> {created} games, {imported} snapshots, {duplicates} duplicates, {} errors in {:?}", errors.len(), started.elapsed());
    for error in &errors {
        println!("  error: {error}");
    }
    let engine = SnapshotEngine::new(&db, snapshots_root.path().to_path_buf());
    let all = SnapshotRepository::new(&db).list(None)?;
    for snapshot in &all {
        engine.verify(&snapshot.id)?;
    }
    println!("{} snapshots verified; source files before {before}, after {}", all.len(), count_files(source));
    assert_eq!(count_files(source), before);
    assert!(errors.is_empty());
    Ok(())
}
