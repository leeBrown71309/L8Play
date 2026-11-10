//! Export of every snapshot this PC can read into one zip, laid out like the Google Drive folder.
//!
//! The export is built from the local archives, not from Drive, so it also holds snapshots that were
//! never uploaded. Layout: `L8Play/<game-slug>/<snapshot-id>.zip` for each snapshot (the same
//! archives, copied as they are) plus `L8Play/l8-export.json`, an index of what was exported.

use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::clock::now_rfc3339;
use crate::db::Database;
use crate::error::{CoreError, CoreResult};
use crate::snapshots::model::SnapshotSummary;
use crate::snapshots::repository::SnapshotRepository;

/// Folder inside the zip, named like the Drive folder so both can be browsed the same way.
const ROOT_FOLDER: &str = "L8Play";
const INDEX_ENTRY: &str = "L8Play/l8-export.json";

/// What an export wrote, and what it had to leave out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportReport {
    pub path: String,
    #[ts(type = "number")]
    pub snapshots: u32,
    #[ts(type = "number")]
    pub games: u32,
    /// Size of the zip written.
    #[ts(type = "number")]
    pub bytes: u64,
    /// Snapshots whose archive is not on this PC (kept in Drive only, or deleted): not in the zip.
    #[ts(type = "number")]
    pub skipped: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexEntry<'a> {
    entry: String,
    game_slug: String,
    #[serde(flatten)]
    snapshot: &'a SnapshotSummary,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportIndex<'a> {
    format_version: u32,
    exported_at: String,
    device_name: String,
    snapshots: Vec<IndexEntry<'a>>,
}

/// Writes every readable snapshot of the library into the zip at `destination`.
///
/// The zip is written to a temporary file next to `destination` and renamed once complete, so a
/// failure never leaves a half-written export under the final name.
pub fn export_all_snapshots(db: &Database, destination: &Path) -> CoreResult<ExportReport> {
    let repository = SnapshotRepository::new(db);
    let snapshots = repository.list(None)?;
    let temporary = temporary_path(destination);
    match write_export(db, &snapshots, &temporary) {
        Ok(mut report) => {
            if destination.exists() {
                std::fs::remove_file(destination)?;
            }
            std::fs::rename(&temporary, destination)?;
            report.path = destination.to_string_lossy().into_owned();
            Ok(report)
        }
        Err(error) => {
            // The partial file is ours; failing to remove it only leaves a stray `.part` file.
            let _ = std::fs::remove_file(&temporary);
            Err(error)
        }
    }
}

fn temporary_path(destination: &Path) -> PathBuf {
    let mut name = destination.file_name().map(|name| name.to_os_string()).unwrap_or_default();
    name.push(".part");
    destination.with_file_name(name)
}

fn write_export(db: &Database, snapshots: &[SnapshotSummary], temporary: &Path) -> CoreResult<ExportReport> {
    if let Some(folder) = temporary.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut zip = ZipWriter::new(BufWriter::new(File::create(temporary)?));
    // The inner archives are zips already: storing them avoids compressing twice for nothing.
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored).large_file(true);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let mut index = Vec::new();
    let mut games = HashSet::new();
    let mut skipped = 0u32;
    for snapshot in snapshots {
        let Some(archive) = snapshot.archive_path.as_deref().map(Path::new).filter(|path| path.is_file()) else {
            skipped += 1;
            continue;
        };
        let slug = game_slug(db, snapshot.game_id)?;
        let entry = format!("{ROOT_FOLDER}/{slug}/{}.zip", snapshot.id);
        zip.start_file(entry.as_str(), stored).map_err(zip_error)?;
        io::copy(&mut BufReader::new(File::open(archive)?), &mut zip)?;
        games.insert(snapshot.game_id);
        index.push(IndexEntry { entry, game_slug: slug, snapshot });
    }
    let device_name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "This PC".into());
    let exported = u32::try_from(index.len()).unwrap_or(u32::MAX);
    let document = ExportIndex { format_version: 1, exported_at: now_rfc3339(), device_name, snapshots: index };
    zip.start_file(INDEX_ENTRY, deflated).map_err(zip_error)?;
    zip.write_all(&serde_json::to_vec_pretty(&document).map_err(|error| CoreError::Io(io::Error::other(error)))?)?;

    let writer = zip.finish().map_err(zip_error)?;
    let file = writer.into_inner().map_err(|error| CoreError::Io(error.into_error()))?;
    file.sync_all()?;
    let bytes = file.metadata()?.len();
    Ok(ExportReport {
        path: String::new(),
        snapshots: exported,
        games: u32::try_from(games.len()).unwrap_or(u32::MAX),
        bytes,
        skipped,
    })
}

fn game_slug(db: &Database, game_id: i64) -> CoreResult<String> {
    Ok(db.connection()?.query_row("SELECT slug FROM games WHERE id = ?1", [game_id], |row| row.get(0))?)
}

fn zip_error(error: zip::result::ZipError) -> CoreError {
    CoreError::Io(io::Error::other(error.to_string()))
}
