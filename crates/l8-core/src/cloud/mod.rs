//! Cloud sync with Google Drive (`drive.file` scope, the user's own OAuth client, no paid service).
//!
//! - `oauth`: authorization code + PKCE with a loopback redirect, tokens in the Credential Manager.
//! - `drive`: Drive API v3 (folders, queries, resumable uploads, streamed downloads).
//! - `remote`: the provider interface the engine uses, plus its Drive implementation.
//! - `sync`: upload queue (backoff, resume), import from Drive, lineage-based conflicts, and the
//!   deletion of Drive copies that retention or the user removed.

pub mod client;
pub mod drive;
pub mod model;
pub mod oauth;
pub mod remote;
pub mod repository;
pub mod sync;
pub mod transport;

#[cfg(test)]
mod tests;

pub use client::{ClientSource, GoogleClient, current_client};
pub use model::{
    AccountStatus, CloudAccountInfo, CloudQuota, CloudStatus, ConflictInfo, ConflictResolution, ConflictSide,
    SyncDirection, SyncOperationInfo, SyncReport, SyncStatus,
};
pub use oauth::{AuthorizationFlow, GoogleTokens, TokenSource};
pub use remote::{DriveRemote, RemoteStore};
pub use repository::CloudRepository;
pub use sync::{CloudSync, is_uploadable};
pub use transport::{Transport, UreqTransport};
