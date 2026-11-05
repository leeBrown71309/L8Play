//! GOG discovery from the registry keys the GOG installers write
//! (`HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\<game id>`). Read-only: no GOG Galaxy database, no
//! credentials. The registry is behind a trait so the rules are tested without touching it.

use std::path::PathBuf;

use crate::error::CoreResult;
use crate::platforms::{DiscoveredGame, Discovery, LibraryProvider, Platform};

/// One `Games\<id>` registry key, as the installer wrote it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GogEntry {
    pub game_id: String,
    pub name: String,
    pub path: String,
    pub exe: String,
    /// Non-empty for add-ons that belong to another GOG product (they carry no executable).
    pub depends_on: String,
}

/// Source of the installed GOG entries. `None` means GOG is not installed on this PC.
pub trait GogRegistry {
    fn entries(&self) -> Option<Vec<GogEntry>>;
}

pub struct GogProvider<R: GogRegistry = WindowsGogRegistry> {
    registry: R,
}

impl GogProvider {
    pub fn from_environment() -> Self {
        Self::with_registry(WindowsGogRegistry)
    }
}

impl<R: GogRegistry> GogProvider<R> {
    pub fn with_registry(registry: R) -> Self {
        Self { registry }
    }
}

impl<R: GogRegistry> LibraryProvider for GogProvider<R> {
    fn platform(&self) -> Platform {
        Platform::Gog
    }

    fn discover(&self) -> CoreResult<Option<Discovery>> {
        let Some(entries) = self.registry.entries() else {
            tracing::info!("GOG registry keys not found; skipping GOG discovery");
            return Ok(None);
        };
        let mut discovery = Discovery::default();
        for entry in entries {
            match to_game(&entry) {
                Ok(Some(game)) => discovery.games.push(game),
                Ok(None) => {}
                Err(reason) => discovery.warnings.push(format!("GOG game skipped ({}): {reason}", entry.name)),
            }
        }
        Ok(Some(discovery))
    }
}

/// Keeps real games whose executable is still on disk; add-ons and leftovers are dropped.
fn to_game(entry: &GogEntry) -> Result<Option<DiscoveredGame>, String> {
    if !entry.depends_on.trim().is_empty() || entry.exe.trim().is_empty() {
        return Ok(None);
    }
    if entry.game_id.trim().is_empty() || !entry.game_id.chars().all(|c| c.is_ascii_digit()) {
        return Err("it has no valid game id".into());
    }
    let executable = PathBuf::from(entry.exe.trim());
    if !executable.is_file() {
        // A key left behind by an uninstall: not installed, so it is not reported.
        return Ok(None);
    }
    let install_path = if entry.path.trim().is_empty() {
        executable.parent().map(PathBuf::from).ok_or("the executable has no folder")?
    } else {
        PathBuf::from(entry.path.trim())
    };
    Ok(Some(DiscoveredGame {
        platform_game_id: entry.game_id.trim().to_owned(),
        title: entry.name.trim().to_owned(),
        install_path,
        executable_path: Some(executable),
        size_bytes: None,
        last_played_at: None,
        playtime_seconds: None,
    }))
}

/// The real registry (Windows only; elsewhere GOG is simply absent).
pub struct WindowsGogRegistry;

#[cfg(windows)]
impl GogRegistry for WindowsGogRegistry {
    fn entries(&self) -> Option<Vec<GogEntry>> {
        use winreg::RegKey;
        use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

        let games =
            RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(r"SOFTWARE\WOW6432Node\GOG.com\Games", KEY_READ);
        let games = match games {
            Ok(key) => key,
            Err(error) => {
                tracing::info!(%error, "GOG registry key not readable");
                return None;
            }
        };
        let text = |key: &RegKey, name: &str| key.get_value::<String, _>(name).unwrap_or_default();
        let entries = games
            .enum_keys()
            .flatten()
            .filter_map(|id| games.open_subkey_with_flags(&id, KEY_READ).ok().map(|key| (id, key)))
            .map(|(id, key)| GogEntry {
                game_id: id,
                name: text(&key, "gameName"),
                path: text(&key, "path"),
                exe: text(&key, "exe"),
                depends_on: text(&key, "dependsOn"),
            })
            .collect();
        Some(entries)
    }
}

#[cfg(not(windows))]
impl GogRegistry for WindowsGogRegistry {
    fn entries(&self) -> Option<Vec<GogEntry>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeRegistry(Option<Vec<GogEntry>>);

    impl GogRegistry for FakeRegistry {
        fn entries(&self) -> Option<Vec<GogEntry>> {
            self.0.clone()
        }
    }

    fn entry(id: &str, name: &str, folder: &std::path::Path, exe: &str, depends_on: &str) -> GogEntry {
        GogEntry {
            game_id: id.into(),
            name: name.into(),
            path: folder.to_string_lossy().into_owned(),
            exe: if exe.is_empty() { String::new() } else { folder.join(exe).to_string_lossy().into_owned() },
            depends_on: depends_on.into(),
        }
    }

    #[test]
    fn keeps_installed_games_and_drops_add_ons_and_leftovers() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("Moonlighter.exe"), b"MZ")?;
        let entries = vec![
            entry("2147483047", "Moonlighter", dir.path(), "Moonlighter.exe", ""),
            entry("1546956723", "Moonlighter - Between Dimensions", dir.path(), "", "2147483047"),
            entry("999", "Gone", dir.path(), "Gone.exe", ""),
            entry("not-a-number", "Odd", dir.path(), "Moonlighter.exe", ""),
        ];
        let discovery = GogProvider::with_registry(FakeRegistry(Some(entries))).discover()?.ok_or("GOG is present")?;
        assert_eq!(discovery.games.len(), 1);
        let game = &discovery.games[0];
        assert_eq!(game.platform_game_id, "2147483047");
        assert_eq!(game.title, "Moonlighter");
        assert_eq!(game.install_path, dir.path());
        assert_eq!(game.executable_path, Some(dir.path().join("Moonlighter.exe")));
        assert_eq!(discovery.warnings.len(), 1, "the entry with a bad id is reported");
        Ok(())
    }

    #[test]
    fn missing_registry_means_gog_is_absent() -> CoreResult<()> {
        assert!(GogProvider::with_registry(FakeRegistry(None)).discover()?.is_none());
        Ok(())
    }

    /// Reads this PC's real registry: `cargo test -p l8-core gog_live -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads the real registry of this PC"]
    fn gog_live() -> CoreResult<()> {
        println!("{:#?}", GogProvider::from_environment().discover()?);
        Ok(())
    }
}
