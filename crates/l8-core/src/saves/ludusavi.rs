//! Thin, read-only wrapper around the Ludusavi CLI (MIT), bundled as a sidecar.
//!
//! Only `find` and `backup --preview` are used: Ludusavi never writes anything for L8Play.
//! It keeps its own configuration and manifest in a private folder (`--config`), updates the
//! manifest itself (ETag-based) and works offline with the last copy (`--try-manifest-update`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::error::{CoreError, CoreResult};

/// Windows: do not flash a console window for each call.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone)]
pub struct LudusaviCli {
    executable: PathBuf,
    config_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct FindOutput {
    #[serde(default)]
    games: HashMap<String, FindGame>,
}

#[derive(Debug, Deserialize)]
struct FindGame {
    score: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct BackupOutput {
    #[serde(default)]
    games: HashMap<String, PreviewGame>,
}

#[derive(Debug, Deserialize)]
struct PreviewGame {
    #[serde(default)]
    files: HashMap<String, PreviewFile>,
}

#[derive(Debug, Deserialize)]
struct PreviewFile {
    #[serde(default)]
    ignored: bool,
    #[serde(default)]
    failed: bool,
}

impl LudusaviCli {
    pub fn new(executable: PathBuf, config_dir: PathBuf) -> Self {
        Self { executable, config_dir }
    }

    pub fn is_available(&self) -> bool {
        self.executable.is_file()
    }

    /// Manifest title for a Steam App ID, if Ludusavi knows the game.
    pub fn find_by_steam_id(&self, app_id: &str) -> CoreResult<Option<String>> {
        if !app_id.chars().all(|c| c.is_ascii_digit()) {
            return Ok(None);
        }
        let output = self.run(&["find", "--api", "--steam-id", app_id])?;
        Ok(best_title(&output))
    }

    /// Manifest title for a display name (exact, then normalized matching).
    pub fn find_by_name(&self, title: &str) -> CoreResult<Option<String>> {
        let exact = self.run(&["find", "--api", title])?;
        if let Some(found) = best_title(&exact) {
            return Ok(Some(found));
        }
        let normalized = self.run(&["find", "--api", "--normalized", title])?;
        Ok(best_title(&normalized))
    }

    /// Files Ludusavi would back up for one game, as absolute paths. Ignored and failed entries
    /// are left out. An empty list means the game is known but has no save on this PC yet.
    pub fn preview_files(&self, manifest_title: &str) -> CoreResult<Vec<PathBuf>> {
        let output = self.run(&["backup", "--preview", "--api", manifest_title])?;
        parse_preview(&output, manifest_title)
    }

    /// Every game of Ludusavi's database that has save files on this PC right now, installed or not:
    /// manifest title -> absolute file paths. One call scans the whole manifest, so it takes a while.
    pub fn preview_all(&self) -> CoreResult<HashMap<String, Vec<PathBuf>>> {
        let output = self.run(&["backup", "--preview", "--api"])?;
        parse_preview_all(&output)
    }

    fn run(&self, args: &[&str]) -> CoreResult<String> {
        if !self.is_available() {
            return Err(CoreError::External(format!("Ludusavi is missing at {}", self.executable.display())));
        }
        std::fs::create_dir_all(&self.config_dir)?;
        let mut command = Command::new(&self.executable);
        command.arg("--config").arg(&self.config_dir).arg("--try-manifest-update").args(args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let output =
            command.output().map_err(|error| CoreError::External(format!("could not run Ludusavi: {error}")))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            tracing::debug!(?args, stderr = %stderr.trim(), "ludusavi stderr");
        }
        // Unknown games exit non-zero but still print valid JSON; only empty stdout is a failure.
        if stdout.trim().is_empty() {
            return Err(CoreError::External(format!(
                "Ludusavi returned no data (exit {:?}): {}",
                output.status.code(),
                stderr.trim()
            )));
        }
        Ok(stdout)
    }
}

fn best_title(json: &str) -> Option<String> {
    let parsed: FindOutput = serde_json::from_str(json).ok()?;
    parsed
        .games
        .into_iter()
        .max_by(|a, b| a.1.score.unwrap_or(0.0).total_cmp(&b.1.score.unwrap_or(0.0)))
        .map(|(title, _)| title)
}

fn parse_preview(json: &str, manifest_title: &str) -> CoreResult<Vec<PathBuf>> {
    let parsed: BackupOutput = serde_json::from_str(json)
        .map_err(|error| CoreError::External(format!("unreadable Ludusavi output: {error}")))?;
    let Some(game) = parsed.games.get(manifest_title) else { return Ok(Vec::new()) };
    let mut files: Vec<PathBuf> = game
        .files
        .iter()
        .filter(|(_, file)| !file.ignored && !file.failed)
        .map(|(path, _)| to_windows_path(path))
        .collect();
    files.sort();
    Ok(files)
}

fn parse_preview_all(json: &str) -> CoreResult<HashMap<String, Vec<PathBuf>>> {
    let parsed: BackupOutput = serde_json::from_str(json)
        .map_err(|error| CoreError::External(format!("unreadable Ludusavi output: {error}")))?;
    let mut games = HashMap::new();
    for (title, game) in parsed.games {
        let mut files: Vec<PathBuf> = game
            .files
            .iter()
            .filter(|(_, file)| !file.ignored && !file.failed)
            .map(|(path, _)| to_windows_path(path))
            .collect();
        if files.is_empty() {
            continue;
        }
        files.sort();
        games.insert(title, files);
    }
    Ok(games)
}

/// Ludusavi prints `C:/Users/...`; Windows APIs and our comparisons expect backslashes.
fn to_windows_path(path: &str) -> PathBuf {
    PathBuf::from(path.replace('/', "\\"))
}

/// Default location of the sidecar next to the application executable (where Tauri puts it).
pub fn sidecar_path(app_executable: &Path) -> Option<PathBuf> {
    app_executable.parent().map(|dir| dir.join("ludusavi.exe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_best_find_match() {
        let json = r#"{"games":{"Elden Ring Nightreign":{"score":1.0},"Elden Ring":{"score":0.6}}}"#;
        assert_eq!(best_title(json).as_deref(), Some("Elden Ring Nightreign"));
        assert_eq!(best_title(r#"{"errors":{"unknownGames":["993090"]},"games":{}}"#), None);
        assert_eq!(best_title("not json"), None);
    }

    #[test]
    fn parses_preview_files() -> CoreResult<()> {
        let json = r#"{
          "overall": {"totalGames": 1},
          "games": {
            "Monster Hunter Wilds": {
              "decision": "Processed",
              "files": {
                "C:/Program Files (x86)/Steam/userdata/1/2246340/remote/win64_save/data001Slot.bin": {"bytes": 346860},
                "D:/SteamLibrary/steamapps/common/MonsterHunterWilds/config.ini": {"bytes": 24564},
                "C:/ignored.bin": {"bytes": 1, "ignored": true},
                "C:/failed.bin": {"bytes": 1, "failed": true}
              },
              "registry": {}
            }
          }
        }"#;
        let files = parse_preview(json, "Monster Hunter Wilds")?;
        assert_eq!(
            files,
            vec![
                PathBuf::from(r"C:\Program Files (x86)\Steam\userdata\1\2246340\remote\win64_save\data001Slot.bin"),
                PathBuf::from(r"D:\SteamLibrary\steamapps\common\MonsterHunterWilds\config.ini"),
            ]
        );
        assert!(parse_preview(json, "Other Game")?.is_empty());
        Ok(())
    }

    #[test]
    fn preview_all_keeps_only_games_with_files() -> CoreResult<()> {
        let json = r#"{"games":{
            "Hades":{"files":{"C:/Users/me/Saved Games/Hades/Profile1.sav":{"bytes":9}}},
            "Empty":{"files":{}},
            "Ignored":{"files":{"C:/x.bin":{"bytes":1,"ignored":true}}}
        }}"#;
        let games = parse_preview_all(json)?;
        assert_eq!(games.len(), 1);
        assert_eq!(games["Hades"], vec![PathBuf::from(r"C:\Users\me\Saved Games\Hades\Profile1.sav")]);
        Ok(())
    }

    #[test]
    fn missing_sidecar_is_reported() {
        let cli = LudusaviCli::new(PathBuf::from(r"Z:\nope\ludusavi.exe"), PathBuf::from(r"Z:\nope"));
        assert!(!cli.is_available());
        assert!(matches!(cli.find_by_steam_id("1"), Err(CoreError::External(_))));
    }
}
