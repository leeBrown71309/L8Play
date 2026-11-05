//! Finds which installed games currently have a running process.
//!
//! A game counts as running when any process's executable lives inside its install folder.
//! This works whether the game was started by L8Play, Steam, Epic or a desktop shortcut.

use std::collections::HashSet;
use std::path::Path;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::error::{CoreError, CoreResult};
use crate::library::WatchedGame;

pub struct ProcessMonitor {
    system: System,
}

impl ProcessMonitor {
    pub fn new() -> CoreResult<Self> {
        let system = System::new()
            .map_err(|error| CoreError::Io(std::io::Error::other(format!("process list unavailable: {error}"))))?;
        Ok(Self { system })
    }

    /// Refreshes the process list (executable paths only — cheap) and returns running game ids.
    pub fn running_games(&mut self, watched: &[WatchedGame]) -> HashSet<i64> {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
        );
        let executables: Vec<String> =
            self.system.processes().values().filter_map(|process| process.exe()).map(normalize).collect();
        match_running(&executables, watched)
    }
}

/// Pure matching step, separated for tests.
fn match_running(executables: &[String], watched: &[WatchedGame]) -> HashSet<i64> {
    let folders: Vec<(i64, String)> = watched
        .iter()
        .map(|game| (game.game_id, format!("{}\\", normalize(&game.install_path).trim_end_matches('\\'))))
        .collect();
    folders
        .iter()
        // A folder shallower than "X:\a\" would match unrelated programs (e.g. a game installed at a drive root).
        .filter(|(_, folder)| folder.matches('\\').count() >= 2)
        .filter(|(_, folder)| executables.iter().any(|exe| exe.starts_with(folder.as_str())))
        .map(|(game_id, _)| *game_id)
        .collect()
}

/// Lower-case, backslash-separated, without the `\\?\` verbatim prefix.
fn normalize(path: &Path) -> String {
    let text = path.to_string_lossy().replace('/', "\\").to_lowercase();
    text.strip_prefix(r"\\?\").map_or(text.clone(), str::to_owned)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn watched(id: i64, path: &str) -> WatchedGame {
        WatchedGame { game_id: id, install_path: PathBuf::from(path) }
    }

    #[test]
    fn matches_processes_inside_install_folders() {
        let exes = vec![
            normalize(Path::new(r"D:\SteamLibrary\steamapps\common\Sekiro\sekiro.exe")),
            normalize(Path::new(r"C:\Program Files (x86)\Steam\steam.exe")),
            normalize(Path::new(r"\\?\D:\Epic\AlanWake2\bin\AlanWake2.exe")),
        ];
        let games = [
            watched(1, r"D:\SteamLibrary\steamapps\common\Sekiro"),
            watched(2, r"d:\epic\alanwake2\"),
            watched(3, r"D:\SteamLibrary\steamapps\common\Sekiro Demo"),
        ];
        let running = match_running(&exes, &games);
        assert_eq!(running, HashSet::from([1, 2]), "prefix must end at a folder boundary");
    }

    #[test]
    fn ignores_drive_roots() {
        let exes = vec![normalize(Path::new(r"D:\anything.exe"))];
        assert!(match_running(&exes, &[watched(1, r"D:\")]).is_empty());
    }
}
