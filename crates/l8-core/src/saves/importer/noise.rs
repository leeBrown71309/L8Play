//! Files that sit next to saves in old backups but are not saves: logs, crash dumps, screenshots.

use std::path::Path;

const NOISE_EXTENSIONS: &[&str] = &["log", "dmp", "mdmp", "tmp", "crash"];
const NOISE_DIRECTORIES: &[&str] = &["screenshots", "screenshot", "logs", "crashes", "crashpad", "crash reports"];
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp"];

/// `true` when `relative` (a path inside a backup folder) is not worth keeping.
pub fn is_noise(relative: &Path) -> bool {
    let in_noise_directory = relative
        .parent()
        .is_some_and(|parent| parent.components().any(|part| is_noise_directory(&part.as_os_str().to_string_lossy())));
    if in_noise_directory {
        return true;
    }
    let Some(name) = relative.file_name().map(|name| name.to_string_lossy().to_lowercase()) else { return false };
    let extension = relative.extension().map(|ext| ext.to_string_lossy().to_lowercase()).unwrap_or_default();
    NOISE_EXTENSIONS.contains(&extension.as_str())
        || (IMAGE_EXTENSIONS.contains(&extension.as_str()) && name.starts_with("screenshot"))
}

/// Directory names that only ever hold noise (used to ignore them while planning the layout).
pub fn is_noise_directory(name: &str) -> bool {
    NOISE_DIRECTORIES.contains(&name.trim().to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn logs_dumps_and_screenshots_are_noise() {
        for path in [
            "Game.log",
            "Game-v1.0 2025-02-25.mdmp",
            "Screenshots/shot1.png",
            "ScreenShot-2023-4-10_20-29-32.png",
            "SaveGames/Crashes/x.dat",
        ] {
            assert!(is_noise(&PathBuf::from(path)), "{path}");
        }
    }

    #[test]
    fn saves_and_settings_are_kept() {
        for path in ["SaveGames/slot1.sav", "GraphicsConfig.xml", "Config.txt", "portrait.png", "Saves/Autosave1.ess"] {
            assert!(!is_noise(&PathBuf::from(path)), "{path}");
        }
    }
}
