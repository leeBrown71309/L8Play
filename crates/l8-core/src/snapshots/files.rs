//! Reading save files safely: enumeration, in-use detection, hashing.

use std::fs::File;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::error::{CoreError, CoreResult};
use crate::saves::{LocationKind, SaveLocation};

/// Hard limits that stop a mis-detected location (e.g. a whole game folder) from being archived.
const MAX_FILES: usize = 20_000;
const MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const CHUNK: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub location_index: usize,
    /// Path inside the location (`a/b.sav`), or the file name for file locations.
    pub relative: PathBuf,
    pub absolute: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

/// Lists every file to back up. Symbolic links and junctions are skipped, never followed.
pub fn collect_files(locations: &[SaveLocation]) -> CoreResult<Vec<SourceFile>> {
    let mut files = Vec::new();
    let mut total: u64 = 0;
    for (index, location) in locations.iter().enumerate() {
        let root = location.path_buf();
        match location.kind {
            LocationKind::File => {
                if let Ok(metadata) = std::fs::symlink_metadata(&root)
                    && metadata.is_file()
                {
                    let name = root
                        .file_name()
                        .map(PathBuf::from)
                        .ok_or_else(|| CoreError::Validation(format!("{} has no file name", root.display())))?;
                    total += metadata.len();
                    files.push(SourceFile {
                        location_index: index,
                        relative: name,
                        absolute: root.clone(),
                        size: metadata.len(),
                        modified: metadata.modified().ok(),
                    });
                }
            }
            LocationKind::Directory => {
                if !root.is_dir() {
                    continue;
                }
                for entry in WalkDir::new(&root).follow_links(false) {
                    let entry = entry.map_err(|error| CoreError::Io(io::Error::other(error.to_string())))?;
                    if entry.path_is_symlink() || !entry.file_type().is_file() {
                        continue;
                    }
                    let metadata =
                        entry.metadata().map_err(|error| CoreError::Io(io::Error::other(error.to_string())))?;
                    let relative = entry
                        .path()
                        .strip_prefix(&root)
                        .map_err(|_| CoreError::Validation("file outside its save location".into()))?
                        .to_path_buf();
                    total += metadata.len();
                    files.push(SourceFile {
                        location_index: index,
                        relative,
                        absolute: entry.path().to_path_buf(),
                        size: metadata.len(),
                        modified: metadata.modified().ok(),
                    });
                    if files.len() > MAX_FILES || total > MAX_BYTES {
                        return Err(CoreError::Validation(format!(
                            "{} holds more than {MAX_FILES} files or 4 GB; check that it is really a save folder",
                            root.display()
                        )));
                    }
                }
            }
        }
    }
    files.sort_by(|a, b| (a.location_index, &a.relative).cmp(&(b.location_index, &b.relative)));
    Ok(files)
}

/// Lists the files of a plain folder that is not (yet) a save location of any game, as location 0.
/// `skip` receives each path relative to `root` and returns `true` for files that must be left out
/// (logs, screenshots…). Symbolic links and junctions are skipped, never followed.
pub fn collect_folder(root: &Path, skip: &dyn Fn(&Path) -> bool) -> CoreResult<Vec<SourceFile>> {
    if !root.is_dir() {
        return Err(CoreError::Validation(format!("{} is not a folder", root.display())));
    }
    let mut files = Vec::new();
    let mut total: u64 = 0;
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|error| CoreError::Io(io::Error::other(error.to_string())))?;
        if entry.path_is_symlink() || !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| CoreError::Validation("file outside its folder".into()))?
            .to_path_buf();
        if skip(&relative) {
            continue;
        }
        let metadata = entry.metadata().map_err(|error| CoreError::Io(io::Error::other(error.to_string())))?;
        total += metadata.len();
        files.push(SourceFile {
            location_index: 0,
            relative,
            absolute: entry.path().to_path_buf(),
            size: metadata.len(),
            modified: metadata.modified().ok(),
        });
        if files.len() > MAX_FILES || total > MAX_BYTES {
            return Err(CoreError::Validation(format!(
                "{} holds more than {MAX_FILES} files or 4 GB; check that it is really a save folder",
                root.display()
            )));
        }
    }
    files.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(files)
}

/// Opens a save file for reading while refusing to share it with a writer: if the game (or
/// anything else) still has it open for writing, this fails instead of copying a half-written file.
pub fn open_exclusive_read(path: &Path) -> CoreResult<File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 0x1;
        options.share_mode(FILE_SHARE_READ);
    }
    options.open(path).map_err(|error| {
        // ERROR_SHARING_VIOLATION / ERROR_LOCK_VIOLATION
        if matches!(error.raw_os_error(), Some(32 | 33)) {
            CoreError::Busy(format!("{} is in use by another program", path.display()))
        } else {
            CoreError::Io(error)
        }
    })
}

/// Streams `reader` into `writer`, returning (bytes, SHA-256 hex).
pub fn copy_hashing(reader: &mut impl Read, writer: &mut impl io::Write) -> io::Result<(u64, String)> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; CHUNK];
    let mut total = 0_u64;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let chunk = buffer.get(..read).unwrap_or_default();
        hasher.update(chunk);
        writer.write_all(chunk)?;
        total += read as u64;
    }
    Ok((total, hex::encode(hasher.finalize())))
}

pub fn sha256_of(reader: &mut impl Read) -> io::Result<(u64, String)> {
    copy_hashing(reader, &mut io::sink())
}

pub fn sha256_file(path: &Path) -> CoreResult<String> {
    Ok(sha256_of(&mut File::open(path)?)?.1)
}

/// Accepts only plain relative paths (no root, drive, `..` or `.`): the zip-slip guard.
pub fn safe_relative(path: &str) -> Option<PathBuf> {
    let candidate = Path::new(path);
    let mut clean = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => clean.push(part),
            _ => return None,
        }
    }
    (!clean.as_os_str().is_empty()).then_some(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_slip_paths_are_rejected() {
        assert_eq!(safe_relative("saves/slot1.sav"), Some(PathBuf::from("saves").join("slot1.sav")));
        assert_eq!(safe_relative("../escape.sav"), None);
        assert_eq!(safe_relative("saves/../../escape"), None);
        assert_eq!(safe_relative(r"C:\Windows\x"), None);
        assert_eq!(safe_relative("/abs"), None);
        assert_eq!(safe_relative(""), None);
    }

    #[test]
    fn hashes_while_copying() -> io::Result<()> {
        let mut out = Vec::new();
        let (bytes, hash) = copy_hashing(&mut &b"hello"[..], &mut out)?;
        assert_eq!(bytes, 5);
        assert_eq!(out, b"hello");
        assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn files_open_for_writing_are_busy() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("save.dat");
        std::fs::write(&path, b"data")?;
        let writer = std::fs::OpenOptions::new().write(true).open(&path)?;
        assert!(matches!(open_exclusive_read(&path), Err(CoreError::Busy(_))));
        drop(writer);
        assert!(open_exclusive_read(&path).is_ok());
        Ok(())
    }
}
