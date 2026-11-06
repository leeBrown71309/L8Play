//! Snapshot archive format: a standard zip anyone can open, plus `l8-snapshot.json`.
//!
//! Layout: `files/<location-index>/<relative path>` for each save file, and a manifest with the
//! original location paths and a SHA-256 per file. Archives are self-describing, so a snapshot
//! can be understood (and restored by hand) without L8Play.

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::clock::to_rfc3339;
use crate::error::{CoreError, CoreResult};
use crate::saves::{LocationKind, SaveLocation};
use crate::snapshots::files::{SourceFile, copy_hashing, open_exclusive_read, safe_relative, sha256_of};

pub const MANIFEST_ENTRY: &str = "l8-snapshot.json";
const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedLocation {
    pub index: usize,
    pub path: String,
    pub kind: LocationKind,
    /// `true` for a snapshot imported from a legacy backup: the folder it came from is not a save
    /// folder of this PC, so `path` is empty and the target is chosen when the snapshot is restored.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unresolved: bool,
    /// Where the files probably belong (e.g. `%LOCALAPPDATA%\Game\Saved\SaveGames`), read from a
    /// hint file shipped with the backup. Only a suggestion, never written to without confirmation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl ArchivedLocation {
    pub fn resolved(index: usize, location: &SaveLocation) -> Self {
        Self { index, path: location.path.clone(), kind: location.kind, unresolved: false, hint: None }
    }

    /// The single location of an imported snapshot.
    pub fn imported(hint: Option<String>) -> Self {
        Self { index: 0, path: String::new(), kind: LocationKind::Directory, unresolved: true, hint }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedFile {
    pub entry: String,
    pub location: usize,
    pub relative_path: String,
    pub original_path: String,
    pub size: u64,
    pub sha256: String,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveManifest {
    pub format_version: u32,
    pub snapshot_id: String,
    pub game_title: String,
    pub created_at: String,
    pub locations: Vec<ArchivedLocation>,
    pub files: Vec<ArchivedFile>,
}

impl ArchiveManifest {
    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|file| file.size).sum()
    }

    /// What the archive contains, independent of where it came from: (archive entry, SHA-256).
    /// Two snapshots with the same signature hold byte-identical files at identical relative paths.
    pub fn signature(&self) -> std::collections::BTreeSet<(String, String)> {
        self.files.iter().map(|file| (file.entry.clone(), file.sha256.clone())).collect()
    }
}

impl<'de> Deserialize<'de> for LocationKind {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Ok(LocationKind::from_db(&value))
    }
}

fn entry_name(location: usize, relative: &Path) -> String {
    let parts: Vec<String> =
        relative.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect();
    format!("files/{location}/{}", parts.join("/"))
}

/// Writes the archive to `destination` (a temporary path), hashing every file as it is read.
/// The caller renames it into place only after [`verify_archive`] succeeds.
pub fn write_archive(
    destination: &Path,
    snapshot_id: &str,
    game_title: &str,
    created_at: &str,
    locations: &[ArchivedLocation],
    files: &[SourceFile],
) -> CoreResult<ArchiveManifest> {
    let output = File::create(destination)?;
    let mut zip = ZipWriter::new(BufWriter::new(output));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).large_file(true);
    let mut archived = Vec::with_capacity(files.len());

    for file in files {
        let entry = entry_name(file.location_index, &file.relative);
        let mut source = BufReader::new(open_exclusive_read(&file.absolute)?);
        zip.start_file(entry.as_str(), options).map_err(zip_error)?;
        let (size, sha256) = copy_hashing(&mut source, &mut zip)?;
        archived.push(ArchivedFile {
            entry,
            location: file.location_index,
            relative_path: file.relative.to_string_lossy().replace('\\', "/"),
            original_path: file.absolute.to_string_lossy().into_owned(),
            size,
            sha256,
            modified_at: file.modified.map(to_rfc3339),
        });
    }

    let manifest = ArchiveManifest {
        format_version: FORMAT_VERSION,
        snapshot_id: snapshot_id.to_owned(),
        game_title: game_title.to_owned(),
        created_at: created_at.to_owned(),
        locations: locations.to_vec(),
        files: archived,
    };
    zip.start_file(MANIFEST_ENTRY, options).map_err(zip_error)?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest).map_err(|error| CoreError::Io(io::Error::other(error)))?)?;
    let writer = zip.finish().map_err(zip_error)?;
    let file = writer.into_inner().map_err(|error| CoreError::Io(error.into_error()))?;
    file.sync_all()?;
    Ok(manifest)
}

/// Re-reads the archive and checks every entry against its recorded size and SHA-256.
pub fn verify_archive(path: &Path) -> CoreResult<ArchiveManifest> {
    let mut archive = ZipArchive::new(BufReader::new(File::open(path)?)).map_err(zip_error)?;
    let manifest: ArchiveManifest = {
        let entry = archive
            .by_name(MANIFEST_ENTRY)
            .map_err(|_| CoreError::Integrity("the archive has no L8Play manifest".into()))?;
        serde_json::from_reader(entry).map_err(|error| CoreError::Integrity(format!("unreadable manifest: {error}")))?
    };
    for file in &manifest.files {
        if safe_relative(&file.relative_path).is_none() || manifest.locations.get(file.location).is_none() {
            return Err(CoreError::Integrity(format!("unsafe entry {}", file.entry)));
        }
        let mut entry =
            archive.by_name(&file.entry).map_err(|_| CoreError::Integrity(format!("{} is missing", file.entry)))?;
        let (size, sha256) = sha256_of(&mut entry)?;
        if size != file.size || sha256 != file.sha256 {
            return Err(CoreError::Integrity(format!("{} does not match its checksum", file.relative_path)));
        }
    }
    Ok(manifest)
}

/// Extracts one archived file to `destination`, verifying its checksum on the way.
pub fn extract_file(
    archive: &mut ZipArchive<BufReader<File>>,
    file: &ArchivedFile,
    destination: &Path,
) -> CoreResult<()> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut entry =
        archive.by_name(&file.entry).map_err(|_| CoreError::Integrity(format!("{} is missing", file.entry)))?;
    let mut output = BufWriter::new(File::create(destination)?);
    let (size, sha256) = copy_hashing(&mut entry, &mut output)?;
    output.flush()?;
    output.get_ref().sync_all()?;
    if size != file.size || sha256 != file.sha256 {
        return Err(CoreError::Integrity(format!("{} does not match its checksum", file.relative_path)));
    }
    Ok(())
}

pub fn open_archive(path: &Path) -> CoreResult<ZipArchive<BufReader<File>>> {
    ZipArchive::new(BufReader::new(File::open(path)?)).map_err(zip_error)
}

fn zip_error(error: zip::result::ZipError) -> CoreError {
    CoreError::Io(io::Error::other(error.to_string()))
}
