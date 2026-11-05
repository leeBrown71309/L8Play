use tauri::State;

use crate::dto::AppInfo;
use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn get_app_info(state: State<'_, AppState>) -> CommandResult<AppInfo> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        database_path: state.db.path().map(|path| path.display().to_string()).unwrap_or_default(),
        schema_version: u32::try_from(state.db.schema_version()?).unwrap_or(u32::MAX),
        ludusavi_available: state.ludusavi.is_available(),
    })
}
