use l8_core::settings::{AppSettings, SettingsPatch, SettingsRepository};
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::error::{CommandError, CommandErrorCode, CommandResult};
use crate::state::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> CommandResult<AppSettings> {
    Ok(SettingsRepository::new(&state.db).load()?)
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> CommandResult<AppSettings> {
    let settings = SettingsRepository::new(&state.db).update(&patch)?;
    if let Some(enabled) = patch.launch_at_startup {
        apply_launch_at_startup(&app, enabled)?;
    }
    Ok(settings)
}

/// Registers or removes the current-user Windows startup entry (no administrator rights needed).
fn apply_launch_at_startup(app: &AppHandle, enabled: bool) -> CommandResult<()> {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|error| CommandError::new(CommandErrorCode::Io, error.to_string()))
}
