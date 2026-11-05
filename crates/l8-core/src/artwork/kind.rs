//! The kinds of artwork a game can have.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ArtworkKind {
    Cover,
    Hero,
    Logo,
    Icon,
}

impl ArtworkKind {
    pub const ALL: [Self; 4] = [Self::Cover, Self::Hero, Self::Logo, Self::Icon];

    /// Kinds fetched automatically for games without any remote art (icons stay manual).
    pub const AUTOMATIC: [Self; 3] = [Self::Cover, Self::Hero, Self::Logo];

    pub fn as_db(self) -> &'static str {
        match self {
            Self::Cover => "cover",
            Self::Hero => "hero",
            Self::Logo => "logo",
            Self::Icon => "icon",
        }
    }

    pub fn from_db(value: &str) -> CoreResult<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_db() == value)
            .ok_or_else(|| CoreError::Validation(format!("unknown artwork kind `{value}`")))
    }

    /// Path segment of SteamGridDB's `/{segment}/game/{id}` endpoints.
    pub fn steamgriddb_segment(self) -> &'static str {
        match self {
            Self::Cover => "grids",
            Self::Hero => "heroes",
            Self::Logo => "logos",
            Self::Icon => "icons",
        }
    }

    /// Extra query for SteamGridDB: vertical covers only, so a "grid" is never a square or a banner.
    pub fn steamgriddb_filters(self) -> &'static str {
        match self {
            Self::Cover => "&dimensions=600x900,342x482,660x930",
            Self::Hero | Self::Logo | Self::Icon => "",
        }
    }
}
