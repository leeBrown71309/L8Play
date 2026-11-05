//! Error type shared by every core service.

use thiserror::Error;

/// Every failure a core service can report. The app layer maps these to user-facing codes.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("database migration failed: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid stored value for `{key}`: {source}")]
    Serialization {
        key: String,
        #[source]
        source: serde_json::Error,
    },

    /// Input that cannot be accepted (wrong file type, path outside a game folder, …).
    #[error("invalid input: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    /// The operation cannot run right now (game running, file in use); retrying later may work.
    #[error("busy: {0}")]
    Busy(String),

    /// A helper tool or external service failed (e.g. the Ludusavi sidecar).
    #[error("external tool failed: {0}")]
    External(String),

    /// An external service refused the credentials (missing, wrong or revoked API key).
    #[error("authentication failed: {0}")]
    Auth(String),

    /// A snapshot archive does not match its recorded checksums.
    #[error("integrity check failed: {0}")]
    Integrity(String),

    #[error("internal lock poisoned: {0}")]
    LockPoisoned(&'static str),
}

pub type CoreResult<T> = Result<T, CoreError>;
