//! Application preferences persisted in the `settings` table (one JSON value per key).

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::Database;
use crate::error::{CoreError, CoreResult};

const KEY_THEME: &str = "appearance.theme";
const KEY_LOCALE: &str = "appearance.locale";
const KEY_NAV_EXPANDED: &str = "shell.navExpanded";
const KEY_SNAPSHOT_ON_EXIT: &str = "backup.snapshotOnExit";
const KEY_SNAPSHOT_BEFORE_LAUNCH: &str = "backup.snapshotBeforeLaunch";
const KEY_KEEP_AUTOMATIC: &str = "backup.keepAutomatic";
const KEY_KEEP_CLOUD: &str = "backup.keepCloud";
const KEY_SNAPSHOT_FOLDER: &str = "backup.snapshotFolder";
const KEY_CLOUD_AUTO_SYNC: &str = "cloud.autoSync";
const KEY_CLOSE_TO_TRAY: &str = "shell.closeToTray";
const KEY_LAUNCH_AT_STARTUP: &str = "shell.launchAtStartup";
const KEY_DESKTOP_NOTIFICATIONS: &str = "shell.desktopNotifications";
const KEY_ONBOARDING_DONE: &str = "shell.onboardingDone";
const KEY_SHOW_OWNED_NOT_INSTALLED: &str = "libraries.showOwnedNotInstalled";
const KEY_UPDATES_AUTO_CHECK: &str = "updates.autoCheck";

/// Bounds for automatic-snapshot retention per game.
pub const KEEP_AUTOMATIC_RANGE: std::ops::RangeInclusive<u32> = 1..=100;

/// Bounds for the Google Drive retention; `0` means "same as this PC".
pub const KEEP_CLOUD_RANGE: std::ops::RangeInclusive<u32> = 0..=100;

/// Colour theme from the design system: Ember (dark, default) or Rosé (light).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Theme {
    Ember,
    Rose,
}

/// Interface language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Locale {
    En,
    Fr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppSettings {
    pub theme: Theme,
    pub locale: Locale,
    pub nav_expanded: bool,
    /// Take a snapshot automatically when a game closes.
    pub snapshot_on_exit: bool,
    /// Take a snapshot automatically before L8Play launches a game.
    pub snapshot_before_launch: bool,
    /// Automatic snapshots kept per game (manual, pinned and safety snapshots are never pruned).
    pub keep_automatic: u32,
    /// Automatic snapshots kept per game in Google Drive; `0` means the same as `keepAutomatic`.
    /// Drive never keeps fewer than this PC (a lower value behaves as `keepAutomatic`).
    pub keep_cloud: u32,
    /// Custom snapshot folder; empty means the default (Documents\L8Play\Snapshots).
    pub snapshot_folder: String,
    /// Upload snapshots to Google Drive in the background while an account is connected.
    pub cloud_auto_sync: bool,
    /// Closing the window keeps L8Play running in the notification area (automatic snapshots keep working).
    pub close_to_tray: bool,
    /// Start with Windows, minimized to the notification area.
    pub launch_at_startup: bool,
    /// Windows notifications for problems that need attention while the window is hidden.
    pub desktop_notifications: bool,
    /// The first-run walkthrough was completed or skipped.
    pub onboarding_done: bool,
    /// List the Steam games the account owns but has not installed (read from the public Steam profile).
    pub show_owned_not_installed: bool,
    /// Check GitHub Releases for a newer version in the background; manual checks always work.
    pub updates_auto_check: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Ember,
            locale: Locale::En,
            nav_expanded: false,
            snapshot_on_exit: true,
            snapshot_before_launch: false,
            keep_automatic: 10,
            keep_cloud: 0,
            snapshot_folder: String::new(),
            cloud_auto_sync: true,
            close_to_tray: true,
            launch_at_startup: false,
            desktop_notifications: true,
            onboarding_done: false,
            show_owned_not_installed: true,
            updates_auto_check: true,
        }
    }
}

/// Partial update sent by the UI; absent fields keep their stored value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct SettingsPatch {
    pub theme: Option<Theme>,
    pub locale: Option<Locale>,
    pub nav_expanded: Option<bool>,
    pub snapshot_on_exit: Option<bool>,
    pub snapshot_before_launch: Option<bool>,
    pub keep_automatic: Option<u32>,
    pub keep_cloud: Option<u32>,
    pub snapshot_folder: Option<String>,
    pub cloud_auto_sync: Option<bool>,
    pub close_to_tray: Option<bool>,
    pub launch_at_startup: Option<bool>,
    pub desktop_notifications: Option<bool>,
    pub onboarding_done: Option<bool>,
    pub show_owned_not_installed: Option<bool>,
    pub updates_auto_check: Option<bool>,
}

/// Reads and writes [`AppSettings`].
pub struct SettingsRepository<'db> {
    db: &'db Database,
}

impl<'db> SettingsRepository<'db> {
    pub fn new(db: &'db Database) -> Self {
        Self { db }
    }

    /// Stored settings layered over defaults. A corrupt value is logged and replaced by its default,
    /// so one bad row never blocks the app from starting.
    pub fn load(&self) -> CoreResult<AppSettings> {
        let defaults = AppSettings::default();
        Ok(AppSettings {
            theme: self.read_or(KEY_THEME, defaults.theme)?,
            locale: self.read_or(KEY_LOCALE, defaults.locale)?,
            nav_expanded: self.read_or(KEY_NAV_EXPANDED, defaults.nav_expanded)?,
            snapshot_on_exit: self.read_or(KEY_SNAPSHOT_ON_EXIT, defaults.snapshot_on_exit)?,
            snapshot_before_launch: self.read_or(KEY_SNAPSHOT_BEFORE_LAUNCH, defaults.snapshot_before_launch)?,
            keep_automatic: self
                .read_or(KEY_KEEP_AUTOMATIC, defaults.keep_automatic)?
                .clamp(*KEEP_AUTOMATIC_RANGE.start(), *KEEP_AUTOMATIC_RANGE.end()),
            keep_cloud: self
                .read_or(KEY_KEEP_CLOUD, defaults.keep_cloud)?
                .clamp(*KEEP_CLOUD_RANGE.start(), *KEEP_CLOUD_RANGE.end()),
            snapshot_folder: self.read_or(KEY_SNAPSHOT_FOLDER, defaults.snapshot_folder)?,
            cloud_auto_sync: self.read_or(KEY_CLOUD_AUTO_SYNC, defaults.cloud_auto_sync)?,
            close_to_tray: self.read_or(KEY_CLOSE_TO_TRAY, defaults.close_to_tray)?,
            launch_at_startup: self.read_or(KEY_LAUNCH_AT_STARTUP, defaults.launch_at_startup)?,
            desktop_notifications: self.read_or(KEY_DESKTOP_NOTIFICATIONS, defaults.desktop_notifications)?,
            onboarding_done: self.read_or(KEY_ONBOARDING_DONE, defaults.onboarding_done)?,
            show_owned_not_installed: self.read_or(KEY_SHOW_OWNED_NOT_INSTALLED, defaults.show_owned_not_installed)?,
            updates_auto_check: self.read_or(KEY_UPDATES_AUTO_CHECK, defaults.updates_auto_check)?,
        })
    }

    /// Applies a patch atomically and returns the resulting settings.
    pub fn update(&self, patch: &SettingsPatch) -> CoreResult<AppSettings> {
        if let Some(keep) = patch.keep_automatic
            && !KEEP_AUTOMATIC_RANGE.contains(&keep)
        {
            return Err(CoreError::Validation(format!(
                "keep between {} and {} automatic snapshots",
                KEEP_AUTOMATIC_RANGE.start(),
                KEEP_AUTOMATIC_RANGE.end()
            )));
        }
        if let Some(keep) = patch.keep_cloud
            && !KEEP_CLOUD_RANGE.contains(&keep)
        {
            return Err(CoreError::Validation(format!(
                "keep between {} and {} snapshots in Google Drive (0 = same as this PC)",
                KEEP_CLOUD_RANGE.start(),
                KEEP_CLOUD_RANGE.end()
            )));
        }
        if let Some(folder) = patch.snapshot_folder.as_deref().filter(|folder| !folder.is_empty())
            && !std::path::Path::new(folder).is_absolute()
        {
            return Err(CoreError::Validation("the snapshot folder must be an absolute path".into()));
        }
        {
            let mut connection = self.db.connection()?;
            let transaction = connection.transaction()?;
            if let Some(theme) = patch.theme {
                write(&transaction, KEY_THEME, &theme)?;
            }
            if let Some(locale) = patch.locale {
                write(&transaction, KEY_LOCALE, &locale)?;
            }
            if let Some(nav_expanded) = patch.nav_expanded {
                write(&transaction, KEY_NAV_EXPANDED, &nav_expanded)?;
            }
            if let Some(value) = patch.snapshot_on_exit {
                write(&transaction, KEY_SNAPSHOT_ON_EXIT, &value)?;
            }
            if let Some(value) = patch.snapshot_before_launch {
                write(&transaction, KEY_SNAPSHOT_BEFORE_LAUNCH, &value)?;
            }
            if let Some(value) = patch.keep_automatic {
                write(&transaction, KEY_KEEP_AUTOMATIC, &value)?;
            }
            if let Some(value) = patch.keep_cloud {
                write(&transaction, KEY_KEEP_CLOUD, &value)?;
            }
            if let Some(value) = &patch.snapshot_folder {
                write(&transaction, KEY_SNAPSHOT_FOLDER, value)?;
            }
            if let Some(value) = patch.cloud_auto_sync {
                write(&transaction, KEY_CLOUD_AUTO_SYNC, &value)?;
            }
            if let Some(value) = patch.close_to_tray {
                write(&transaction, KEY_CLOSE_TO_TRAY, &value)?;
            }
            if let Some(value) = patch.launch_at_startup {
                write(&transaction, KEY_LAUNCH_AT_STARTUP, &value)?;
            }
            if let Some(value) = patch.desktop_notifications {
                write(&transaction, KEY_DESKTOP_NOTIFICATIONS, &value)?;
            }
            if let Some(value) = patch.onboarding_done {
                write(&transaction, KEY_ONBOARDING_DONE, &value)?;
            }
            if let Some(value) = patch.show_owned_not_installed {
                write(&transaction, KEY_SHOW_OWNED_NOT_INSTALLED, &value)?;
            }
            if let Some(value) = patch.updates_auto_check {
                write(&transaction, KEY_UPDATES_AUTO_CHECK, &value)?;
            }
            transaction.commit()?;
        }
        self.load()
    }

    fn read_or<T: for<'de> Deserialize<'de>>(&self, key: &str, default: T) -> CoreResult<T> {
        let raw: Option<String> = self
            .db
            .connection()?
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| row.get(0))
            .optional()?;
        let Some(raw) = raw else { return Ok(default) };
        match serde_json::from_str(&raw) {
            Ok(value) => Ok(value),
            Err(error) => {
                tracing::warn!(key, %error, "ignoring unreadable setting, using default");
                Ok(default)
            }
        }
    }
}

fn write<T: Serialize>(connection: &rusqlite::Connection, key: &str, value: &T) -> CoreResult<()> {
    let json = serde_json::to_string(value).map_err(|source| CoreError::Serialization { key: key.into(), source })?;
    connection.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, json],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_nothing_is_stored() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        assert_eq!(SettingsRepository::new(&db).load()?, AppSettings::default());
        Ok(())
    }

    #[test]
    fn patch_updates_only_given_fields() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let repo = SettingsRepository::new(&db);
        let updated = repo.update(&SettingsPatch { locale: Some(Locale::Fr), ..Default::default() })?;
        assert_eq!(updated, AppSettings { locale: Locale::Fr, ..AppSettings::default() });

        let updated =
            repo.update(&SettingsPatch { theme: Some(Theme::Rose), nav_expanded: Some(true), ..Default::default() })?;
        assert_eq!(
            updated,
            AppSettings { theme: Theme::Rose, locale: Locale::Fr, nav_expanded: true, ..AppSettings::default() }
        );
        assert!(repo.update(&SettingsPatch { keep_automatic: Some(0), ..Default::default() }).is_err());
        assert!(
            repo.update(&SettingsPatch { snapshot_folder: Some(r"relative\dir".into()), ..Default::default() })
                .is_err()
        );
        assert_eq!(repo.update(&SettingsPatch { keep_automatic: Some(20), ..Default::default() })?.keep_automatic, 20);
        assert!(repo.update(&SettingsPatch { keep_cloud: Some(101), ..Default::default() }).is_err());
        let updated = repo.update(&SettingsPatch { keep_cloud: Some(30), ..Default::default() })?;
        assert_eq!((updated.keep_automatic, updated.keep_cloud), (20, 30));
        assert_eq!(repo.update(&SettingsPatch { keep_cloud: Some(0), ..Default::default() })?.keep_cloud, 0);
        Ok(())
    }

    #[test]
    fn owned_games_listing_defaults_on_and_can_be_switched_off() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let repo = SettingsRepository::new(&db);
        assert!(repo.load()?.show_owned_not_installed);
        let off = repo.update(&SettingsPatch { show_owned_not_installed: Some(false), ..Default::default() })?;
        assert!(!off.show_owned_not_installed);
        assert!(!repo.load()?.show_owned_not_installed);
        Ok(())
    }

    #[test]
    fn update_checks_default_on_and_can_be_switched_off() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let repo = SettingsRepository::new(&db);
        assert!(repo.load()?.updates_auto_check);
        let off = repo.update(&SettingsPatch { updates_auto_check: Some(false), ..Default::default() })?;
        assert!(!off.updates_auto_check);
        assert!(!repo.load()?.updates_auto_check);
        let on = repo.update(&SettingsPatch { updates_auto_check: Some(true), ..Default::default() })?;
        assert!(on.updates_auto_check);
        Ok(())
    }

    #[test]
    fn corrupt_value_falls_back_to_default() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        db.connection()?
            .execute("INSERT INTO settings (key, value, updated_at) VALUES (?1, 'not json', 'now')", [KEY_THEME])?;
        assert_eq!(SettingsRepository::new(&db).load()?.theme, Theme::Ember);
        Ok(())
    }

    #[test]
    fn serializes_with_frontend_casing() -> Result<(), serde_json::Error> {
        let json = serde_json::to_string(&AppSettings::default())?;
        assert!(json.starts_with(r#"{"theme":"ember","locale":"en","navExpanded":false,"snapshotOnExit":true"#));
        assert!(json.ends_with(r#""showOwnedNotInstalled":true,"updatesAutoCheck":true}"#));
        Ok(())
    }
}
