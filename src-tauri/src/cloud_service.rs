//! Cloud sync glue: building the Drive stack from app state, the background worker, and the
//! hooks other features call (a snapshot was created, a snapshot is needed locally).

use std::time::{Duration, Instant};

use l8_core::cloud::{AccountStatus, CloudRepository, SyncReport};
use l8_core::settings::SettingsRepository;
use l8_core::{CoreError, CoreResult};
use tauri::{AppHandle, Manager};

use crate::events;
use crate::state::AppState;

const QUEUE_INTERVAL: Duration = Duration::from_secs(30);
const FULL_SYNC_INTERVAL: Duration = Duration::from_secs(10 * 60);

/// A snapshot was just created: queue its upload (a database write only, no network here). The
/// worker picks it up within seconds.
pub fn enqueue_upload(app: &AppHandle, snapshot_id: &str) {
    let state = app.state::<AppState>();
    let enabled = SettingsRepository::new(&state.db).load().map(|settings| settings.cloud_auto_sync).unwrap_or(false);
    if !enabled {
        return;
    }
    match state.cloud(|sync| sync.queue_upload(snapshot_id)) {
        Ok(true) => events::cloud_changed(app, 0),
        Ok(false) => {}
        Err(error) => tracing::warn!(%error, "could not queue the snapshot for upload"),
    }
}

/// One synchronisation pass. `full` also imports from Drive and looks for conflicts.
pub fn run_pass(app: &AppHandle, full: bool) -> CoreResult<SyncReport> {
    let state = app.state::<AppState>();
    let _guard = state.cloud_lock()?;
    let report = state.cloud(|sync| {
        if full {
            sync.sync_all()
        } else {
            let queue = sync.run_queue()?;
            Ok(SyncReport {
                uploaded: queue.uploaded,
                downloaded: queue.downloaded,
                failed: queue.failed,
                ..SyncReport::default()
            })
        }
    })?;
    if report.uploaded + report.downloaded + report.imported + report.failed + report.new_conflicts > 0 {
        events::cloud_changed(app, report.new_conflicts);
    }
    Ok(report)
}

/// Runs the queue once in the background (Drive deletions the user just asked for should not wait
/// for the next 30 s tick). Does nothing while signed out or offline; the worker retries later.
pub fn kick_queue(app: &AppHandle) {
    let app = app.clone();
    let spawned = std::thread::Builder::new().name("l8-cloud-kick".into()).spawn(move || {
        if !connected(&app.state::<AppState>()) {
            return;
        }
        match run_pass(&app, false) {
            Ok(_) => events::cloud_changed(&app, 0),
            Err(error) => tracing::info!(%error, "cloud queue pass skipped"),
        }
    });
    if let Err(error) = spawned {
        tracing::warn!(%error, "could not start a cloud queue pass");
    }
}

fn connected(state: &AppState) -> bool {
    CloudRepository::new(&state.db)
        .account()
        .ok()
        .flatten()
        .is_some_and(|account| account.status == AccountStatus::Connected)
}

/// Background worker: the queue every 30 s, a full pass (import, conflicts) every 10 min.
pub fn start_worker(app: AppHandle) {
    let spawned = std::thread::Builder::new().name("l8-cloud".into()).spawn(move || {
        let state = app.state::<AppState>();
        if let Err(error) = CloudRepository::new(&state.db).requeue_interrupted() {
            tracing::warn!(%error, "could not requeue interrupted cloud operations");
        }
        let mut last_full: Option<Instant> = None;
        loop {
            std::thread::sleep(QUEUE_INTERVAL);
            let enabled = SettingsRepository::new(&state.db).load().map(|s| s.cloud_auto_sync).unwrap_or(false);
            if !enabled || !connected(&state) {
                continue;
            }
            let full = last_full.is_none_or(|at| at.elapsed() >= FULL_SYNC_INTERVAL);
            match run_pass(&app, full) {
                Ok(_) => {
                    if full {
                        last_full = Some(Instant::now());
                    }
                }
                Err(CoreError::Auth(_)) => {
                    tracing::warn!("Google Drive needs a new sign-in; cloud sync is paused");
                    events::cloud_changed(&app, 0);
                    crate::shell::notify_attention(&app, crate::shell::Notice::CloudSignInNeeded);
                }
                // Offline or rate limited: the next tick tries again; operations back off on their own.
                Err(error) => tracing::info!(%error, "cloud pass skipped"),
            }
        }
    });
    if let Err(error) = spawned {
        tracing::error!(%error, "cloud worker could not start; use \"Sync now\"");
    }
}
