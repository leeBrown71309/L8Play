//! Google Drive commands: OAuth client setup, sign-in, status, sync and conflict resolution.
//! The client secret and tokens are written to the Credential Manager and never returned.

use l8_core::artwork::SecretStore;
use l8_core::cloud::oauth::{self, SECRET_CLIENT_ID, SECRET_CLIENT_SECRET, SECRET_REFRESH_TOKEN};
use l8_core::cloud::{
    AccountStatus, AuthorizationFlow, ClientSource, CloudAccountInfo, CloudQuota, CloudRepository, CloudStatus,
    ConflictInfo, ConflictResolution, RemoteStore, SyncOperationInfo, SyncReport, current_client,
};
use l8_core::snapshots::{SnapshotEngine, SnapshotRepository};
use l8_core::{CoreError, CoreResult};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::cloud_service;
use crate::error::{CommandError, CommandErrorCode, CommandResult};
use crate::events;
use crate::state::AppState;

/// Runs blocking cloud work (network, disk) off the async runtime.
async fn blocking<T, F>(app: &AppHandle, work: F) -> CommandResult<T>
where
    T: Send + 'static,
    F: FnOnce(&AppState) -> CoreResult<T> + Send + 'static,
{
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || work(&app.state::<AppState>()))
        .await
        .map_err(|error| CommandError::new(CommandErrorCode::Internal, error.to_string()))?
        .map_err(CommandError::from)
}

fn client_source(state: &AppState) -> CoreResult<ClientSource> {
    Ok(current_client(state.secrets())?.map_or(ClientSource::None, |client| client.source))
}

fn status(state: &AppState) -> CoreResult<CloudStatus> {
    let repository = CloudRepository::new(&state.db);
    let account = repository.account()?;
    let (queued, running, failed) = match &account {
        Some(account) => repository.counts(account.id)?,
        None => (0, 0, 0),
    };
    Ok(CloudStatus {
        client_source: client_source(state)?,
        account: account.as_ref().map(CloudAccountInfo::from),
        quota: None,
        queued,
        running,
        failed,
        open_conflicts: u32::try_from(repository.open_conflicts()?.len()).unwrap_or(u32::MAX),
        last_sync_at: repository.last_sync()?,
    })
}

#[tauri::command]
pub async fn get_cloud_status(app: AppHandle) -> CommandResult<CloudStatus> {
    blocking(&app, status).await
}

/// Storage figures from Drive (a network call, so separate from the quick status).
#[tauri::command]
pub async fn get_cloud_quota(app: AppHandle) -> CommandResult<CloudQuota> {
    blocking(&app, |state| {
        let quota = state.drive(|drive| drive.quota())?;
        Ok(CloudQuota { used_bytes: quota.used, limit_bytes: quota.limit })
    })
    .await
}

/// Saves the user's own Google OAuth client (advanced option; it replaces the built-in one).
#[tauri::command]
pub async fn save_google_client(app: AppHandle, client_id: String, client_secret: String) -> CommandResult<()> {
    blocking(&app, move |state| {
        if CloudRepository::new(&state.db).account()?.is_some() {
            return Err(CoreError::Validation("disconnect Google Drive before changing the client".into()));
        }
        let id = client_id.trim();
        let secret = client_secret.trim();
        if !id.ends_with(".apps.googleusercontent.com") || id.len() > 200 || id.contains(char::is_whitespace) {
            return Err(CoreError::Validation("the client id ends with .apps.googleusercontent.com".into()));
        }
        if secret.len() < 8 || secret.len() > 200 || secret.contains(char::is_whitespace) {
            return Err(CoreError::Validation("the client secret looks invalid".into()));
        }
        state.secrets().set(SECRET_CLIENT_ID, id)?;
        state.secrets().set(SECRET_CLIENT_SECRET, secret)
    })
    .await
}

#[tauri::command]
pub async fn clear_google_client(app: AppHandle) -> CommandResult<()> {
    blocking(&app, |state| {
        if CloudRepository::new(&state.db).account()?.is_some() {
            return Err(CoreError::Validation("disconnect Google Drive before removing the client".into()));
        }
        state.secrets().delete(SECRET_CLIENT_ID)?;
        state.secrets().delete(SECRET_CLIENT_SECRET)
    })
    .await
}

/// Opens Google's consent page in the browser and waits (up to 3 minutes) for the redirect.
#[tauri::command]
pub async fn connect_google_drive(app: AppHandle) -> CommandResult<CloudStatus> {
    let opener_app = app.clone();
    let status = blocking(&app, move |state| {
        let client = current_client(state.secrets())?.ok_or_else(|| {
            CoreError::Validation("no Google client is available; add your own in the advanced options".into())
        })?;
        let flow = AuthorizationFlow::start(&client.id, &client.secret)?;
        opener_app
            .opener()
            .open_url(flow.authorization_url()?, None::<&str>)
            .map_err(|error| CoreError::External(format!("the browser could not be opened: {error}")))?;
        let grant = flow.finish(state.google())?;
        let refresh = grant.refresh_token.ok_or_else(|| {
            CoreError::Auth(
                "Google sent no refresh token; remove the app's access in your Google account and retry".into(),
            )
        })?;
        l8_core::redact::register_secret(&refresh);
        state.secrets().set(SECRET_REFRESH_TOKEN, &refresh)?;
        state.forget_access_token();

        let (email, root_id) = state.drive(|drive| {
            let quota = drive.quota()?;
            let root = drive.ensure_folder(None)?;
            Ok((quota.email, root))
        })?;
        CloudRepository::new(&state.db).connect(&email, &root_id)?;
        tracing::info!("Google Drive connected");
        status(state)
    })
    .await?;
    events::cloud_changed(&app, 0);
    // First pass in the background: import what Drive already holds, upload what it lacks.
    let background = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = cloud_service::run_pass(&background, true) {
            tracing::warn!(%error, "first cloud pass failed");
        }
    });
    Ok(status)
}

#[tauri::command]
pub async fn disconnect_google_drive(app: AppHandle) -> CommandResult<CloudStatus> {
    let status = blocking(&app, |state| {
        let _guard = state.cloud_lock()?;
        if let Some(token) = state.secrets().get(SECRET_REFRESH_TOKEN)?
            && let Err(error) = oauth::revoke(state.google(), &token)
        {
            // Offline revocation is best effort: the local token is deleted either way.
            tracing::warn!(%error, "could not revoke the Google token");
        }
        state.secrets().delete(SECRET_REFRESH_TOKEN)?;
        state.forget_access_token();
        CloudRepository::new(&state.db).disconnect()?;
        status(state)
    })
    .await?;
    events::cloud_changed(&app, 0);
    Ok(status)
}

#[tauri::command]
pub async fn sync_cloud_now(app: AppHandle) -> CommandResult<SyncReport> {
    let worker = app.clone();
    let report = tauri::async_runtime::spawn_blocking(move || cloud_service::run_pass(&worker, true))
        .await
        .map_err(|error| CommandError::new(CommandErrorCode::Internal, error.to_string()))??;
    events::cloud_changed(&app, report.new_conflicts);
    Ok(report)
}

#[tauri::command]
pub async fn list_sync_operations(app: AppHandle) -> CommandResult<Vec<SyncOperationInfo>> {
    blocking(&app, |state| {
        let repository = CloudRepository::new(&state.db);
        match repository.account()? {
            Some(account) => repository.pending(account.id),
            None => Ok(Vec::new()),
        }
    })
    .await
}

/// "Retry" for failed operations; the worker picks them up on its next tick.
#[tauri::command]
pub async fn retry_sync(app: AppHandle) -> CommandResult<u32> {
    let count = blocking(&app, |state| {
        let repository = CloudRepository::new(&state.db);
        let Some(account) = repository.account()? else { return Ok(0) };
        if account.status == AccountStatus::NeedsReauth {
            return Err(CoreError::Auth("sign in to Google Drive again".into()));
        }
        Ok(u32::try_from(repository.retry_failed(account.id)?).unwrap_or(u32::MAX))
    })
    .await?;
    events::cloud_changed(&app, 0);
    Ok(count)
}

#[tauri::command]
pub async fn list_conflicts(app: AppHandle) -> CommandResult<Vec<ConflictInfo>> {
    blocking(&app, |state| state.cloud(|sync| sync.conflicts())).await
}

#[tauri::command]
pub async fn resolve_conflict(app: AppHandle, conflict_id: i64, resolution: ConflictResolution) -> CommandResult<()> {
    let game_id = blocking(&app, move |state| {
        let _cloud = state.cloud_lock()?;
        let repository = CloudRepository::new(&state.db);
        let (game_id, _, _) = repository.conflict(conflict_id)?;
        if resolution == ConflictResolution::UseRemote {
            state.ensure_not_running(game_id)?;
        }
        let _snapshots = state.snapshot_lock()?;
        let engine = SnapshotEngine::new(&state.db, state.snapshot_root()?);
        state.cloud(|sync| sync.resolve_conflict(conflict_id, resolution, &engine))?;
        Ok(game_id)
    })
    .await?;
    events::saves_changed(&app, game_id);
    events::cloud_changed(&app, 0);
    Ok(())
}

/// Brings a cloud-only snapshot onto this PC (verified) without restoring it.
#[tauri::command]
pub async fn download_snapshot(app: AppHandle, snapshot_id: String) -> CommandResult<()> {
    let game_id = blocking(&app, move |state| {
        let _cloud = state.cloud_lock()?;
        let game_id = SnapshotRepository::new(&state.db).get(&snapshot_id)?.game_id;
        state.cloud(|sync| sync.download_now(&snapshot_id))?;
        Ok(game_id)
    })
    .await?;
    events::saves_changed(&app, game_id);
    Ok(())
}
