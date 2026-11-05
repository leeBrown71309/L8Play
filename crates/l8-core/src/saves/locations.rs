//! Turns a list of save files into a small, safe set of save locations.
//!
//! A location is a folder (everything inside is a save) or a single file. A file's parent folder
//! is used unless that folder is "too broad": a system or profile root (Documents, AppData, a
//! drive root…) or the game's install folder, which would pull gigabytes of game data into every
//! snapshot. In those cases only the file itself is kept.

use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum LocationKind {
    Directory,
    File,
}

impl LocationKind {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Directory => "directory",
            Self::File => "file",
        }
    }

    pub fn from_db(value: &str) -> Self {
        if value == "file" { Self::File } else { Self::Directory }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedLocation {
    pub path: PathBuf,
    pub kind: LocationKind,
}

/// Lower-case, backslash-separated, no trailing separator: the form used for comparisons.
pub fn normalize_path(path: &Path) -> String {
    let text = path.to_string_lossy().replace('/', "\\").to_lowercase();
    let text = text.strip_prefix(r"\\?\").map_or(text.clone(), str::to_owned);
    text.trim_end_matches('\\').to_owned()
}

fn is_same_or_ancestor(ancestor: &str, path: &str) -> bool {
    path == ancestor || path.starts_with(&format!("{ancestor}\\"))
}

/// Folders that must never become a save location, from the user's environment.
pub fn broad_roots_from_environment() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for var in [
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "PUBLIC",
        "ProgramData",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "SystemRoot",
    ] {
        if let Some(value) = std::env::var_os(var) {
            roots.push(PathBuf::from(value));
        }
    }
    if let Some(profile) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        for child in [
            "Documents",
            "Documents\\My Games",
            "Saved Games",
            "Desktop",
            "Downloads",
            "AppData",
            "AppData\\LocalLow",
            "OneDrive",
            "OneDrive\\Documents",
        ] {
            roots.push(profile.join(child));
        }
    }
    roots
}

/// True for drive roots, the listed broad roots, and the game install folder or its ancestors.
fn is_too_broad(folder: &Path, broad_roots: &[String], install: Option<&str>) -> bool {
    let normalized = normalize_path(folder);
    let is_drive_root = normalized.len() <= 3 && normalized.ends_with(':');
    let is_listed = broad_roots.contains(&normalized);
    let contains_install = install.is_some_and(|install| is_same_or_ancestor(&normalized, install));
    is_drive_root || is_listed || contains_install
}

/// Minimal set of locations covering `files`. Deterministic: sorted by path.
pub fn derive_locations(
    files: &[PathBuf],
    install_path: Option<&Path>,
    broad_roots: &[PathBuf],
) -> Vec<DerivedLocation> {
    let broad: Vec<String> = broad_roots.iter().map(|root| normalize_path(root)).collect();
    let install = install_path.map(normalize_path);

    let mut folders: Vec<PathBuf> = Vec::new();
    let mut single_files: Vec<PathBuf> = Vec::new();
    for file in files {
        match file.parent() {
            Some(parent) if !is_too_broad(parent, &broad, install.as_deref()) => folders.push(parent.to_path_buf()),
            _ => single_files.push(file.clone()),
        }
    }

    // Keep a folder only if no other candidate folder already contains it.
    folders.sort_by_key(|folder| normalize_path(folder).len());
    let mut kept: Vec<PathBuf> = Vec::new();
    for folder in folders {
        let normalized = normalize_path(&folder);
        if !kept.iter().any(|existing| is_same_or_ancestor(&normalize_path(existing), &normalized)) {
            kept.push(folder);
        }
    }
    let covered = |file: &Path| {
        let normalized = normalize_path(file);
        kept.iter().any(|folder| is_same_or_ancestor(&normalize_path(folder), &normalized))
    };

    let mut locations: Vec<DerivedLocation> =
        kept.iter().map(|path| DerivedLocation { path: path.clone(), kind: LocationKind::Directory }).collect();
    for file in single_files {
        if !covered(&file) && !locations.iter().any(|location| normalize_path(&location.path) == normalize_path(&file))
        {
            locations.push(DerivedLocation { path: file, kind: LocationKind::File });
        }
    }
    locations.sort_by_key(|location| normalize_path(&location.path));
    locations
}

/// Rejects folders a user must not pick as a save location (same rules as detection).
pub fn validate_user_folder(folder: &Path, install_path: Option<&Path>, broad_roots: &[PathBuf]) -> Result<(), String> {
    if !folder.is_dir() {
        return Err(format!("{} is not a folder", folder.display()));
    }
    let broad: Vec<String> = broad_roots.iter().map(|root| normalize_path(root)).collect();
    if is_too_broad(folder, &broad, install_path.map(normalize_path).as_deref()) {
        return Err(format!("{} is too broad to back up; choose the game's own save folder", folder.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn broad() -> Vec<PathBuf> {
        vec![
            PathBuf::from(r"C:\Users\ryu"),
            PathBuf::from(r"C:\Users\ryu\AppData\Roaming"),
            PathBuf::from(r"C:\Users\ryu\Documents"),
        ]
    }

    #[test]
    fn merges_nested_folders() {
        let files = vec![
            PathBuf::from(r"C:\Users\ryu\AppData\Roaming\Nightreign\76561198988209285\NR0000.sl2"),
            PathBuf::from(r"C:\Users\ryu\AppData\Roaming\Nightreign\GraphicsConfig.xml"),
        ];
        assert_eq!(
            derive_locations(&files, None, &broad()),
            vec![DerivedLocation {
                path: PathBuf::from(r"C:\Users\ryu\AppData\Roaming\Nightreign"),
                kind: LocationKind::Directory
            }]
        );
    }

    #[test]
    fn never_takes_the_install_folder() {
        let install = PathBuf::from(r"D:\SteamLibrary\steamapps\common\MonsterHunterWilds");
        let files = vec![
            PathBuf::from(r"C:\Program Files (x86)\Steam\userdata\1\2246340\remote\win64_save\data001Slot.bin"),
            PathBuf::from(r"C:\Program Files (x86)\Steam\userdata\1\2246340\remote\win64_save\data002Slot.bin"),
            install.join("config.ini"),
        ];
        assert_eq!(
            derive_locations(&files, Some(&install), &broad()),
            vec![
                DerivedLocation {
                    path: PathBuf::from(r"C:\Program Files (x86)\Steam\userdata\1\2246340\remote\win64_save"),
                    kind: LocationKind::Directory
                },
                DerivedLocation { path: install.join("config.ini"), kind: LocationKind::File },
            ]
        );
    }

    #[test]
    fn files_in_broad_roots_stay_files() {
        let files = vec![PathBuf::from(r"C:\Users\ryu\Documents\save.dat"), PathBuf::from(r"D:\save.bin")];
        let locations = derive_locations(&files, None, &broad());
        assert!(locations.iter().all(|location| location.kind == LocationKind::File));
        assert_eq!(locations.len(), 2);
    }

    #[test]
    fn folder_subpaths_of_broad_roots_are_fine() {
        let files = vec![PathBuf::from(r"C:\Users\ryu\Documents\My Games\Skyrim\Saves\a.ess")];
        assert_eq!(derive_locations(&files, None, &broad())[0].kind, LocationKind::Directory);
    }

    #[test]
    fn user_folders_are_validated() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let saves = dir.path().join("saves");
        std::fs::create_dir(&saves)?;
        assert!(validate_user_folder(&saves, None, &[]).is_ok());
        assert!(validate_user_folder(dir.path(), None, &[dir.path().to_path_buf()]).is_err());
        assert!(validate_user_folder(&saves, Some(&saves), &[]).is_err(), "install folder");
        assert!(validate_user_folder(&dir.path().join("missing"), None, &[]).is_err());
        Ok(())
    }
}
