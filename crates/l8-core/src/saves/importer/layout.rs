//! Reads how a game's backup folder is organised: one save, or several named snapshots.
//!
//! Example: `Stellar Blade/{after ending A, after ending B, before final choice}/SaveGames/…` are three
//! manual snapshots of one game, while `Elden Ring/76561…/ER0000.sav` is a single save.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::saves::importer::noise::{is_noise, is_noise_directory};

/// How deep a chain of single sub-folders is followed to find the snapshots
/// (`Khazan/Saved/<ending names>` is two levels down).
const MAX_DESCENT: usize = 4;
const MIN_SHAPE_SIMILARITY: f64 = 0.5;

/// What lives directly inside a folder, ignoring noise and hidden entries.
#[derive(Debug, Default)]
pub struct Listing {
    pub files: Vec<PathBuf>,
    pub folders: Vec<PathBuf>,
}

pub fn list_folder(folder: &Path, ignored_file: Option<&str>) -> Listing {
    let mut listing = Listing::default();
    let Ok(entries) = std::fs::read_dir(folder) else { return listing };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(kind) = entry.file_type() else { continue };
        if name.starts_with('.') || kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            if !is_noise_directory(&name) && has_files(&entry.path()) {
                listing.folders.push(entry.path());
            }
        } else if kind.is_file() && !is_noise(Path::new(&name)) && ignored_file != Some(name.as_str()) {
            listing.files.push(entry.path());
        }
    }
    listing.files.sort();
    listing.folders.sort_by_key(|path| file_name_lower(path));
    listing
}

/// Folders (with their names) that are the named snapshots of a game, if the game is organised that way.
///
/// Starting at `root`, a chain of single sub-folders is followed (nothing else may be next to it) until a
/// folder holds several sub-folders that look like snapshots.
pub fn find_variants(root: &Path, ignored_file: Option<&str>) -> Option<Vec<(String, PathBuf)>> {
    let mut current = root.to_path_buf();
    for _ in 0..=MAX_DESCENT {
        let listing = list_folder(&current, ignored_file);
        if !listing.files.is_empty() {
            return None;
        }
        match listing.folders.as_slice() {
            [] => return None,
            [only] => current = only.clone(),
            many => return variants_among(many),
        }
    }
    None
}

/// The sub-folders of `root` itself when they look like snapshots (no descent).
pub fn variants_at(root: &Path, ignored_file: Option<&str>) -> Option<Vec<(String, PathBuf)>> {
    let listing = list_folder(root, ignored_file);
    if listing.files.is_empty() { variants_among(&listing.folders) } else { None }
}

fn variants_among(folders: &[PathBuf]) -> Option<Vec<(String, PathBuf)>> {
    if folders.len() < 2 {
        return None;
    }
    let names: Vec<String> =
        folders.iter().map(|path| path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()).collect();
    let all_phrases = names.iter().all(|name| is_human_label(name) && name.contains(' '));
    let all_labels_and_alike = names.iter().all(|name| is_human_label(name)) && shapes_are_alike(folders);
    (all_phrases || all_labels_and_alike).then(|| names.into_iter().zip(folders.iter().cloned()).collect())
}

/// A name a person typed to describe a save (`after ending A`), as opposed to an identifier
/// (`76561199774717161`, `Save_eCjgz0F4YUuqOxInfRGCJA`, `2026-09-10 18-36-21`).
pub fn is_human_label(name: &str) -> bool {
    let total = name.chars().count();
    let letters = name.chars().filter(|c| c.is_alphabetic()).count();
    let digits = name.chars().filter(char::is_ascii_digit).count();
    if letters < 3 || total == 0 {
        return false;
    }
    let mostly_digits = digits * 10 >= total * 4;
    let looks_random = !name.contains(' ') && total > 20;
    !mostly_digits && !looks_random
}

fn shapes_are_alike(folders: &[PathBuf]) -> bool {
    let shapes: Vec<BTreeSet<String>> = folders.iter().map(|folder| shape_of(folder)).collect();
    let Some(first) = shapes.first() else { return false };
    shapes.iter().skip(1).all(|other| jaccard(first, other) >= MIN_SHAPE_SIMILARITY)
}

/// The relative paths of a folder with every number and identifier blurred: two saves of the same game
/// have the same shape even though slot numbers and Steam ids differ.
fn shape_of(folder: &Path) -> BTreeSet<String> {
    WalkDir::new(folder)
        .follow_links(false)
        .max_depth(8)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| entry.path().strip_prefix(folder).ok().map(blur_path))
        .collect()
}

fn blur_path(relative: &Path) -> String {
    relative
        .components()
        .map(|part| {
            let text = part.as_os_str().to_string_lossy().to_lowercase();
            let random = text.len() > 16 && !text.contains(' ');
            if random { "#".to_owned() } else { blur_digits(&text) }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn blur_digits(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_digits = false;
    for c in text.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                out.push('#');
            }
            in_digits = true;
        } else {
            out.push(c);
            in_digits = false;
        }
    }
    out
}

fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    a.intersection(b).count() as f64 / union as f64
}

fn has_files(folder: &Path) -> bool {
    WalkDir::new(folder)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .any(|entry| entry.file_type().is_file() && !is_noise(entry.path().strip_prefix(folder).unwrap_or(entry.path())))
}

fn file_name_lower(path: &Path) -> String {
    path.file_name().map(|name| name.to_string_lossy().to_lowercase()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_not_labels() {
        for name in ["76561199774717161", "Save_eCjgz0F4YUuqOxInfRGCJA", "2026-09-10 18-36-21", "0000000000000001"] {
            assert!(!is_human_label(name), "{name}");
        }
        for name in ["after ending A", "BEFORE ANY CHOICE", "1er fin CHAOS 3 PHASES", "2eme fin", "Before Final Choice"] {
            assert!(is_human_label(name), "{name}");
        }
    }

    #[test]
    fn blurring_ignores_numbers_and_identifiers() {
        assert_eq!(blur_path(Path::new("SaveGames/76561199774717161/Slot12.sav")), "savegames/#/slot#.sav");
    }
}
