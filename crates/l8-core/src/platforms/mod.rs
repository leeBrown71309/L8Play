//! Game platforms and their local discovery.
//!
//! Each provider reads the launcher's own files (no network, no credentials, no recursive drive
//! scan) and returns what is installed. Merging into the database is done by
//! [`crate::library::LibraryRepository`].

pub mod epic;
pub mod gog;
pub mod steam;
pub mod steam_owned;
pub mod xbox;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::CoreResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Platform {
    Steam,
    Epic,
    Xbox,
    Gog,
    Standalone,
}

impl Platform {
    /// Value stored in `platforms.id`.
    pub fn as_db_id(self) -> &'static str {
        match self {
            Self::Steam => "steam",
            Self::Epic => "epic",
            Self::Xbox => "xbox",
            Self::Gog => "gog",
            Self::Standalone => "standalone",
        }
    }

    pub fn from_db_id(value: &str) -> Option<Self> {
        match value {
            "steam" => Some(Self::Steam),
            "epic" => Some(Self::Epic),
            "xbox" => Some(Self::Xbox),
            "gog" => Some(Self::Gog),
            "standalone" => Some(Self::Standalone),
            _ => None,
        }
    }
}

/// One installed game as reported by a launcher.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredGame {
    /// Steam AppID, Epic AppName, Xbox application id (`<package family>!<app id>`) or GOG game id.
    pub platform_game_id: String,
    pub title: String,
    pub install_path: PathBuf,
    pub executable_path: Option<PathBuf>,
    pub size_bytes: Option<u64>,
    /// Last play time known to the launcher itself (RFC 3339), if any.
    pub last_played_at: Option<String>,
    /// Total playtime known to the launcher itself.
    pub playtime_seconds: Option<i64>,
}

/// Result of one provider run.
#[derive(Debug, Clone, Default)]
pub struct Discovery {
    pub games: Vec<DiscoveredGame>,
    /// Library folders the games were found in (stored in `library_roots`).
    pub library_paths: Vec<PathBuf>,
    /// Non-fatal problems (a malformed manifest, an unreadable library); logged and reported.
    pub warnings: Vec<String>,
}

/// A launcher whose installed games can be listed from local files.
pub trait LibraryProvider {
    fn platform(&self) -> Platform;

    /// `Ok(None)` means the launcher is not installed on this PC, which is different from
    /// "installed with zero games": only a real discovery may mark games as uninstalled.
    fn discover(&self) -> CoreResult<Option<Discovery>>;
}
