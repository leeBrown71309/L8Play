//! Path forms that can be compared: one folder can be written several ways on Windows
//! (`C:\Users\RUNNER~1\…` and `C:\Users\runneradmin\…`, with or without `\?\`, any case).

use std::path::{Path, PathBuf};

use crate::saves::normalize_path;

/// Canonical path without the `\?\` prefix Windows adds, so paths stay readable and comparable.
/// A path that does not exist is returned as given.
pub fn dunce_canonicalize(path: &Path) -> PathBuf {
    match std::fs::canonicalize(path) {
        Ok(canonical) => {
            let text = canonical.to_string_lossy();
            text.strip_prefix(r"\?\").map_or(canonical.clone(), PathBuf::from)
        }
        Err(_) => path.to_path_buf(),
    }
}

/// Comparison key for "is this the same file or folder": canonical (long names), then normalized.
pub fn path_key(path: &Path) -> String {
    normalize_path(&dunce_canonicalize(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_and_long_names_of_a_folder_give_the_same_key() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let folder = dir.path().join("A Folder With A Long Name");
        std::fs::create_dir_all(&folder)?;
        let written_differently = PathBuf::from(folder.to_string_lossy().to_uppercase());
        assert_eq!(path_key(&folder), path_key(&written_differently));
        assert_eq!(path_key(Path::new(r"Z:\does\not\exist")), r"z:\does\not\exist");
        Ok(())
    }
}
