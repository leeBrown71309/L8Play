//! Launching games and tracking when they run.
//!
//! Steam and Epic games go through their official launchers (URI schemes) and Xbox games through
//! their package (the Windows shell), so their DRM, cloud sync and overlays keep working and no
//! store credential is ever needed. GOG and standalone games start their executable directly.

mod monitor;
mod sessions;

use std::path::PathBuf;

pub use monitor::ProcessMonitor;
pub use sessions::{SessionChange, SessionTracker};

use crate::error::{CoreError, CoreResult};
use crate::library::LaunchTarget;
use crate::platforms::Platform;

/// How the app shell must start a game. Executing it is the shell's job (it owns the OS APIs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchPlan {
    /// Open a launcher URI (`steam://…`, `com.epicgames.launcher://…`).
    Uri(String),
    /// Open an installed app package by its application user model id, the way the Start menu
    /// does (`explorer.exe "shell:AppsFolder" plus a backslash and the id`).
    AppPackage(String),
    /// Start an executable with its folder as working directory.
    Executable { path: PathBuf, working_dir: PathBuf },
}

/// Builds the launch plan for a game, or explains why it cannot be launched.
pub fn plan_launch(target: &LaunchTarget) -> CoreResult<LaunchPlan> {
    match target.platform {
        Platform::Steam => {
            let app_id = target
                .platform_game_id
                .as_deref()
                .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()));
            let app_id = app_id.ok_or_else(|| CoreError::Validation("this Steam game has no valid App ID".into()))?;
            let action = if target.installed { "rungameid" } else { "install" };
            Ok(LaunchPlan::Uri(format!("steam://{action}/{app_id}")))
        }
        Platform::Epic => {
            let app_name = target.platform_game_id.as_deref().filter(|name| !name.is_empty());
            let app_name = app_name.ok_or_else(|| CoreError::Validation("this Epic game has no app name".into()))?;
            Ok(LaunchPlan::Uri(format!(
                "com.epicgames.launcher://apps/{}?action=launch&silent=true",
                encode_uri_segment(app_name)
            )))
        }
        Platform::Xbox => match target.platform_game_id.as_deref() {
            // `<package family>!<app id>`; a package without an app id starts its executable.
            Some(id) if id.contains('!') => {
                if !is_app_user_model_id(id) {
                    return Err(CoreError::Validation("this Xbox game has an invalid application id".into()));
                }
                Ok(LaunchPlan::AppPackage(id.to_owned()))
            }
            _ => plan_executable(target),
        },
        Platform::Gog | Platform::Standalone => plan_executable(target),
    }
}

fn plan_executable(target: &LaunchTarget) -> CoreResult<LaunchPlan> {
    let path =
        target.executable_path.clone().ok_or_else(|| CoreError::Validation("no executable is configured".into()))?;
    if !path.is_file() {
        return Err(CoreError::Validation(format!("{} no longer exists", path.display())));
    }
    let working_dir =
        path.parent().map(PathBuf::from).ok_or_else(|| CoreError::Validation("the executable has no folder".into()))?;
    Ok(LaunchPlan::Executable { path, working_dir })
}

/// `Name_PublisherId!AppId`: only the characters package names allow, so nothing can be
/// smuggled into the shell command.
fn is_app_user_model_id(value: &str) -> bool {
    let mut parts = value.split('!');
    let (Some(family), Some(app), None) = (parts.next(), parts.next(), parts.next()) else { return false };
    let valid = |text: &str| {
        !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    };
    valid(family) && family.contains('_') && valid(app)
}

/// Percent-encodes everything except unreserved URI characters.
fn encode_uri_segment(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (byte as char).to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(platform: Platform, id: Option<&str>, installed: bool) -> LaunchTarget {
        LaunchTarget {
            game_id: 1,
            platform,
            platform_game_id: id.map(str::to_owned),
            installed,
            executable_path: None,
            install_path: None,
        }
    }

    #[test]
    fn xbox_games_start_through_their_package() -> CoreResult<()> {
        let id = "Microsoft.MinecraftDungeons2_8wekyb3d8bbwe!AppMinecraftDungeonsIIShipping";
        assert_eq!(plan_launch(&target(Platform::Xbox, Some(id), true))?, LaunchPlan::AppPackage(id.to_owned()));
        for bad in ["a_b!c d", "nofamily!App", "a_b!App!More", "a_b!App&calc", "!App", "a_b!"] {
            assert!(plan_launch(&target(Platform::Xbox, Some(bad), true)).is_err(), "{bad}");
        }
        // Without an app id the executable is used (and it must exist).
        assert!(plan_launch(&target(Platform::Xbox, Some("Publisher.Game"), true)).is_err());
        Ok(())
    }

    #[test]
    fn gog_games_start_their_executable() {
        assert!(plan_launch(&target(Platform::Gog, Some("2147483047"), true)).is_err(), "no executable configured");
    }

    #[test]
    fn steam_uses_its_uri_scheme() -> CoreResult<()> {
        assert_eq!(
            plan_launch(&target(Platform::Steam, Some("814380"), true))?,
            LaunchPlan::Uri("steam://rungameid/814380".into())
        );
        assert_eq!(
            plan_launch(&target(Platform::Steam, Some("814380"), false))?,
            LaunchPlan::Uri("steam://install/814380".into())
        );
        assert!(plan_launch(&target(Platform::Steam, Some("81;calc"), true)).is_err());
        Ok(())
    }

    #[test]
    fn a_steam_game_that_is_not_installed_only_opens_its_install_page() {
        for bad in ["81;calc", "../x", "", "12 34", "steam://run/1"] {
            assert!(plan_launch(&target(Platform::Steam, Some(bad), false)).is_err(), "{bad}");
        }
        assert!(plan_launch(&target(Platform::Steam, None, false)).is_err());
        // Without an installation there is no executable to start for the other stores.
        assert!(plan_launch(&target(Platform::Gog, Some("1"), false)).is_err());
        assert!(plan_launch(&target(Platform::Standalone, None, false)).is_err());
    }

    #[test]
    fn epic_app_names_are_encoded() -> CoreResult<()> {
        assert_eq!(
            plan_launch(&target(Platform::Epic, Some("Fennec:Base Game"), true))?,
            LaunchPlan::Uri("com.epicgames.launcher://apps/Fennec%3ABase%20Game?action=launch&silent=true".into())
        );
        Ok(())
    }

    #[test]
    fn standalone_requires_an_existing_executable() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let exe = dir.path().join("game.exe");
        let mut standalone = target(Platform::Standalone, None, true);
        standalone.executable_path = Some(exe.clone());
        assert!(plan_launch(&standalone).is_err());
        std::fs::write(&exe, b"MZ")?;
        assert_eq!(
            plan_launch(&standalone)?,
            LaunchPlan::Executable { path: exe, working_dir: dir.path().to_path_buf() }
        );
        Ok(())
    }
}
