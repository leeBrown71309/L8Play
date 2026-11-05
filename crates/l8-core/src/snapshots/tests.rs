//! Integration tests of the snapshot engine on real temporary folders.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::library::LibraryRepository;
use crate::platforms::{DiscoveredGame, Discovery, Platform};
use crate::saves::{DerivedLocation, LocationKind, SaveLocationRepository};
use crate::snapshots::{
    CreateOutcome, CreateRequest, RetentionPolicy, SnapshotEngine, SnapshotKind, SnapshotRepository, SnapshotStatus,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Fixture {
    _dir: tempfile::TempDir,
    db: Database,
    game_id: i64,
    saves: PathBuf,
    snapshots: PathBuf,
}

impl Fixture {
    /// One game whose save folder is `<tmp>/saves` with two files in it.
    fn new() -> CoreResult<Self> {
        let dir = tempfile::tempdir()?;
        let saves = dir.path().join("saves");
        fs::create_dir_all(saves.join("slot"))?;
        fs::write(saves.join("slot").join("data.sav"), b"chapter 1")?;
        fs::write(saves.join("settings.ini"), b"volume=10")?;
        let db = Database::open_in_memory()?;
        let library = LibraryRepository::new(&db);
        library.merge_discovery(
            Platform::Steam,
            &Discovery {
                games: vec![DiscoveredGame {
                    platform_game_id: "814380".into(),
                    title: "Sekiro".into(),
                    install_path: dir.path().join("install"),
                    executable_path: None,
                    size_bytes: None,
                    last_played_at: None,
                    playtime_seconds: None,
                }],
                ..Discovery::default()
            },
        )?;
        let game_id = library.list()?[0].id;
        SaveLocationRepository::new(&db).replace_detected(
            game_id,
            "ludusavi",
            &[DerivedLocation { path: saves.clone(), kind: LocationKind::Directory }],
        )?;
        let snapshots = dir.path().join("snapshots");
        Ok(Self { _dir: dir, db, game_id, saves, snapshots })
    }

    fn engine(&self) -> SnapshotEngine<'_> {
        SnapshotEngine::new(&self.db, self.snapshots.clone())
    }

    fn manual(&self, name: &str) -> CreateRequest {
        CreateRequest {
            game_id: self.game_id,
            name: name.into(),
            description: None,
            kind: SnapshotKind::Manual,
            pinned: false,
            skip_if_unchanged: false,
            storage: Default::default(),
        }
    }

    fn create(&self, request: &CreateRequest) -> CoreResult<String> {
        match self.engine().create(request)? {
            CreateOutcome::Created(summary) => Ok(summary.id),
            other => Err(CoreError::Validation(format!("expected a snapshot, got {other:?}"))),
        }
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn create_writes_a_verified_archive_without_touching_saves() -> TestResult {
    let fx = Fixture::new()?;
    let before = fs::metadata(fx.saves.join("slot").join("data.sav"))?.modified()?;
    let id = fx.create(&fx.manual("Before Isshin"))?;
    let snapshot = SnapshotRepository::new(&fx.db).get(&id)?;
    assert_eq!(snapshot.status, SnapshotStatus::Ready);
    assert_eq!(snapshot.total_files, 2);
    assert_eq!(snapshot.name, "Before Isshin");
    let archive = PathBuf::from(snapshot.archive_path.as_deref().unwrap_or_default());
    assert!(archive.is_file());
    assert!(archive.starts_with(&fx.snapshots));
    assert_eq!(fs::metadata(fx.saves.join("slot").join("data.sav"))?.modified()?, before, "originals are read-only");
    let leftovers: Vec<_> = fs::read_dir(archive.parent().ok_or("no parent")?)?
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".partial"))
        .collect();
    assert!(leftovers.is_empty());
    assert_eq!(fx.engine().verify(&id)?.status, SnapshotStatus::Ready);
    Ok(())
}

#[test]
fn unchanged_saves_are_not_duplicated() -> TestResult {
    let fx = Fixture::new()?;
    fx.create(&fx.manual("first"))?;
    let auto = CreateRequest { kind: SnapshotKind::AutoExit, skip_if_unchanged: true, ..fx.manual("") };
    assert!(matches!(fx.engine().create(&auto)?, CreateOutcome::Unchanged { .. }));
    fs::write(fx.saves.join("slot").join("data.sav"), b"chapter 2")?;
    assert!(matches!(fx.engine().create(&auto)?, CreateOutcome::Created(_)));
    assert!(fx.engine().current_state(fx.game_id)?.matches_latest);
    Ok(())
}

#[test]
fn restore_round_trip_with_safety_copy() -> TestResult {
    let fx = Fixture::new()?;
    let id = fx.create(&fx.manual("chapter 1"))?;

    fs::write(fx.saves.join("slot").join("data.sav"), b"chapter 5")?;
    fs::write(fx.saves.join("slot").join("new.sav"), b"created later")?;

    let report = fx.engine().restore(&id)?;
    assert_eq!(read(&fx.saves.join("slot").join("data.sav")), "chapter 1");
    assert_eq!(read(&fx.saves.join("settings.ini")), "volume=10");
    assert!(!fx.saves.join("slot").join("new.sav").exists(), "the folder is exactly the snapshot");
    assert_eq!(report.files_restored, 2);

    // The replaced save is recoverable from the safety snapshot.
    let safety_id = report.safety_snapshot_id.ok_or("no safety snapshot")?;
    let safety = SnapshotRepository::new(&fx.db).get(&safety_id)?;
    assert_eq!(safety.kind, SnapshotKind::Safety);
    fx.engine().restore(&safety_id)?;
    assert_eq!(read(&fx.saves.join("slot").join("data.sav")), "chapter 5");
    assert_eq!(read(&fx.saves.join("slot").join("new.sav")), "created later");

    let leftovers: Vec<_> = fs::read_dir(fx.saves.parent().ok_or("no parent")?)?
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(".l8-"))
        .collect();
    assert!(leftovers.is_empty(), "staging and previous copies are cleaned up");
    Ok(())
}

#[test]
fn corrupted_archives_are_refused_and_saves_untouched() -> TestResult {
    let fx = Fixture::new()?;
    let id = fx.create(&fx.manual("good"))?;
    let archive = PathBuf::from(SnapshotRepository::new(&fx.db).get(&id)?.archive_path.unwrap_or_default());
    let mut bytes = fs::read(&archive)?;
    let middle = bytes.len() / 2;
    if let Some(byte) = bytes.get_mut(middle) {
        *byte ^= 0xFF;
    }
    fs::write(&archive, bytes)?;
    fs::write(fx.saves.join("slot").join("data.sav"), b"current progress")?;

    assert!(matches!(fx.engine().restore(&id), Err(CoreError::Integrity(_))));
    assert_eq!(read(&fx.saves.join("slot").join("data.sav")), "current progress");
    assert_eq!(SnapshotRepository::new(&fx.db).get(&id)?.status, SnapshotStatus::Corrupt);
    Ok(())
}

#[test]
fn restore_only_writes_to_registered_locations() -> TestResult {
    let fx = Fixture::new()?;
    let id = fx.create(&fx.manual("x"))?;
    let locations = SaveLocationRepository::new(&fx.db);
    let location = locations.list(fx.game_id)?.into_iter().next().ok_or("no location")?;
    locations.reject(fx.game_id, location.id)?;
    assert!(matches!(fx.engine().restore(&id), Err(CoreError::Validation(_))));
    Ok(())
}

#[test]
fn failed_restore_rolls_back_every_location() -> TestResult {
    let fx = Fixture::new()?;
    // Second location whose parent is a file: its swap must fail after the first one succeeded.
    let blocker = fx.saves.parent().ok_or("no parent")?.join("blocker");
    fs::write(&blocker, b"i am a file")?;
    let impossible = blocker.join("saves");
    let locations = SaveLocationRepository::new(&fx.db);
    locations.replace_detected(
        fx.game_id,
        "ludusavi",
        &[
            DerivedLocation { path: fx.saves.clone(), kind: LocationKind::Directory },
            DerivedLocation { path: impossible, kind: LocationKind::Directory },
        ],
    )?;
    let id = fx.create(&fx.manual("two locations"))?;
    fs::write(fx.saves.join("slot").join("data.sav"), b"newer progress")?;

    assert!(fx.engine().restore(&id).is_err());
    assert_eq!(read(&fx.saves.join("slot").join("data.sav")), "newer progress", "first location rolled back");
    Ok(())
}

#[test]
fn single_file_locations_restore() -> TestResult {
    let fx = Fixture::new()?;
    let config = fx.saves.parent().ok_or("no parent")?.join("config.ini");
    fs::write(&config, b"quality=high")?;
    SaveLocationRepository::new(&fx.db).replace_detected(
        fx.game_id,
        "ludusavi",
        &[DerivedLocation { path: config.clone(), kind: LocationKind::File }],
    )?;
    let id = fx.create(&fx.manual("config"))?;
    fs::write(&config, b"quality=low")?;
    fx.engine().restore(&id)?;
    assert_eq!(read(&config), "quality=high");
    Ok(())
}

#[test]
fn zip_slip_entries_never_reach_the_disk() -> TestResult {
    let fx = Fixture::new()?;
    let id = fx.create(&fx.manual("legit"))?;
    let repository = SnapshotRepository::new(&fx.db);
    let archive = PathBuf::from(repository.get(&id)?.archive_path.unwrap_or_default());

    // Rewrite the archive with a manifest pointing outside the save folder.
    let manifest = format!(
        r#"{{"formatVersion":1,"snapshotId":"{id}","gameTitle":"Sekiro","createdAt":"2026-11-04T21:00:00Z",
            "locations":[{{"index":0,"path":"{}","kind":"directory"}}],
            "files":[{{"entry":"files/0/evil","location":0,"relativePath":"../evil.txt","originalPath":"x","size":4,"sha256":"{}","modifiedAt":null}}]}}"#,
        fx.saves.to_string_lossy().replace('\\', "\\\\"),
        "88d4266fd4e6338d13b845fcf289579d209c897823b9217da3e161936f031589"
    );
    {
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive)?);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("files/0/evil", options)?;
        zip.write_all(b"abcd")?;
        zip.start_file("l8-snapshot.json", options)?;
        zip.write_all(manifest.as_bytes())?;
        zip.finish()?;
    }
    let new_hash = crate::snapshots::files::sha256_file(&archive)?;
    fx.db
        .connection()?
        .execute("UPDATE snapshots SET archive_sha256 = ?2 WHERE id = ?1", rusqlite::params![id, new_hash])?;

    assert!(matches!(fx.engine().restore(&id), Err(CoreError::Integrity(_))));
    assert!(!fx.saves.parent().ok_or("no parent")?.join("evil.txt").exists());
    Ok(())
}

#[cfg(windows)]
#[test]
fn files_in_use_block_the_snapshot_cleanly() -> TestResult {
    let fx = Fixture::new()?;
    let _game_is_writing = fs::OpenOptions::new().write(true).open(fx.saves.join("slot").join("data.sav"))?;
    assert!(matches!(fx.engine().create(&fx.manual("busy")), Err(CoreError::Busy(_))));
    let game_folder = fx.snapshots.join("sekiro");
    let leftovers = fs::read_dir(&game_folder).map(|entries| entries.count()).unwrap_or(0);
    assert_eq!(leftovers, 0, "no partial archive left behind");
    Ok(())
}

#[test]
fn empty_locations_have_nothing_to_back_up() -> TestResult {
    let fx = Fixture::new()?;
    fs::remove_dir_all(&fx.saves)?;
    assert_eq!(fx.engine().create(&fx.manual("none"))?, CreateOutcome::NothingToBackUp);
    Ok(())
}

#[test]
fn retention_only_removes_unpinned_automatic_snapshots() -> TestResult {
    let fx = Fixture::new()?;
    let manual = fx.create(&fx.manual("keep me"))?;
    let mut automatic = Vec::new();
    for round in 0..4 {
        fs::write(fx.saves.join("slot").join("data.sav"), format!("round {round}"))?;
        automatic.push(fx.create(&CreateRequest { kind: SnapshotKind::AutoExit, ..fx.manual("") })?);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let repository = SnapshotRepository::new(&fx.db);
    repository.set_pinned(automatic.first().ok_or("none")?, true)?;

    let report = fx.engine().apply_retention(fx.game_id, RetentionPolicy { keep_local: 2, keep_cloud: 0 })?;
    assert_eq!(report.removed_local, 1, "4 automatic, 1 pinned, keep 2 newest → 1 removed");
    assert_eq!(report.removed_cloud, 0, "nothing was in Drive");
    let remaining: Vec<String> = repository.list(Some(fx.game_id))?.into_iter().map(|snapshot| snapshot.id).collect();
    assert!(remaining.contains(&manual));
    assert!(remaining.contains(automatic.first().ok_or("none")?));
    assert_eq!(remaining.len(), 4);
    Ok(())
}

#[test]
fn delete_relinks_lineage_and_removes_the_archive() -> TestResult {
    let fx = Fixture::new()?;
    let first = fx.create(&fx.manual("1"))?;
    fs::write(fx.saves.join("slot").join("data.sav"), b"2")?;
    let second = fx.create(&fx.manual("2"))?;
    fs::write(fx.saves.join("slot").join("data.sav"), b"3")?;
    let third = fx.create(&fx.manual("3"))?;
    let repository = SnapshotRepository::new(&fx.db);
    let archive = PathBuf::from(repository.get(&second)?.archive_path.unwrap_or_default());

    fx.engine().delete(&second)?;
    assert!(!archive.exists());
    assert!(matches!(repository.get(&second), Err(CoreError::NotFound(_))));
    let parent: Option<String> =
        fx.db
            .connection()?
            .query_row("SELECT parent_snapshot_id FROM snapshots WHERE id = ?1", [&third], |row| row.get(0))?;
    assert_eq!(parent.as_deref(), Some(first.as_str()));
    Ok(())
}
