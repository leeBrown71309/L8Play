//! What the sync engine needs from a cloud provider, and its Google Drive implementation.
//!
//! The metadata of every uploaded archive travels with it (Drive `appProperties`), which is how a
//! new PC rebuilds its snapshot list, with lineage, from Drive alone.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::cloud::drive::{DriveClient, DriveFile, ROOT_FOLDER_NAME, UploadSession, UploadSpec};
use crate::error::{CoreError, CoreResult};

/// Snapshot facts stored next to the archive in Drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteMeta {
    pub snapshot_id: String,
    pub parent_id: Option<String>,
    pub game_slug: String,
    pub game_title: String,
    pub kind: String,
    pub name: String,
    pub description: Option<String>,
    pub sha256: String,
    pub created_at: String,
    pub device_id: String,
    pub device_name: String,
    pub total_files: i64,
    pub total_bytes: i64,
    pub playtime_seconds: Option<i64>,
}

/// Drive limits `key + value` to 124 bytes per property; text is cut on a character boundary.
fn clip(value: &str, key: &str) -> String {
    let budget = 124usize.saturating_sub(key.len());
    let mut end = value.len().min(budget);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

impl RemoteMeta {
    pub fn to_properties(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let mut put = |key: &str, value: &str| {
            map.insert(key.to_owned(), clip(value, key));
        };
        put("l8id", &self.snapshot_id);
        if let Some(parent) = &self.parent_id {
            put("l8parent", parent);
        }
        put("l8slug", &self.game_slug);
        put("l8title", &self.game_title);
        put("l8kind", &self.kind);
        put("l8name", &self.name);
        put("l8sha", &self.sha256);
        put("l8at", &self.created_at);
        put("l8dev", &self.device_id);
        put("l8devname", &self.device_name);
        put("l8files", &self.total_files.to_string());
        put("l8bytes", &self.total_bytes.to_string());
        if let Some(play) = self.playtime_seconds {
            put("l8play", &play.to_string());
        }
        map
    }
}

/// An archive found in the cloud.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSnapshot {
    pub remote_id: String,
    pub size: i64,
    pub meta: RemoteMeta,
}

impl RemoteSnapshot {
    /// `None` when the file was not made by L8Play (missing or malformed metadata).
    pub fn from_drive(file: &DriveFile) -> Option<Self> {
        let property = |key: &str| file.app_properties.get(key).cloned();
        let number = |key: &str| property(key).and_then(|value| value.parse::<i64>().ok());
        Some(Self {
            remote_id: file.id.clone(),
            size: file.size,
            meta: RemoteMeta {
                snapshot_id: property("l8id").filter(|id| uuid::Uuid::parse_str(id).is_ok())?,
                parent_id: property("l8parent").filter(|id| uuid::Uuid::parse_str(id).is_ok()),
                game_slug: property("l8slug").filter(|slug| !slug.is_empty())?,
                game_title: property("l8title").unwrap_or_default(),
                kind: property("l8kind").unwrap_or_else(|| "manual".into()),
                name: property("l8name").unwrap_or_default(),
                description: file.description.clone().filter(|text| !text.is_empty()),
                sha256: property("l8sha").filter(|sha| sha.len() == 64)?,
                created_at: property("l8at")?,
                device_id: property("l8dev").filter(|id| uuid::Uuid::parse_str(id).is_ok())?,
                device_name: property("l8devname").unwrap_or_else(|| "Another PC".into()),
                total_files: number("l8files").unwrap_or(0),
                total_bytes: number("l8bytes").unwrap_or(0),
                playtime_seconds: number("l8play"),
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteObject {
    pub remote_id: String,
    pub size: i64,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteQuota {
    pub email: String,
    pub used: i64,
    pub limit: Option<i64>,
}

pub struct UploadRequest<'a> {
    pub meta: &'a RemoteMeta,
    pub local_path: &'a Path,
}

/// A cloud provider as the sync engine sees it.
pub trait RemoteStore {
    fn quota(&self) -> CoreResult<RemoteQuota>;
    fn list_snapshots(&self) -> CoreResult<Vec<RemoteSnapshot>>;
    fn upload(
        &self,
        request: &UploadRequest<'_>,
        session: &mut UploadSession,
        on_progress: &mut dyn FnMut(&UploadSession),
    ) -> CoreResult<RemoteObject>;
    /// Downloads into `dest` (the engine passes a temporary path and verifies the checksum).
    fn download(&self, remote_id: &str, dest: &Path) -> CoreResult<()>;
    /// Permanently deletes a snapshot's archive; `false` when it was already gone. A file that is
    /// not that snapshot's archive is refused (`CoreError::Validation`).
    fn delete_snapshot(&self, remote_id: &str, snapshot_id: &str) -> CoreResult<bool>;
}

pub struct DriveRemote<'a> {
    client: DriveClient<'a>,
    /// Folder ids found or created in this session (root, then one per game).
    folders: Mutex<HashMap<String, String>>,
}

impl<'a> DriveRemote<'a> {
    pub fn new(client: DriveClient<'a>) -> Self {
        Self { client, folders: Mutex::new(HashMap::new()) }
    }

    /// `L8Play/` (and, with a slug, `L8Play/<slug>/`) created on first use.
    pub fn ensure_folder(&self, slug: Option<&str>) -> CoreResult<String> {
        let key = slug.unwrap_or("").to_owned();
        let lock = || self.folders.lock().map_err(|_| CoreError::LockPoisoned("drive folders"));
        if let Some(id) = lock()?.get(&key) {
            return Ok(id.clone());
        }
        let id = match slug {
            None => self.client.ensure_folder(ROOT_FOLDER_NAME, None)?,
            Some(slug) => {
                let root = self.ensure_folder(None)?;
                self.client.ensure_folder(slug, Some(&root))?
            }
        };
        lock()?.insert(key, id.clone());
        Ok(id)
    }
}

impl RemoteStore for DriveRemote<'_> {
    fn quota(&self) -> CoreResult<RemoteQuota> {
        let about = self.client.about()?;
        Ok(RemoteQuota { email: about.email, used: about.used, limit: about.limit })
    }

    fn list_snapshots(&self) -> CoreResult<Vec<RemoteSnapshot>> {
        let files = self.client.list("mimeType != 'application/vnd.google-apps.folder' and trashed = false")?;
        Ok(files.iter().filter_map(RemoteSnapshot::from_drive).collect())
    }

    fn upload(
        &self,
        request: &UploadRequest<'_>,
        session: &mut UploadSession,
        on_progress: &mut dyn FnMut(&UploadSession),
    ) -> CoreResult<RemoteObject> {
        let meta = request.meta;
        let folder = self.ensure_folder(Some(&meta.game_slug))?;
        let name = format!("{}.zip", meta.snapshot_id);
        let properties = meta.to_properties();
        let spec = UploadSpec {
            parent: &folder,
            name: &name,
            description: meta.description.as_deref(),
            app_properties: &properties,
            path: request.local_path,
        };
        let file = self.client.upload_resumable(&spec, session, on_progress)?;
        Ok(RemoteObject {
            remote_id: file.id,
            size: file.size,
            path: format!("{ROOT_FOLDER_NAME}/{}/{name}", meta.game_slug),
        })
    }

    fn download(&self, remote_id: &str, dest: &Path) -> CoreResult<()> {
        let mut file = File::create(dest)?;
        self.client.download(remote_id, &mut file)?;
        file.sync_all()?;
        Ok(())
    }

    fn delete_snapshot(&self, remote_id: &str, snapshot_id: &str) -> CoreResult<bool> {
        self.client.delete_snapshot_file(remote_id, snapshot_id)
    }
}

/// Convenience for callers that only have a folder path.
pub fn partial_path(dest: &Path) -> PathBuf {
    let mut name = dest.file_name().map(std::ffi::OsStr::to_os_string).unwrap_or_default();
    name.push(".partial");
    dest.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> RemoteMeta {
        RemoteMeta {
            snapshot_id: "0190a0a0-0000-7000-8000-000000000001".into(),
            parent_id: Some("0190a0a0-0000-7000-8000-000000000000".into()),
            game_slug: "sekiro".into(),
            game_title: "Sekiro: Shadows Die Twice".into(),
            kind: "manual".into(),
            name: "Before the boss".into(),
            description: Some("note".into()),
            sha256: "a".repeat(64),
            created_at: "2026-11-04T21:12:00Z".into(),
            device_id: "0190a0a0-0000-7000-8000-0000000000aa".into(),
            device_name: "DESKTOP-RYU".into(),
            total_files: 3,
            total_bytes: 1234,
            playtime_seconds: Some(3600),
        }
    }

    fn drive_file(properties: HashMap<String, String>) -> DriveFile {
        DriveFile {
            id: "remote1".into(),
            name: "x.zip".into(),
            size: 99,
            description: Some("note".into()),
            parents: Vec::new(),
            app_properties: properties,
        }
    }

    #[test]
    fn metadata_survives_a_round_trip_through_drive_properties() {
        let original = meta();
        let parsed = RemoteSnapshot::from_drive(&drive_file(original.to_properties()));
        assert_eq!(parsed.map(|snapshot| snapshot.meta), Some(original));
    }

    #[test]
    fn files_not_made_by_the_app_are_ignored() {
        assert!(RemoteSnapshot::from_drive(&drive_file(HashMap::new())).is_none());
        let mut broken = meta().to_properties();
        broken.insert("l8sha".into(), "short".into());
        assert!(RemoteSnapshot::from_drive(&drive_file(broken)).is_none());
        let mut bad_id = meta().to_properties();
        bad_id.insert("l8id".into(), "../../etc".into());
        assert!(RemoteSnapshot::from_drive(&drive_file(bad_id)).is_none());
    }

    #[test]
    fn long_text_is_clipped_to_the_property_limit_on_a_character_boundary() {
        let mut long = meta();
        long.name = "é".repeat(100);
        let properties = long.to_properties();
        let value = &properties["l8name"];
        assert!(value.len() + "l8name".len() <= 124);
        assert!(value.chars().all(|c| c == 'é'));
    }

    #[test]
    fn partial_files_sit_next_to_their_destination() {
        assert_eq!(partial_path(Path::new(r"C:\a\b.zip")), PathBuf::from(r"C:\a\b.zip.partial"));
    }
}
