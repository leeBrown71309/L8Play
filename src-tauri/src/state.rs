//! Shared application state managed by Tauri.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use l8_core::artwork::{ArtworkService, CredentialManagerStore, UreqHttp};
use l8_core::cloud::drive::DriveClient;
use l8_core::cloud::{CloudSync, DriveRemote, GoogleTokens, UreqTransport};
use l8_core::db::Database;
use l8_core::launcher::SessionTracker;
use l8_core::platforms::steam_owned::OwnedLibraryStatus;
use l8_core::saves::LudusaviCli;
use l8_core::settings::SettingsRepository;
use l8_core::snapshots::migrate_legacy_root;
use l8_core::{CoreError, CoreResult};

const DEFAULT_SNAPSHOT_SUBFOLDER: [&str; 2] = ["L8Play", "Snapshots"];
/// The folder name used before the app was renamed (with a space).
const LEGACY_SNAPSHOT_SUBFOLDER: [&str; 2] = ["L8 Play", "Snapshots"];

pub struct AppState {
    pub db: Database,
    pub ludusavi: LudusaviCli,
    /// Windows Credential Manager (SteamGridDB key).
    secrets: CredentialManagerStore,
    /// Shared HTTP client: its 1 request/second pace covers every artwork request.
    http: UreqHttp,
    /// Transport for the Google APIs (OAuth and Drive).
    google: UreqTransport,
    /// Cached Google access token (memory only) and its expiry.
    token_cache: Mutex<Option<(String, std::time::Instant)>>,
    /// One cloud pass at a time: the worker and "Sync now" never overlap.
    cloud_lock: Mutex<()>,
    /// `%LOCALAPPDATA%com.l8play.appartwork` (re-creatable cache).
    artwork_root: PathBuf,
    /// The user's Documents folder (default parent of the snapshot folder).
    documents_dir: PathBuf,
    tracker: Mutex<SessionTracker>,
    /// Serializes library scans (startup scan and the "Scan libraries" button may overlap).
    scan_lock: Mutex<()>,
    /// Outcome of the last owned-games lookup (memory only; the settings page shows a hint from it).
    owned_library: Mutex<OwnedLibraryStatus>,
    /// Serializes snapshot work: a restore never runs alongside a snapshot of the same files.
    snapshot_lock: Mutex<()>,
}

impl AppState {
    pub fn open(
        database_path: &Path,
        ludusavi: LudusaviCli,
        documents_dir: PathBuf,
        artwork_root: PathBuf,
    ) -> CoreResult<Self> {
        let state = Self::build(database_path, ludusavi, documents_dir, artwork_root)?;
        state.migrate_default_snapshot_folder();
        Ok(state)
    }

    /// Moves the old "L8 Play/Snapshots" folder in Documents to "L8Play/Snapshots" when the user never chose a
    /// custom folder, keeping the recorded archive paths valid.
    fn migrate_default_snapshot_folder(&self) {
        let custom =
            SettingsRepository::new(&self.db).load().map(|settings| !settings.snapshot_folder.trim().is_empty());
        if custom.unwrap_or(true) {
            return;
        }
        let join = |parts: [&str; 2]| parts.iter().fold(self.documents_dir.clone(), |path, part| path.join(part));
        if let Err(error) =
            migrate_legacy_root(&self.db, &join(LEGACY_SNAPSHOT_SUBFOLDER), &join(DEFAULT_SNAPSHOT_SUBFOLDER))
        {
            tracing::warn!(%error, "could not move the snapshot folder to its new name; it stays where it is");
        }
    }

    fn build(
        database_path: &Path,
        ludusavi: LudusaviCli,
        documents_dir: PathBuf,
        artwork_root: PathBuf,
    ) -> CoreResult<Self> {
        Ok(Self {
            db: Database::open(database_path)?,
            ludusavi,
            secrets: CredentialManagerStore,
            http: UreqHttp::new(),
            google: UreqTransport::new(),
            token_cache: Mutex::new(None),
            cloud_lock: Mutex::new(()),
            artwork_root,
            documents_dir,
            tracker: Mutex::new(SessionTracker::new()),
            scan_lock: Mutex::new(()),
            owned_library: Mutex::new(OwnedLibraryStatus::default()),
            snapshot_lock: Mutex::new(()),
        })
    }

    /// What the last owned-games lookup found (`Disabled` until a scan has run).
    pub fn owned_library_status(&self) -> OwnedLibraryStatus {
        self.owned_library.lock().map(|status| *status).unwrap_or_default()
    }

    pub fn set_owned_library_status(&self, status: OwnedLibraryStatus) {
        if let Ok(mut current) = self.owned_library.lock() {
            *current = status;
        }
    }

    pub fn documents_dir(&self) -> &Path {
        &self.documents_dir
    }

    /// Shared HTTP client (also used for read-only knowledge lookups).
    pub fn http(&self) -> &UreqHttp {
        &self.http
    }

    pub fn artwork_root(&self) -> &Path {
        &self.artwork_root
    }

    pub fn artwork(&self) -> ArtworkService<'_> {
        ArtworkService::new(&self.db, &self.artwork_root, &self.secrets, &self.http)
    }

    pub fn secrets(&self) -> &CredentialManagerStore {
        &self.secrets
    }

    pub fn google(&self) -> &UreqTransport {
        &self.google
    }

    pub fn forget_access_token(&self) {
        if let Ok(mut cache) = self.token_cache.lock() {
            *cache = None;
        }
    }

    pub fn cloud_lock(&self) -> CoreResult<MutexGuard<'_, ()>> {
        self.cloud_lock.lock().map_err(|_| CoreError::LockPoisoned("cloud lock"))
    }

    /// Runs `work` with a sync engine wired to Google Drive and this PC's snapshot folder.
    pub fn cloud<T>(&self, work: impl FnOnce(&CloudSync<'_>) -> CoreResult<T>) -> CoreResult<T> {
        let tokens = GoogleTokens::new(&self.secrets, &self.google, &self.token_cache);
        let remote = DriveRemote::new(DriveClient::new(&self.google, &tokens));
        let sync = CloudSync::new(&self.db, &remote, self.snapshot_root()?);
        work(&sync)
    }

    /// Like [`Self::cloud`], for callers that need the Drive remote itself (account, quota, folders).
    pub fn drive<T>(&self, work: impl FnOnce(&DriveRemote<'_>) -> CoreResult<T>) -> CoreResult<T> {
        let tokens = GoogleTokens::new(&self.secrets, &self.google, &self.token_cache);
        let remote = DriveRemote::new(DriveClient::new(&self.google, &tokens));
        work(&remote)
    }

    pub fn tracker(&self) -> CoreResult<MutexGuard<'_, SessionTracker>> {
        self.tracker.lock().map_err(|_| CoreError::LockPoisoned("session tracker"))
    }

    pub fn scan_lock(&self) -> CoreResult<MutexGuard<'_, ()>> {
        self.scan_lock.lock().map_err(|_| CoreError::LockPoisoned("scan lock"))
    }

    pub fn snapshot_lock(&self) -> CoreResult<MutexGuard<'_, ()>> {
        self.snapshot_lock.lock().map_err(|_| CoreError::LockPoisoned("snapshot lock"))
    }

    /// The configured snapshot folder, or `Documents\L8Play\Snapshots`.
    pub fn snapshot_root(&self) -> CoreResult<PathBuf> {
        let configured = SettingsRepository::new(&self.db).load()?.snapshot_folder;
        if !configured.trim().is_empty() {
            return Ok(PathBuf::from(configured));
        }
        Ok(DEFAULT_SNAPSHOT_SUBFOLDER.iter().fold(self.documents_dir.clone(), |path, part| path.join(part)))
    }

    /// Refuses snapshot and restore work while the game is running.
    pub fn ensure_not_running(&self, game_id: i64) -> CoreResult<()> {
        if self.tracker()?.running().contains(&game_id) {
            return Err(CoreError::Busy("the game is running; close it first".into()));
        }
        Ok(())
    }
}
