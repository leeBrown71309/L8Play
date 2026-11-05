//! Background work started with the app: the initial library scan, then the session monitor.

use std::collections::HashSet;
use std::time::Duration;

use l8_core::launcher::{ProcessMonitor, SessionChange};
use l8_core::library::LibraryRepository;
use tauri::{AppHandle, Manager};

use l8_core::settings::SettingsRepository;
use l8_core::snapshots::SnapshotKind;

use crate::commands::library::run_scan;
use crate::dto::SessionEvent;
use crate::events;
use crate::saves_service;
use crate::state::AppState;

/// Lets the game finish flushing its save files before they are copied.
const EXIT_SNAPSHOT_DELAY: Duration = Duration::from_secs(5);

/// One process sample every 3 s: responsive enough for start/exit detection, negligible CPU.
const SAMPLE_INTERVAL: Duration = Duration::from_secs(3);

/// Fills in cover, hero and logo for Epic and standalone games once a SteamGridDB key is saved.
fn fetch_missing_artwork(app: &AppHandle, state: &AppState) {
    let progressive = state.artwork().fetch_missing_with(|game, outcome| {
        if outcome.fetched > 0 {
            events::artwork_fetched(app, game.id);
        }
    });
    match progressive {
        Ok(report) => {
            if report.fetched > 0 {
                events::library_changed(app);
            }
            tracing::info!(
                fetched = report.fetched,
                not_found = report.not_found,
                failed = report.failed,
                "artwork lookup finished"
            );
        }
        Err(l8_core::CoreError::Auth(_)) => tracing::warn!("artwork lookup skipped: the saved API key was refused"),
        Err(error) => tracing::warn!(%error, "artwork lookup failed"),
    }
}

pub fn start(app: AppHandle) {
    let spawned = std::thread::Builder::new().name("l8-background".into()).spawn(move || run(&app));
    if let Err(error) = spawned {
        tracing::error!(%error, "background worker could not start; scans and sessions must be triggered manually");
    }
}

fn run(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Err(error) = LibraryRepository::new(&state.db).close_dangling_sessions() {
        tracing::warn!(%error, "could not close sessions left open by a previous run");
    }
    match run_scan(&state) {
        Ok(_) => events::library_changed(app),
        Err(error) => tracing::warn!(%error, "startup library scan failed"),
    }
    saves_service::detect_new_games(app, &state);
    fetch_missing_artwork(app, &state);

    let mut monitor = match ProcessMonitor::new() {
        Ok(monitor) => monitor,
        Err(error) => {
            tracing::error!(%error, "process monitor unavailable; play sessions will not be recorded");
            return;
        }
    };
    loop {
        std::thread::sleep(SAMPLE_INTERVAL);
        if let Err(error) = sample(app, &state, &mut monitor) {
            tracing::warn!(%error, "session sample failed");
        }
    }
}

fn sample(app: &AppHandle, state: &AppState, monitor: &mut ProcessMonitor) -> l8_core::CoreResult<()> {
    let repository = LibraryRepository::new(&state.db);
    let watched = repository.watched_games()?;
    let running: HashSet<i64> = monitor.running_games(&watched);
    let changes = state.tracker()?.update(&repository, &running)?;
    for change in changes {
        match change {
            SessionChange::Started { game_id } => {
                tracing::info!(game_id, "game started");
                events::game_session(app, SessionEvent { game_id, running: true, duration_seconds: None });
            }
            SessionChange::Ended { game_id, duration_seconds } => {
                tracing::info!(game_id, duration_seconds, "game exited");
                events::game_session(
                    app,
                    SessionEvent { game_id, running: false, duration_seconds: Some(duration_seconds) },
                );
                events::library_changed(app);
                schedule_exit_snapshot(app, state, game_id);
            }
        }
    }
    Ok(())
}

/// After a game closes: snapshot its save (if enabled) on a separate thread, after a short delay.
fn schedule_exit_snapshot(app: &AppHandle, state: &AppState, game_id: i64) {
    match SettingsRepository::new(&state.db).load() {
        Ok(settings) if settings.snapshot_on_exit => {}
        Ok(_) => return,
        Err(error) => {
            tracing::warn!(%error, "could not read settings; skipping the exit snapshot");
            return;
        }
    }
    let app = app.clone();
    let spawned = std::thread::Builder::new().name("l8-exit-snapshot".into()).spawn(move || {
        std::thread::sleep(EXIT_SNAPSHOT_DELAY);
        let state = app.state::<AppState>();
        saves_service::automatic_snapshot_and_notify(&app, &state, game_id, SnapshotKind::AutoExit);
    });
    if let Err(error) = spawned {
        tracing::error!(%error, game_id, "could not start the exit snapshot");
    }
}
