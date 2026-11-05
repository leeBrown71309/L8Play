//! Epic Games discovery from the launcher's JSON manifests
//! (`%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests\*.item`).
//!
//! The format is undocumented, so parsing is defensive: unknown fields are ignored, a malformed
//! file is skipped with a warning, and nothing here is ever fatal.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::CoreResult;
use crate::platforms::{DiscoveredGame, Discovery, LibraryProvider, Platform};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EpicManifest {
    app_name: Option<String>,
    display_name: Option<String>,
    install_location: Option<PathBuf>,
    launch_executable: Option<String>,
    install_size: Option<u64>,
    main_game_app_name: Option<String>,
    #[serde(default)]
    app_categories: Vec<String>,
    #[serde(rename = "bIsApplication", default = "default_true")]
    is_application: bool,
    #[serde(rename = "bIsIncompleteInstall", default)]
    is_incomplete_install: bool,
}

fn default_true() -> bool {
    true
}

pub struct EpicProvider {
    manifests_dir: PathBuf,
}

impl EpicProvider {
    /// Uses the launcher's standard manifest folder.
    pub fn from_environment() -> Self {
        let program_data =
            std::env::var_os("ProgramData").map_or_else(|| PathBuf::from(r"C:\ProgramData"), PathBuf::from);
        Self::with_manifests_dir(program_data.join("Epic").join("EpicGamesLauncher").join("Data").join("Manifests"))
    }

    pub fn with_manifests_dir(manifests_dir: PathBuf) -> Self {
        Self { manifests_dir }
    }
}

impl LibraryProvider for EpicProvider {
    fn platform(&self) -> Platform {
        Platform::Epic
    }

    fn discover(&self) -> CoreResult<Option<Discovery>> {
        if !self.manifests_dir.is_dir() {
            tracing::info!(dir = %self.manifests_dir.display(), "Epic manifests not found; skipping Epic discovery");
            return Ok(None);
        }
        let mut discovery = Discovery { library_paths: vec![self.manifests_dir.clone()], ..Discovery::default() };
        for entry in std::fs::read_dir(&self.manifests_dir)?.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|ext| !ext.eq_ignore_ascii_case("item")) {
                continue;
            }
            match read_manifest(&path) {
                Ok(Some(game)) => discovery.games.push(game),
                Ok(None) => {}
                Err(reason) => discovery.warnings.push(format!("Epic manifest skipped ({}): {reason}", path.display())),
            }
        }
        Ok(Some(discovery))
    }
}

fn read_manifest(path: &Path) -> Result<Option<DiscoveredGame>, String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let manifest: EpicManifest = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    Ok(to_game(manifest))
}

/// Keeps installed, complete games; drops DLC, engines and tools.
fn to_game(manifest: EpicManifest) -> Option<DiscoveredGame> {
    let app_name = manifest.app_name.filter(|name| !name.is_empty())?;
    let install_path = manifest.install_location.filter(|path| !path.as_os_str().is_empty())?;
    // Real manifests of base games carry an empty `MainGameAppName`; only a non-empty, different
    // name means the item belongs to another game (DLC).
    let is_dlc =
        manifest.main_game_app_name.as_deref().filter(|main| !main.is_empty()).is_some_and(|main| main != app_name);
    let is_game =
        manifest.app_categories.is_empty() || manifest.app_categories.iter().any(|category| category == "games");
    if !manifest.is_application || manifest.is_incomplete_install || is_dlc || !is_game {
        return None;
    }
    let executable_path = manifest.launch_executable.filter(|exe| !exe.is_empty()).map(|exe| install_path.join(exe));
    Some(DiscoveredGame {
        title: manifest.display_name.unwrap_or_else(|| app_name.clone()),
        platform_game_id: app_name,
        install_path,
        executable_path,
        size_bytes: manifest.install_size,
        last_played_at: None,
        playtime_seconds: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, body: &str) -> std::io::Result<()> {
        std::fs::write(dir.join(name), body)
    }

    /// Shape of the manifests Epic writes today (base games have an empty `MainGameAppName`).
    #[test]
    fn base_games_with_an_empty_main_game_name_are_kept() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        write(
            dir.path(),
            "rocket.item",
            r#"{"FormatVersion":0,"bIsIncompleteInstall":false,"LaunchExecutable":"Binaries/Win64/Launcher.exe","bIsApplication":true,"bIsManaged":false,"AppCategories":["public","games","applications"],"DisplayName":"Rocket League®","InstallLocation":"C:\\Program Files\\Epic Games\\rocketleague","InstallSize":1,"MainGameAppName":"","AppName":"Sugar"}"#,
        )?;
        let discovery = EpicProvider::with_manifests_dir(dir.path().to_path_buf())
            .discover()?
            .ok_or("the manifests folder exists")?;
        assert_eq!(discovery.games.len(), 1);
        assert_eq!(discovery.games[0].title, "Rocket League®");
        assert_eq!(discovery.games[0].platform_game_id, "Sugar");
        Ok(())
    }

    #[test]
    fn discovers_games_and_skips_the_rest() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        write(
            dir.path(),
            "a.item",
            r#"{"AppName":"Fennec","DisplayName":"Alan Wake 2","InstallLocation":"D:\\Epic\\AlanWake2","LaunchExecutable":"AlanWake2.exe","InstallSize":95000000000,"MainGameAppName":"Fennec","AppCategories":["public","games","applications"],"bIsApplication":true,"bIsIncompleteInstall":false,"UnknownField":42}"#,
        )?;
        write(
            dir.path(),
            "dlc.item",
            r#"{"AppName":"FennecDLC","DisplayName":"Night Springs","InstallLocation":"D:\\Epic\\AlanWake2","MainGameAppName":"Fennec","AppCategories":["addons"]}"#,
        )?;
        write(
            dir.path(),
            "engine.item",
            r#"{"AppName":"UE_5.4","DisplayName":"Unreal Engine","InstallLocation":"C:\\UE","AppCategories":["engines"]}"#,
        )?;
        write(
            dir.path(),
            "partial.item",
            r#"{"AppName":"Half","DisplayName":"Half","InstallLocation":"D:\\Half","bIsIncompleteInstall":true}"#,
        )?;
        write(dir.path(), "broken.item", "{ not json")?;
        write(dir.path(), "notes.txt", "ignored")?;

        let discovery =
            EpicProvider::with_manifests_dir(dir.path().to_path_buf()).discover()?.ok_or("expected discovery")?;
        assert_eq!(discovery.games.len(), 1);
        let game = &discovery.games[0];
        assert_eq!(game.platform_game_id, "Fennec");
        assert_eq!(game.title, "Alan Wake 2");
        assert_eq!(game.executable_path.as_deref(), Some(Path::new(r"D:\Epic\AlanWake2\AlanWake2.exe")));
        assert_eq!(discovery.warnings.len(), 1, "the malformed manifest is reported");
        Ok(())
    }

    #[test]
    fn missing_launcher_is_not_an_empty_library() -> CoreResult<()> {
        let provider = EpicProvider::with_manifests_dir(PathBuf::from(r"Z:\definitely\not\here"));
        assert!(provider.discover()?.is_none());
        Ok(())
    }
}
