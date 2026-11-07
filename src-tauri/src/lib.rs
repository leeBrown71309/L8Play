//! L8Play desktop shell: wires Tauri (window, commands, plugins) to the `l8-core` services.
//! Business logic lives in `l8-core`; this crate only adapts it to the WebView.

mod artwork_protocol;
mod background;
mod cloud_service;
mod commands;
mod dto;
mod error;
mod events;
mod logging;
mod saves_service;
mod shell;
mod state;

use l8_core::saves::{LudusaviCli, sidecar_path};
use l8_core::settings::SettingsRepository;
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

use crate::state::AppState;

const MAIN_WINDOW: &str = "main";
const START_MINIMIZED_FLAG: &str = "--minimized";
const DATABASE_FILE: &str = "l8.db";

/// Starts the application. Fatal start-up errors are reported and end the process.
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| shell::show_main_window(app)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![START_MINIMIZED_FLAG])))
        .on_window_event(shell::on_window_event)
        .register_uri_scheme_protocol("l8art", |context, request| {
            artwork_protocol::respond(context.app_handle(), &request)
        })
        .setup(|app| {
            let log_guard = logging::init(&app.path().app_log_dir()?)?;
            let database_path = app.path().app_data_dir()?.join(DATABASE_FILE);
            let ludusavi_exe = std::env::current_exe().ok().as_deref().and_then(sidecar_path).unwrap_or_default();
            let ludusavi = LudusaviCli::new(ludusavi_exe, app.path().app_data_dir()?.join("ludusavi"));
            let artwork_root = app.path().app_local_data_dir()?.join("artwork");
            let state = AppState::open(&database_path, ludusavi, app.path().document_dir()?, artwork_root)?;
            app.manage(log_guard);
            app.manage(state);
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "L8Play started");
            shell::setup_tray(app.handle())?;
            let start_in_console = SettingsRepository::new(&app.state::<AppState>().db)
                .load()
                .map(|settings| settings.start_in_console)
                .unwrap_or_default();
            if std::env::args().any(|argument| argument == START_MINIMIZED_FLAG)
                && !start_in_console
                && let Some(window) = app.get_webview_window(MAIN_WINDOW)
            {
                // Launched by Windows at sign-in: stay in the notification area until the user opens it.
                // (With "start in console" the window stays up: the console is the point of the sign-in launch.)
                window.hide()?;
            }
            background::start(app.handle().clone());
            cloud_service::start_worker(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::system::get_app_info,
            commands::library::scan_libraries_now,
            commands::library::get_owned_library_status,
            commands::library::get_steam_key_status,
            commands::library::save_steam_key,
            commands::library::clear_steam_key,
            commands::library::open_steam_api_key_page,
            commands::library::list_games,
            commands::library::get_game,
            commands::library::set_favorite,
            commands::library::get_link_candidates,
            commands::library::link_archived_game,
            commands::library::add_standalone_game,
            commands::library::scan_game_folder,
            commands::library::add_games,
            commands::library::get_running_games,
            commands::library::launch_game,
            commands::library::open_install_folder,
            commands::saves::list_save_locations,
            commands::saves::detect_saves,
            commands::saves::add_save_folder,
            commands::saves::remove_save_location,
            commands::saves::list_snapshots,
            commands::saves::get_save_statuses,
            commands::saves::get_current_save,
            commands::saves::create_snapshot,
            commands::saves::restore_snapshot,
            commands::saves::delete_snapshot,
            commands::saves::verify_snapshot,
            commands::saves::set_snapshot_pinned,
            commands::saves::reveal_snapshot,
            commands::saves::get_snapshot_folder,
            commands::saves::find_saves,
            commands::saves::open_wiki_page,
            commands::import::scan_backup_folder,
            commands::import::replan_backup_game,
            commands::import::import_backup_games,
            commands::saves::rename_snapshot,
            commands::saves::get_restore_targets,
            commands::saves::restore_snapshot_to,
            commands::artwork::get_artwork_key_status,
            commands::artwork::get_artwork_pending,
            commands::artwork::save_artwork_key,
            commands::artwork::clear_artwork_key,
            commands::artwork::verify_artwork_key,
            commands::artwork::get_artwork_selections,
            commands::artwork::find_artwork,
            commands::artwork::apply_artwork,
            commands::artwork::import_artwork,
            commands::artwork::reset_artwork,
            commands::artwork::fetch_missing_artwork,
            commands::artwork::update_game_metadata,
            commands::artwork::open_artwork_key_page,
            commands::cloud::get_cloud_status,
            commands::cloud::get_cloud_quota,
            commands::cloud::save_google_client,
            commands::cloud::clear_google_client,
            commands::cloud::connect_google_drive,
            commands::cloud::disconnect_google_drive,
            commands::cloud::sync_cloud_now,
            commands::cloud::list_sync_operations,
            commands::cloud::retry_sync,
            commands::cloud::list_conflicts,
            commands::cloud::resolve_conflict,
            commands::cloud::download_snapshot,
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("L8Play failed to start: {error}");
        std::process::exit(1);
    }
}
