//! Hint files: a note shipped with a backup that says where the files belong
//! (e.g. `location.txt` containing `%LOCALAPPDATA%\Game\Saved\SaveGames`).

use std::path::Path;

/// A restore target suggested by a hint file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    /// Name of the file in the backup folder that held the hint (left out of the snapshot).
    pub file_name: String,
    /// The suggested folder, with its environment variables unexpanded.
    pub target: String,
}

const MAX_HINT_FILE_BYTES: u64 = 4096;

/// Looks at the small text files directly inside `folder` for a Windows path with an
/// environment variable in it, and returns the first one.
pub fn find_hint(folder: &Path) -> Option<Hint> {
    let mut names: Vec<_> = std::fs::read_dir(folder)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter(|entry| entry.metadata().is_ok_and(|meta| meta.len() <= MAX_HINT_FILE_BYTES))
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("txt")))
        .collect();
    names.sort();
    names.into_iter().find_map(|path| {
        let text = std::fs::read_to_string(&path).ok()?;
        let target = text.lines().find_map(extract_target)?;
        Some(Hint { file_name: path.file_name()?.to_string_lossy().into_owned(), target })
    })
}

/// `Type %LOCALAPPDATA%\Game\Saved in the box and press Enter.` → `%LOCALAPPDATA%\Game\Saved`.
fn extract_target(line: &str) -> Option<String> {
    let start = line.find('%')?;
    let rest = line.get(start..)?;
    let closing = rest.get(1..)?.find('%')? + 1;
    let variable = rest.get(1..closing)?;
    if variable.is_empty() || !variable.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let after = rest.get(closing + 1..)?;
    if !after.starts_with('\\') {
        return None;
    }
    let end = after.find(" in the ").or_else(|| after.find(" and ")).unwrap_or(after.len());
    let path = after.get(..end)?.trim().trim_end_matches(['.', '"']);
    Some(format!("%{variable}%{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_path_out_of_an_instruction_sentence() {
        let line = r"Step 2: Type %LOCALAPPDATA%\The First Berserker Khazan\Saved\SaveGames in the box and press Enter.";
        assert_eq!(
            extract_target(line).as_deref(),
            Some(r"%LOCALAPPDATA%\The First Berserker Khazan\Saved\SaveGames")
        );
    }

    #[test]
    fn ignores_lines_without_an_environment_path() {
        assert_eq!(extract_target("Step 1: Press the Win + R keys to open the Run dialog."), None);
        assert_eq!(extract_target("100% done"), None);
    }

    #[test]
    fn finds_the_hint_file_in_a_folder() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("notes.txt"), "nothing here")?;
        std::fs::write(dir.path().join("location.txt"), "Type %APPDATA%\\Game\\Saves in the box.\n")?;
        let hint = find_hint(dir.path()).ok_or("no hint")?;
        assert_eq!(hint.file_name, "location.txt");
        assert_eq!(hint.target, r"%APPDATA%\Game\Saves");
        Ok(())
    }
}
