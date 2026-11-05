//! Window-level behaviour: the notification-area icon, "close to tray", and Windows notifications
//! for things that need attention while the window is hidden. The few native strings here follow
//! the interface language chosen in Settings (the webview's own text is translated with Lingui).

use l8_core::settings::{AppSettings, Locale, SettingsRepository};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Window, WindowEvent};
use tauri_plugin_notification::NotificationExt;

use crate::cloud_service;
use crate::state::AppState;

const MAIN_WINDOW: &str = "main";
const MENU_OPEN: &str = "open";
const MENU_SYNC: &str = "sync";
const MENU_QUIT: &str = "quit";

/// What a native message is about; the text depends on the interface language.
#[derive(Debug, Clone, Copy)]
pub enum Notice {
    ConflictFound,
    CloudSignInNeeded,
    SnapshotFailed,
}

fn settings(app: &AppHandle) -> AppSettings {
    let state = app.state::<AppState>();
    SettingsRepository::new(&state.db).load().unwrap_or_default()
}

fn menu_text(locale: Locale, id: &str) -> &'static str {
    match (locale, id) {
        (Locale::Fr, MENU_OPEN) => "Ouvrir L8Play",
        (Locale::Fr, MENU_SYNC) => "Synchroniser maintenant",
        (Locale::Fr, _) => "Quitter L8Play",
        (Locale::En, MENU_OPEN) => "Open L8Play",
        (Locale::En, MENU_SYNC) => "Sync now",
        (Locale::En, _) => "Quit L8Play",
    }
}

fn notice_text(locale: Locale, notice: Notice) -> (&'static str, &'static str) {
    match (locale, notice) {
        (Locale::Fr, Notice::ConflictFound) => {
            ("Deux sauvegardes différentes", "Un jeu a été joué sur deux appareils. Ouvrez L8Play pour choisir.")
        }
        (Locale::Fr, Notice::CloudSignInNeeded) => (
            "Google Drive : connexion requise",
            "La synchronisation est en pause tant que vous ne vous reconnectez pas.",
        ),
        (Locale::Fr, Notice::SnapshotFailed) => {
            ("Sauvegarde automatique échouée", "Une sauvegarde n’a pas pu être copiée. Consultez le journal.")
        }
        (Locale::En, Notice::ConflictFound) => {
            ("Two different saves", "A game was played on two devices. Open L8Play to choose which to keep.")
        }
        (Locale::En, Notice::CloudSignInNeeded) => {
            ("Google Drive needs you to sign in", "Syncing is paused until you sign in again.")
        }
        (Locale::En, Notice::SnapshotFailed) => {
            ("Automatic snapshot failed", "A save could not be copied. Check the log file for details.")
        }
    }
}

/// Shows a Windows notification, but only when the user enabled them and is not already looking at
/// the window.
pub fn notify_attention(app: &AppHandle, notice: Notice) {
    let settings = settings(app);
    if !settings.desktop_notifications {
        return;
    }
    let window_in_use = app
        .get_webview_window(MAIN_WINDOW)
        .is_some_and(|window| window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false));
    if window_in_use {
        return;
    }
    let (title, body) = notice_text(settings.locale, notice);
    if let Err(error) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!(%error, "could not show a notification");
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        // Each step may fail harmlessly (e.g. already visible); the goal is only a focused window.
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Creates the notification-area icon with its menu.
pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let locale = settings(app).locale;
    let open = MenuItem::with_id(app, MENU_OPEN, menu_text(locale, MENU_OPEN), true, None::<&str>)?;
    let sync = MenuItem::with_id(app, MENU_SYNC, menu_text(locale, MENU_SYNC), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, menu_text(locale, MENU_QUIT), true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &sync, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("L8Play")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_OPEN => show_main_window(app),
            MENU_SYNC => {
                let app = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    if let Err(error) = cloud_service::run_pass(&app, true) {
                        tracing::info!(%error, "tray sync skipped");
                    }
                });
            }
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Closing the window hides it when "close to tray" is on, so background snapshots keep working.
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != MAIN_WINDOW {
        return;
    }
    if let WindowEvent::CloseRequested { api, .. } = event
        && settings(window.app_handle()).close_to_tray
    {
        api.prevent_close();
        if let Err(error) = window.hide() {
            tracing::warn!(%error, "could not hide the window to the tray");
        }
    }
}
