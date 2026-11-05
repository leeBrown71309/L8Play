//! Sync-engine tests: two simulated PCs share an in-memory "Drive".

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::cloud::drive::UploadSession;
use crate::cloud::model::{AccountStatus, ConflictResolution, SyncStatus};
use crate::cloud::remote::{RemoteMeta, RemoteObject, RemoteQuota, RemoteSnapshot, RemoteStore, UploadRequest};
use crate::cloud::repository::CloudRepository;
use crate::cloud::sync::CloudSync;
use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::library::LibraryRepository;
use crate::platforms::{DiscoveredGame, Discovery, Platform};
use crate::saves::{DerivedLocation, LocationKind, SaveLocationRepository};
use crate::snapshots::{
    CloudState, CreateOutcome, CreateRequest, RetentionPolicy, RetentionReport, SnapshotEngine, SnapshotKind,
    SnapshotRepository, SnapshotStatus, SnapshotStorage,
};

#[derive(Clone, Copy)]
enum Failure {
    Network,
    Auth,
}

#[derive(Default)]
struct DriveState {
    files: HashMap<String, (RemoteMeta, Vec<u8>)>,
    upload_failures: Vec<Failure>,
    delete_failures: Vec<Failure>,
    quota_limit: Option<i64>,
    uploads_attempted: u32,
    next_id: u32,
}

/// A fake Drive: stores archives and their metadata in memory.
#[derive(Default)]
struct FakeDrive {
    state: Mutex<DriveState>,
}

impl FakeDrive {
    fn lock(&self) -> CoreResult<std::sync::MutexGuard<'_, DriveState>> {
        self.state.lock().map_err(|_| CoreError::LockPoisoned("fake drive"))
    }
}

impl RemoteStore for FakeDrive {
    fn quota(&self) -> CoreResult<RemoteQuota> {
        let state = self.lock()?;
        let used = state.files.values().map(|(_, bytes)| i64::try_from(bytes.len()).unwrap_or(0)).sum();
        Ok(RemoteQuota { email: "ryu@example.com".into(), used, limit: state.quota_limit })
    }

    fn list_snapshots(&self) -> CoreResult<Vec<RemoteSnapshot>> {
        Ok(self
            .lock()?
            .files
            .iter()
            .map(|(id, (meta, bytes))| RemoteSnapshot {
                remote_id: id.clone(),
                size: i64::try_from(bytes.len()).unwrap_or(0),
                meta: meta.clone(),
            })
            .collect())
    }

    fn upload(
        &self,
        request: &UploadRequest<'_>,
        session: &mut UploadSession,
        on_progress: &mut dyn FnMut(&UploadSession),
    ) -> CoreResult<RemoteObject> {
        let mut state = self.lock()?;
        state.uploads_attempted += 1;
        if !state.upload_failures.is_empty() {
            return match state.upload_failures.remove(0) {
                Failure::Network => Err(CoreError::External("network error: connection reset".into())),
                Failure::Auth => Err(CoreError::Auth("Drive refused the access token".into())),
            };
        }
        session.resumable_uri = Some("https://www.googleapis.com/upload/fake".into());
        session.bytes_done = 1;
        on_progress(session);
        let bytes = fs::read(request.local_path)?;
        state.next_id += 1;
        let remote_id = format!("remote-{}", state.next_id);
        let size = i64::try_from(bytes.len()).unwrap_or(0);
        state.files.insert(remote_id.clone(), (request.meta.clone(), bytes));
        Ok(RemoteObject {
            remote_id,
            size,
            path: format!("L8Play/{}/{}.zip", request.meta.game_slug, request.meta.snapshot_id),
        })
    }

    fn delete_snapshot(&self, remote_id: &str, snapshot_id: &str) -> CoreResult<bool> {
        let mut state = self.lock()?;
        if !state.delete_failures.is_empty() {
            return match state.delete_failures.remove(0) {
                Failure::Network => Err(CoreError::External("network error: connection reset".into())),
                Failure::Auth => Err(CoreError::Auth("Drive refused the access token".into())),
            };
        }
        match state.files.get(remote_id) {
            None => Ok(false),
            Some((meta, _)) if meta.snapshot_id != snapshot_id => {
                Err(CoreError::Validation("the Drive file is not that snapshot".into()))
            }
            Some(_) => Ok(state.files.remove(remote_id).is_some()),
        }
    }

    fn download(&self, remote_id: &str, dest: &Path) -> CoreResult<()> {
        let state = self.lock()?;
        let (_, bytes) = state.files.get(remote_id).ok_or_else(|| CoreError::NotFound("remote file".into()))?;
        fs::write(dest, bytes)?;
        Ok(())
    }
}

/// One simulated PC: its own database, snapshot folder, and a save folder.
struct Pc {
    db: Database,
    game_id: i64,
    saves: PathBuf,
    snapshots: PathBuf,
}

impl Pc {
    /// `saves` may be shared between PCs so a restore on one lands somewhere real.
    fn new(base: &Path, name: &str, saves: &Path, with_game: bool) -> CoreResult<Self> {
        let db = Database::open_in_memory()?;
        let library = LibraryRepository::new(&db);
        let title = if with_game { "Sekiro" } else { "Other" };
        library.merge_discovery(
            Platform::Steam,
            &Discovery {
                games: vec![DiscoveredGame {
                    platform_game_id: "814380".into(),
                    title: title.into(),
                    install_path: base.join(name).join("install"),
                    executable_path: None,
                    size_bytes: None,
                    last_played_at: None,
                    playtime_seconds: None,
                }],
                ..Discovery::default()
            },
        )?;
        let game_id = library.list()?[0].id;
        fs::create_dir_all(saves)?;
        SaveLocationRepository::new(&db).replace_detected(
            game_id,
            "ludusavi",
            &[DerivedLocation { path: saves.to_path_buf(), kind: LocationKind::Directory }],
        )?;
        let snapshots = base.join(name).join("snapshots");
        Ok(Self { db, game_id, saves: saves.to_path_buf(), snapshots })
    }

    fn engine(&self) -> SnapshotEngine<'_> {
        SnapshotEngine::new(&self.db, self.snapshots.clone())
    }

    fn sync<'a>(&'a self, drive: &'a FakeDrive) -> CloudSync<'a> {
        CloudSync::new(&self.db, drive, self.snapshots.clone())
    }

    fn connect(&self) -> CoreResult<()> {
        CloudRepository::new(&self.db).connect("ryu@example.com", "root").map(|_| ())
    }

    fn snapshot(&self, content: &str, kind: SnapshotKind) -> CoreResult<String> {
        self.snapshot_stored(content, kind, SnapshotStorage::Both)
    }

    fn snapshot_stored(&self, content: &str, kind: SnapshotKind, storage: SnapshotStorage) -> CoreResult<String> {
        fs::write(self.saves.join("slot.sav"), content)?;
        match self.engine().create(&CreateRequest {
            game_id: self.game_id,
            name: content.into(),
            description: None,
            kind,
            pinned: false,
            skip_if_unchanged: false,
            storage,
        })? {
            CreateOutcome::Created(summary) => Ok(summary.id),
            other => Err(CoreError::Validation(format!("unexpected outcome {other:?}"))),
        }
    }

    fn cloud_state(&self, id: &str) -> CoreResult<CloudState> {
        Ok(SnapshotRepository::new(&self.db).get(id)?.cloud)
    }
}

fn make_due_now(pc: &Pc) -> CoreResult<()> {
    pc.db.connection()?.execute("UPDATE sync_operations SET next_attempt_at = '2000-01-01T00:00:00Z'", [])?;
    Ok(())
}

#[test]
fn a_snapshot_is_uploaded_once_with_its_metadata() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let id = pc.snapshot("chapter 1", SnapshotKind::Manual)?;

    let sync = pc.sync(&drive);
    assert!(sync.queue_upload(&id)?);
    assert!(!sync.queue_upload(&id)?, "already queued");
    assert_eq!(pc.cloud_state(&id)?, CloudState::Queued);
    let report = sync.run_queue()?;
    assert_eq!((report.uploaded, report.failed), (1, 0));
    assert_eq!(pc.cloud_state(&id)?, CloudState::Synced);
    assert!(!sync.queue_upload(&id)?, "already in Drive");

    let remote = drive.list_snapshots()?;
    assert_eq!(remote.len(), 1);
    assert_eq!(remote[0].meta.game_slug, "sekiro");
    assert_eq!(remote[0].meta.snapshot_id, id);
    Ok(())
}

#[test]
fn safety_copies_and_conflict_copies_never_leave_the_pc() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let safety = pc.snapshot("before restore", SnapshotKind::Safety)?;
    let manual = pc.snapshot("keep", SnapshotKind::Manual)?;
    let sync = pc.sync(&drive);
    assert!(!sync.queue_upload(&safety)?);
    assert_eq!(sync.queue_missing_uploads()?, 1);
    sync.run_queue()?;
    assert_eq!(pc.cloud_state(&manual)?, CloudState::Synced);
    assert_eq!(pc.cloud_state(&safety)?, CloudState::None);
    Ok(())
}

#[test]
fn failures_are_retried_with_backoff_and_then_succeed() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    drive.lock()?.upload_failures.push(Failure::Network);
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let id = pc.snapshot("chapter 1", SnapshotKind::AutoExit)?;
    let sync = pc.sync(&drive);
    sync.queue_upload(&id)?;

    let first = sync.run_queue()?;
    assert_eq!((first.uploaded, first.failed), (0, 1));
    assert_eq!(pc.cloud_state(&id)?, CloudState::Failed);
    let waiting = sync.run_queue()?;
    assert_eq!((waiting.uploaded, waiting.failed), (0, 0), "the retry time has not come yet");
    assert_eq!(drive.lock()?.uploads_attempted, 1);

    make_due_now(&pc)?;
    assert_eq!(sync.run_queue()?.uploaded, 1);
    assert_eq!(pc.cloud_state(&id)?, CloudState::Synced);
    Ok(())
}

#[test]
fn a_refused_sign_in_pauses_the_queue_until_the_user_reconnects() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    drive.lock()?.upload_failures.push(Failure::Auth);
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let id = pc.snapshot("chapter 1", SnapshotKind::Manual)?;
    let sync = pc.sync(&drive);
    sync.queue_upload(&id)?;
    assert!(matches!(sync.run_queue(), Err(CoreError::Auth(_))));

    let repository = CloudRepository::new(&pc.db);
    let account = repository.account()?.ok_or(CoreError::NotFound("account".into()))?;
    assert_eq!(account.status, AccountStatus::NeedsReauth);
    assert!(matches!(sync.run_queue(), Err(CoreError::Auth(_))), "nothing runs while sign-in is needed");
    assert_eq!(repository.pending(account.id)?[0].status, SyncStatus::Paused);

    repository.set_status(account.id, AccountStatus::Connected)?;
    assert_eq!(sync.run_queue()?.uploaded, 1, "the paused upload resumes after signing in again");
    Ok(())
}

#[test]
fn a_full_drive_and_a_damaged_archive_are_not_retried_automatically() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let full = pc.snapshot("big", SnapshotKind::Manual)?;
    drive.lock()?.quota_limit = Some(1);
    let sync = pc.sync(&drive);
    sync.queue_upload(&full)?;
    assert_eq!(sync.run_queue()?.failed, 1);
    make_due_now(&pc)?;
    assert_eq!(sync.run_queue()?.failed, 0, "a permanent failure waits for the user's Retry");
    assert_eq!(drive.lock()?.uploads_attempted, 0, "nothing was sent to a full Drive");

    drive.lock()?.quota_limit = None;
    let account = CloudRepository::new(&pc.db).account()?.ok_or(CoreError::NotFound("account".into()))?;
    assert_eq!(CloudRepository::new(&pc.db).retry_failed(account.id)?, 1);
    assert_eq!(sync.run_queue()?.uploaded, 1);

    let damaged = pc.snapshot("will be damaged", SnapshotKind::Manual)?;
    let path = SnapshotRepository::new(&pc.db).get(&damaged)?.archive_path.ok_or(CoreError::NotFound("path".into()))?;
    let mut bytes = fs::read(&path)?;
    bytes.push(0);
    fs::write(&path, bytes)?;
    sync.queue_upload(&damaged)?;
    let report = sync.run_queue()?;
    assert_eq!(report.failed, 1);
    assert_eq!(drive.lock()?.files.len(), 1, "the damaged archive was never uploaded");
    Ok(())
}

#[test]
fn a_new_pc_rebuilds_its_history_from_drive_and_downloads_with_verification() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let a = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    a.connect()?;
    let first = a.snapshot("chapter 1", SnapshotKind::Manual)?;
    let second = a.snapshot("chapter 2", SnapshotKind::Manual)?;
    a.sync(&drive).sync_all()?;

    let b = Pc::new(dir.path(), "b", &dir.path().join("saves-b"), true)?;
    b.connect()?;
    let report = b.sync(&drive).sync_all()?;
    assert_eq!(report.imported, 2);
    assert_eq!(report.new_conflicts, 0, "an imported history alone is never a conflict");

    let imported = SnapshotRepository::new(&b.db).get(&second)?;
    assert_eq!(imported.status, SnapshotStatus::MissingLocal);
    assert_eq!(imported.cloud, CloudState::Synced);
    assert_eq!(imported.device_name, SnapshotRepository::new(&a.db).get(&second)?.device_name);

    b.sync(&drive).download_now(&second)?;
    let downloaded = SnapshotRepository::new(&b.db).get(&second)?;
    assert_eq!(downloaded.status, SnapshotStatus::Ready);
    b.engine().verify(&second)?;
    assert_eq!(SnapshotRepository::new(&b.db).get(&first)?.status, SnapshotStatus::MissingLocal);
    Ok(())
}

#[test]
fn a_tampered_download_is_rejected_and_removed() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let a = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    a.connect()?;
    let id = a.snapshot("chapter 1", SnapshotKind::Manual)?;
    a.sync(&drive).sync_all()?;
    for (_, bytes) in drive.lock()?.files.values_mut() {
        bytes.push(1);
    }
    let b = Pc::new(dir.path(), "b", &dir.path().join("saves-b"), true)?;
    b.connect()?;
    b.sync(&drive).reconcile()?;
    assert!(matches!(b.sync(&drive).download_now(&id), Err(CoreError::Integrity(_))));
    let leftovers: Vec<_> =
        fs::read_dir(b.snapshots.join("sekiro")).map(|entries| entries.flatten().collect()).unwrap_or_default();
    assert!(leftovers.is_empty(), "neither the archive nor a partial file is kept");
    assert_eq!(SnapshotRepository::new(&b.db).get(&id)?.status, SnapshotStatus::MissingLocal);
    Ok(())
}

#[test]
fn drive_snapshots_of_unknown_games_are_left_alone() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let a = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    a.connect()?;
    a.snapshot("chapter 1", SnapshotKind::Manual)?;
    a.sync(&drive).sync_all()?;
    let other = Pc::new(dir.path(), "other", &dir.path().join("saves-other"), false)?;
    other.connect()?;
    let report = other.sync(&drive).sync_all()?;
    assert_eq!((report.imported, report.skipped_unknown_games), (0, 1));
    Ok(())
}

#[test]
fn a_linear_history_across_two_pcs_is_not_a_conflict() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let a = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    a.connect()?;
    a.snapshot("chapter 1", SnapshotKind::Manual)?;
    a.sync(&drive).sync_all()?;
    let b = Pc::new(dir.path(), "b", &dir.path().join("saves-b"), true)?;
    b.connect()?;
    b.sync(&drive).sync_all()?;
    b.snapshot("chapter 2", SnapshotKind::Manual)?;
    let report = b.sync(&drive).sync_all()?;
    assert_eq!(report.new_conflicts, 0);
    assert_eq!(report.uploaded, 1);
    Ok(())
}

/// Both PCs save while offline from a shared starting point, then sync: a real divergence.
fn diverged(dir: &Path, drive: &FakeDrive) -> CoreResult<(Pc, Pc, String, String)> {
    let shared = dir.join("shared-saves");
    let a = Pc::new(dir, "a", &shared, true)?;
    a.connect()?;
    let b = Pc::new(dir, "b", &shared, true)?;
    b.connect()?;
    let a_snapshot = a.snapshot("played on A", SnapshotKind::Manual)?;
    let b_snapshot = b.snapshot("played on B", SnapshotKind::Manual)?;
    a.sync(drive).sync_all()?;
    let report = b.sync(drive).sync_all()?;
    assert_eq!(report.new_conflicts, 1);
    Ok((a, b, a_snapshot, b_snapshot))
}

#[test]
fn diverged_histories_open_one_conflict_and_never_overwrite() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let (_, b, a_snapshot, b_snapshot) = diverged(dir.path(), &drive)?;
    let sync = b.sync(&drive);
    let conflicts = sync.conflicts()?;
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].local.snapshot_id, b_snapshot);
    assert_eq!(conflicts[0].remote.snapshot_id, a_snapshot);
    assert!(!conflicts[0].remote.local_copy);
    assert_eq!(sync.detect_conflicts()?, 0, "the same pair is never reported twice");
    assert_eq!(drive.lock()?.files.len(), 2, "both saves are in Drive");
    Ok(())
}

#[test]
fn use_local_keeps_this_save_and_marks_the_other_as_a_conflict_copy() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let (_, b, a_snapshot, _) = diverged(dir.path(), &drive)?;
    let sync = b.sync(&drive);
    let id = sync.conflicts()?[0].id;
    sync.resolve_conflict(id, ConflictResolution::UseLocal, &b.engine())?;
    assert!(sync.conflicts()?.is_empty());
    assert_eq!(SnapshotRepository::new(&b.db).get(&a_snapshot)?.kind, SnapshotKind::ConflictCopy);
    assert_eq!(sync.detect_conflicts()?, 0);
    Ok(())
}

#[test]
fn keep_both_downloads_the_other_save_as_a_conflict_copy() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let (_, b, a_snapshot, _) = diverged(dir.path(), &drive)?;
    let sync = b.sync(&drive);
    let id = sync.conflicts()?[0].id;
    sync.resolve_conflict(id, ConflictResolution::KeepBoth, &b.engine())?;
    let kept = SnapshotRepository::new(&b.db).get(&a_snapshot)?;
    assert_eq!(kept.kind, SnapshotKind::ConflictCopy);
    assert_eq!(kept.status, SnapshotStatus::Ready, "the archive is on this PC now");
    Ok(())
}

#[test]
fn use_remote_restores_the_other_save_after_protecting_this_one() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let (_, b, a_snapshot, b_snapshot) = diverged(dir.path(), &drive)?;
    let sync = b.sync(&drive);
    let id = sync.conflicts()?[0].id;
    sync.resolve_conflict(id, ConflictResolution::UseRemote, &b.engine())?;
    assert_eq!(fs::read_to_string(b.saves.join("slot.sav"))?, "played on A");
    assert_eq!(SnapshotRepository::new(&b.db).get(&b_snapshot)?.kind, SnapshotKind::ConflictCopy);
    assert_eq!(SnapshotRepository::new(&b.db).get(&a_snapshot)?.kind, SnapshotKind::Manual);
    let safety_copies = SnapshotRepository::new(&b.db)
        .list(Some(b.game_id))?
        .into_iter()
        .filter(|s| s.kind == SnapshotKind::Safety)
        .count();
    assert!(safety_copies <= 1);
    Ok(())
}

#[test]
fn deleting_a_synced_snapshot_keeps_the_cloud_copy_listed() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let id = pc.snapshot("chapter 1", SnapshotKind::AutoExit)?;
    pc.sync(&drive).sync_all()?;
    pc.engine().delete(&id)?;
    let listed = SnapshotRepository::new(&pc.db).get(&id)?;
    assert_eq!(listed.status, SnapshotStatus::MissingLocal);
    assert_eq!(listed.cloud, CloudState::Synced);
    let within_drive_limit = RetentionPolicy { keep_local: 0, keep_cloud: 10 };
    assert_eq!(
        pc.engine().apply_retention(pc.game_id, within_drive_limit)?,
        RetentionReport::default(),
        "a cloud-only snapshot inside the Drive limit is kept"
    );

    pc.sync(&drive).download_now(&id)?;
    assert_eq!(SnapshotRepository::new(&pc.db).get(&id)?.status, SnapshotStatus::Ready);
    Ok(())
}

#[test]
fn reconcile_notices_copies_that_vanished_from_drive_and_requeues_them() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let id = pc.snapshot("chapter 1", SnapshotKind::Manual)?;
    pc.sync(&drive).sync_all()?;
    drive.lock()?.files.clear();
    let report = pc.sync(&drive).sync_all()?;
    assert_eq!(report.uploaded, 1, "the missing copy is uploaded again");
    assert_eq!(pc.cloud_state(&id)?, CloudState::Synced);
    Ok(())
}

#[test]
fn disconnecting_forgets_cloud_state_but_keeps_snapshots() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let id = pc.snapshot("chapter 1", SnapshotKind::Manual)?;
    pc.sync(&drive).sync_all()?;
    CloudRepository::new(&pc.db).disconnect()?;
    assert_eq!(pc.cloud_state(&id)?, CloudState::None);
    assert!(!pc.sync(&drive).queue_upload(&id)?, "no account, nothing queued");
    pc.engine().delete(&id)?;
    assert!(SnapshotRepository::new(&pc.db).get(&id).is_err(), "a local-only snapshot is fully deleted");
    Ok(())
}

// ---------------------------------------------------------------- retention in Drive

/// `count` automatic snapshots, oldest first, each uploaded to `drive`.
fn synced_autos(pc: &Pc, drive: &FakeDrive, count: usize) -> CoreResult<Vec<String>> {
    let mut ids = Vec::new();
    for round in 0..count {
        ids.push(pc.snapshot(&format!("round {round}"), SnapshotKind::AutoExit)?);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    pc.sync(drive).sync_all()?;
    Ok(ids)
}

fn drive_files(drive: &FakeDrive) -> CoreResult<usize> {
    Ok(drive.lock()?.files.len())
}

fn exists(pc: &Pc, id: &str) -> bool {
    SnapshotRepository::new(&pc.db).get(id).is_ok()
}

fn pending_deletions(pc: &Pc) -> CoreResult<i64> {
    Ok(pc.db.connection()?.query_row("SELECT COUNT(*) FROM cloud_deletions", [], |row| row.get(0))?)
}

#[test]
fn retention_deletes_the_oldest_automatic_snapshots_from_drive_too() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let ids = synced_autos(&pc, &drive, 3)?;
    assert_eq!(drive_files(&drive)?, 3);

    let report = pc.engine().apply_retention(pc.game_id, RetentionPolicy { keep_local: 1, keep_cloud: 0 })?;
    assert_eq!(report, RetentionReport { removed_local: 2, removed_cloud: 2 });
    assert!(!exists(&pc, &ids[0]) && !exists(&pc, &ids[1]), "forgotten once no copy is left anywhere");
    assert!(exists(&pc, &ids[2]));
    assert_eq!(drive_files(&drive)?, 3, "Drive is only contacted by the queue");

    let queue = pc.sync(&drive).run_queue()?;
    assert_eq!((queue.deleted, queue.failed), (2, 0));
    let left: Vec<String> = drive.lock()?.files.values().map(|(meta, _)| meta.snapshot_id.clone()).collect();
    assert_eq!(left, vec![ids[2].clone()]);
    assert_eq!(pending_deletions(&pc)?, 0);

    // The next full sync neither re-imports the deleted copies nor re-uploads anything.
    let report = pc.sync(&drive).sync_all()?;
    assert_eq!((report.imported, report.uploaded), (0, 0));
    assert_eq!(SnapshotRepository::new(&pc.db).list(Some(pc.game_id))?.len(), 1);
    Ok(())
}

#[test]
fn a_deletion_survives_being_offline_and_signed_out() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let ids = synced_autos(&pc, &drive, 2)?;
    pc.engine().apply_retention(pc.game_id, RetentionPolicy { keep_local: 1, keep_cloud: 0 })?;
    assert_eq!(pending_deletions(&pc)?, 1);

    // Offline: the attempt fails, the entry stays, and a sync pass does not bring the file back.
    drive.lock()?.delete_failures.push(Failure::Network);
    let queue = pc.sync(&drive).run_queue()?;
    assert_eq!((queue.deleted, queue.failed), (0, 1));
    assert_eq!(pending_deletions(&pc)?, 1);
    assert_eq!(pc.sync(&drive).reconcile()?.imported, 0);
    assert!(!exists(&pc, &ids[0]));

    // Retry time not reached yet: nothing is attempted.
    assert_eq!(pc.sync(&drive).run_queue()?.deleted, 0);
    pc.db.connection()?.execute("UPDATE cloud_deletions SET next_attempt_at = '2000-01-01T00:00:00Z'", [])?;

    // Signed out: the queue pauses with an authentication error, the entry stays.
    let account = CloudRepository::new(&pc.db).account()?.ok_or(CoreError::NotFound("account".into()))?;
    CloudRepository::new(&pc.db).set_status(account.id, AccountStatus::NeedsReauth)?;
    assert!(matches!(pc.sync(&drive).run_queue(), Err(CoreError::Auth(_))));
    assert_eq!(pending_deletions(&pc)?, 1);
    assert_eq!(drive_files(&drive)?, 2);

    // Back online and signed in again: the Drive copy is deleted.
    CloudRepository::new(&pc.db).set_status(account.id, AccountStatus::Connected)?;
    assert_eq!(pc.sync(&drive).run_queue()?.deleted, 1);
    assert_eq!(drive_files(&drive)?, 1);
    assert_eq!(pending_deletions(&pc)?, 0);
    Ok(())
}

#[test]
fn a_copy_already_gone_from_drive_counts_as_deleted() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    synced_autos(&pc, &drive, 2)?;
    pc.engine().apply_retention(pc.game_id, RetentionPolicy { keep_local: 1, keep_cloud: 0 })?;
    drive.lock()?.files.clear();
    let queue = pc.sync(&drive).run_queue()?;
    assert_eq!((queue.deleted, queue.failed), (1, 0));
    assert_eq!(pending_deletions(&pc)?, 0);
    Ok(())
}

#[test]
fn a_drive_file_that_is_not_the_recorded_snapshot_is_never_deleted() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let ids = synced_autos(&pc, &drive, 2)?;
    pc.engine().apply_retention(pc.game_id, RetentionPolicy { keep_local: 1, keep_cloud: 0 })?;
    for (meta, _) in drive.lock()?.files.values_mut() {
        if meta.snapshot_id == ids[0] {
            meta.snapshot_id = "someone-else".into();
        }
    }
    let queue = pc.sync(&drive).run_queue()?;
    assert_eq!((queue.deleted, queue.failed), (0, 1));
    assert_eq!(drive_files(&drive)?, 2, "nothing was removed");
    assert_eq!(pc.sync(&drive).run_queue()?.failed, 0, "parked: not retried automatically");
    Ok(())
}

#[test]
fn retention_never_touches_manual_or_pinned_snapshots() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let manual = pc.snapshot("named by me", SnapshotKind::Manual)?;
    let pinned = pc.snapshot("pinned", SnapshotKind::AutoExit)?;
    SnapshotRepository::new(&pc.db).set_pinned(&pinned, true)?;
    synced_autos(&pc, &drive, 3)?;
    assert_eq!(drive_files(&drive)?, 5);

    let report = pc.engine().apply_retention(pc.game_id, RetentionPolicy { keep_local: 1, keep_cloud: 1 })?;
    assert_eq!(report, RetentionReport { removed_local: 2, removed_cloud: 2 });
    pc.sync(&drive).run_queue()?;
    assert_eq!(drive_files(&drive)?, 3, "manual, pinned and the newest automatic snapshot");
    for id in [&manual, &pinned] {
        let snapshot = SnapshotRepository::new(&pc.db).get(id)?;
        assert_eq!((snapshot.status, snapshot.cloud), (SnapshotStatus::Ready, CloudState::Synced));
    }
    Ok(())
}

#[test]
fn drive_keeps_more_history_than_this_pc_when_asked() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let ids = synced_autos(&pc, &drive, 4)?;

    let report = pc.engine().apply_retention(pc.game_id, RetentionPolicy { keep_local: 1, keep_cloud: 3 })?;
    assert_eq!(report, RetentionReport { removed_local: 3, removed_cloud: 1 });
    pc.sync(&drive).run_queue()?;
    assert_eq!(drive_files(&drive)?, 3, "Drive keeps the newest 3");
    assert!(!exists(&pc, &ids[0]), "the oldest left both places");
    let on_pc: Vec<_> = SnapshotRepository::new(&pc.db)
        .list(Some(pc.game_id))?
        .into_iter()
        .filter(|snapshot| snapshot.status == SnapshotStatus::Ready)
        .map(|snapshot| snapshot.id)
        .collect();
    assert_eq!(on_pc, vec![ids[3].clone()]);
    let cloud_only = SnapshotRepository::new(&pc.db).get(&ids[1])?;
    assert_eq!((cloud_only.status, cloud_only.cloud), (SnapshotStatus::MissingLocal, CloudState::Synced));

    // A Drive limit below the PC limit cannot make a local snapshot upload again and again.
    let report = pc.engine().apply_retention(pc.game_id, RetentionPolicy { keep_local: 3, keep_cloud: 1 })?;
    assert_eq!(report, RetentionReport::default());
    Ok(())
}

#[test]
fn deleting_from_pc_and_drive_removes_both_copies() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let ids = synced_autos(&pc, &drive, 2)?;
    pc.engine().delete(&ids[0])?;
    assert_eq!(drive_files(&drive)?, 2, "a plain delete keeps the Drive copy");

    pc.engine().delete_everywhere(&ids[1])?;
    assert!(!exists(&pc, &ids[1]));
    assert_eq!(pc.sync(&drive).run_queue()?.deleted, 1);
    assert_eq!(drive_files(&drive)?, 1);
    // The snapshot deleted from this PC only is still listed as cloud-only and can be removed later.
    pc.engine().delete_everywhere(&ids[0])?;
    assert_eq!(pc.sync(&drive).run_queue()?.deleted, 1);
    assert_eq!(drive_files(&drive)?, 0);
    Ok(())
}

fn archive_on_disk(pc: &Pc, id: &str) -> CoreResult<bool> {
    Ok(SnapshotRepository::new(&pc.db).get(id)?.archive_path.is_some_and(|path| Path::new(&path).is_file()))
}

#[test]
fn a_pc_only_snapshot_is_never_uploaded() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let id = pc.snapshot_stored("private", SnapshotKind::Manual, SnapshotStorage::Local)?;
    let sync = pc.sync(&drive);
    assert!(!sync.queue_upload(&id)?, "queueing a PC-only snapshot is refused");
    assert_eq!(sync.queue_missing_uploads()?, 0, "so is the catch-up after connecting");
    sync.run_queue()?;
    assert_eq!(drive_files(&drive)?, 0);
    assert_eq!(pc.cloud_state(&id)?, CloudState::None);
    assert!(archive_on_disk(&pc, &id)?);
    Ok(())
}

#[test]
fn a_drive_only_snapshot_drops_its_local_archive_after_the_upload_is_confirmed() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let id = pc.snapshot_stored("only in the cloud", SnapshotKind::Manual, SnapshotStorage::Cloud)?;
    let sync = pc.sync(&drive);
    assert!(sync.queue_upload(&id)?);
    assert!(archive_on_disk(&pc, &id)?, "the archive stays until Drive confirms the upload");

    sync.run_queue()?;
    assert_eq!(pc.cloud_state(&id)?, CloudState::Synced);
    assert_eq!(drive_files(&drive)?, 1);
    let summary = SnapshotRepository::new(&pc.db).get(&id)?;
    assert!(summary.archive_path.is_none(), "the local archive is gone");
    assert_eq!(summary.status, SnapshotStatus::MissingLocal, "the row stays as a cloud-only snapshot");
    assert!(!pc.snapshots.join("sekiro").join(format!("{id}.zip")).exists());
    Ok(())
}

#[test]
fn a_drive_only_snapshot_keeps_its_local_archive_while_the_upload_keeps_failing() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    drive.lock()?.upload_failures.extend([Failure::Network, Failure::Network]);
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let id = pc.snapshot_stored("only copy", SnapshotKind::Manual, SnapshotStorage::Cloud)?;
    let sync = pc.sync(&drive);
    sync.queue_upload(&id)?;

    for _ in 0..2 {
        let report = sync.run_queue()?;
        assert_eq!((report.uploaded, report.failed), (0, 1));
        assert!(archive_on_disk(&pc, &id)?, "never lose the only copy");
        make_due_now(&pc)?;
    }
    sync.run_queue()?;
    assert!(!archive_on_disk(&pc, &id)?, "dropped once the third attempt is confirmed");
    assert_eq!(drive_files(&drive)?, 1);
    Ok(())
}

#[test]
fn a_snapshot_for_both_keeps_the_local_archive_and_the_drive_copy() -> CoreResult<()> {
    let dir = tempfile::tempdir()?;
    let drive = FakeDrive::default();
    let pc = Pc::new(dir.path(), "a", &dir.path().join("saves-a"), true)?;
    pc.connect()?;
    let id = pc.snapshot_stored("both", SnapshotKind::Manual, SnapshotStorage::Both)?;
    let sync = pc.sync(&drive);
    sync.queue_upload(&id)?;
    sync.run_queue()?;
    assert_eq!(pc.cloud_state(&id)?, CloudState::Synced);
    assert!(archive_on_disk(&pc, &id)?);
    assert_eq!(drive_files(&drive)?, 1);
    Ok(())
}
