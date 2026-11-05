//! Steam discovery: registry → `libraryfolders.vdf` → `appmanifest_*.acf` (via `steamlocate`),
//! plus last-played times and playtime from each local user's `localconfig.vdf`.

use std::collections::HashMap;
use std::path::Path;

use keyvalues_parser::{Obj, Value};
use steamlocate::app::StateFlag;

use crate::clock::unix_to_rfc3339;
use crate::error::CoreResult;
use crate::platforms::{DiscoveredGame, Discovery, LibraryProvider, Platform};

/// Steam entries that are runtimes or tools, not games.
pub(crate) const NON_GAME_APP_IDS: &[u32] = &[
    228_980,   // Steamworks Common Redistributables
    1_070_560, // Steam Linux Runtime 1.0 (scout)
    1_391_110, // Steam Linux Runtime 2.0 (soldier)
    1_628_350, // Steam Linux Runtime 3.0 (sniper)
    1_493_710, // Proton Experimental
    1_826_330, // Proton EasyAntiCheat Runtime
    1_161_040, // Proton BattlEye Runtime
    250_820,   // SteamVR
];

#[derive(Debug, Default)]
pub struct SteamProvider;

impl LibraryProvider for SteamProvider {
    fn platform(&self) -> Platform {
        Platform::Steam
    }

    fn discover(&self) -> CoreResult<Option<Discovery>> {
        let steam_dir = match steamlocate::SteamDir::locate() {
            Ok(dir) => dir,
            Err(error) => {
                tracing::info!(%error, "Steam not found; skipping Steam discovery");
                return Ok(None);
            }
        };
        let mut discovery = Discovery::default();
        let activity = read_local_activity(steam_dir.path(), &mut discovery.warnings);

        let libraries = match steam_dir.libraries() {
            Ok(libraries) => libraries,
            Err(error) => {
                // Steam exists but its library list is unreadable: report, and do not pretend
                // the library is empty (that would mark every game as uninstalled).
                tracing::warn!(%error, "Steam libraries could not be read");
                return Err(std::io::Error::other(format!("Steam libraries could not be read: {error}")).into());
            }
        };

        for library in libraries {
            let library = match library {
                Ok(library) => library,
                Err(error) => {
                    discovery.warnings.push(format!("Steam library skipped: {error}"));
                    continue;
                }
            };
            discovery.library_paths.push(library.path().to_path_buf());
            for app in library.apps() {
                let app = match app {
                    Ok(app) => app,
                    Err(error) => {
                        discovery.warnings.push(format!("Steam app manifest skipped: {error}"));
                        continue;
                    }
                };
                if NON_GAME_APP_IDS.contains(&app.app_id) || !is_fully_installed(&app) {
                    continue;
                }
                let install_path = library.resolve_app_dir(&app);
                let known = activity.get(&app.app_id);
                discovery.games.push(DiscoveredGame {
                    platform_game_id: app.app_id.to_string(),
                    title: app.name.clone().unwrap_or_else(|| app.install_dir.clone()),
                    install_path,
                    executable_path: None,
                    size_bytes: app.size_on_disk,
                    last_played_at: known.and_then(|entry| entry.last_played.clone()),
                    playtime_seconds: known.map(|entry| entry.playtime_minutes * 60),
                });
            }
        }
        Ok(Some(discovery))
    }
}

fn is_fully_installed(app: &steamlocate::App) -> bool {
    app.state_flags.is_none_or(|flags| flags.flags().any(|flag| flag == StateFlag::FullyInstalled))
}

/// What Steam itself remembers about a game for the users of this PC.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct SteamActivity {
    /// Unix seconds.
    last_played: Option<i64>,
    playtime_minutes: i64,
}

impl SteamActivity {
    fn last_played_rfc3339(self) -> Option<String> {
        self.last_played.and_then(unix_to_rfc3339)
    }
}

/// Merges `userdata/<account>/config/localconfig.vdf` of every local account (latest play, max playtime).
fn read_local_activity(steam_path: &Path, warnings: &mut Vec<String>) -> HashMap<u32, KnownActivity> {
    let mut merged: HashMap<u32, SteamActivity> = HashMap::new();
    let Ok(accounts) = std::fs::read_dir(steam_path.join("userdata")) else { return HashMap::new() };
    for account in accounts.flatten() {
        let config = account.path().join("config").join("localconfig.vdf");
        let Ok(text) = std::fs::read_to_string(&config) else { continue };
        match parse_local_config(&text) {
            Ok(per_app) => {
                for (app_id, activity) in per_app {
                    let entry = merged.entry(app_id).or_default();
                    entry.last_played = entry.last_played.max(activity.last_played);
                    entry.playtime_minutes = entry.playtime_minutes.max(activity.playtime_minutes);
                }
            }
            Err(error) => warnings.push(format!("Steam local config skipped ({}): {error}", config.display())),
        }
    }
    merged
        .into_iter()
        .map(|(app_id, activity)| {
            (
                app_id,
                KnownActivity {
                    last_played: activity.last_played_rfc3339(),
                    playtime_minutes: activity.playtime_minutes,
                },
            )
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
struct KnownActivity {
    last_played: Option<String>,
    playtime_minutes: i64,
}

/// Reads `UserLocalConfigStore/Software/Valve/Steam/apps/<appid>/{LastPlayed, Playtime}`.
fn parse_local_config(text: &str) -> Result<HashMap<u32, SteamActivity>, String> {
    let vdf = keyvalues_parser::parse(text).map_err(|error| error.to_string())?;
    let root = vdf.value.get_obj().ok_or("root is not an object")?;
    let apps = ["Software", "Valve", "Steam", "apps"]
        .iter()
        .try_fold(root, |obj, key| child_obj(obj, key))
        .ok_or("no apps section")?;

    let mut result = HashMap::new();
    for (key, values) in &apps.0 {
        let (Ok(app_id), Some(app)) = (key.parse::<u32>(), values.first().and_then(Value::get_obj)) else { continue };
        let last_played =
            child_str(app, "LastPlayed").and_then(|value| value.parse::<i64>().ok()).filter(|value| *value > 0);
        let playtime_minutes = child_str(app, "Playtime").and_then(|value| value.parse::<i64>().ok()).unwrap_or(0);
        if last_played.is_some() || playtime_minutes > 0 {
            result.insert(app_id, SteamActivity { last_played, playtime_minutes });
        }
    }
    Ok(result)
}

/// VDF keys are case-insensitive in practice (`Software` / `software`).
fn child<'a>(obj: &'a Obj<'a>, key: &str) -> Option<&'a Value<'a>> {
    obj.0.iter().find(|(name, _)| name.eq_ignore_ascii_case(key)).and_then(|(_, values)| values.first())
}

fn child_obj<'a>(obj: &'a Obj<'a>, key: &str) -> Option<&'a Obj<'a>> {
    child(obj, key).and_then(Value::get_obj)
}

fn child_str<'a>(obj: &'a Obj<'a>, key: &str) -> Option<&'a str> {
    child(obj, key).and_then(Value::get_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCAL_CONFIG: &str = r#"
"UserLocalConfigStore"
{
    "software"
    {
        "valve"
        {
            "Steam"
            {
                "apps"
                {
                    "814380"
                    {
                        "LastPlayed"        "1793826720"
                        "Playtime"          "3842"
                    }
                    "582010"
                    {
                        "Playtime"          "24720"
                    }
                    "228980"
                    {
                        "cloud" { "last_sync_state" "synchronized" }
                    }
                }
            }
        }
    }
}
"#;

    #[test]
    fn reads_last_played_and_playtime() -> Result<(), String> {
        let apps = parse_local_config(LOCAL_CONFIG)?;
        assert_eq!(
            apps.get(&814_380),
            Some(&SteamActivity { last_played: Some(1_793_826_720), playtime_minutes: 3842 })
        );
        assert_eq!(apps.get(&582_010), Some(&SteamActivity { last_played: None, playtime_minutes: 24_720 }));
        assert!(!apps.contains_key(&228_980), "apps without activity are skipped");
        Ok(())
    }

    #[test]
    fn rejects_unrelated_files() {
        assert!(parse_local_config("\"Other\" { \"x\" \"1\" }").is_err());
    }
}
