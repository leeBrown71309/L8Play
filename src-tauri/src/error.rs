//! The error shape every command returns to the frontend.

use l8_core::CoreError;
use serde::Serialize;
use ts_rs::TS;

/// Stable error codes the UI can branch on; messages are for logs and diagnostics only
/// (the UI shows its own translated text per code).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CommandErrorCode {
    Database,
    Io,
    Validation,
    NotFound,
    Launch,
    /// The game is running or a file is in use; try again later.
    Busy,
    /// A helper tool (Ludusavi) failed.
    External,
    /// A snapshot failed its integrity check.
    Integrity,
    /// The artwork service refused the API key (missing, wrong or revoked).
    Auth,
    Internal,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CommandError {
    pub code: CommandErrorCode,
    pub message: String,
}

impl CommandError {
    pub fn new(code: CommandErrorCode, message: impl Into<String>) -> Self {
        let message = message.into();
        tracing::error!(?code, %message, "command failed");
        Self { code, message }
    }
}

impl From<CoreError> for CommandError {
    fn from(error: CoreError) -> Self {
        let code = match &error {
            CoreError::Database(_) | CoreError::Migration(_) => CommandErrorCode::Database,
            CoreError::Io(_) => CommandErrorCode::Io,
            CoreError::Validation(_) => CommandErrorCode::Validation,
            CoreError::NotFound(_) => CommandErrorCode::NotFound,
            CoreError::Busy(_) => CommandErrorCode::Busy,
            CoreError::External(_) => CommandErrorCode::External,
            CoreError::Integrity(_) => CommandErrorCode::Integrity,
            CoreError::Auth(_) => CommandErrorCode::Auth,
            CoreError::Serialization { .. } | CoreError::LockPoisoned(_) => CommandErrorCode::Internal,
        };
        Self::new(code, error.to_string())
    }
}

impl From<tauri::Error> for CommandError {
    fn from(error: tauri::Error) -> Self {
        Self::new(CommandErrorCode::Internal, error.to_string())
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
