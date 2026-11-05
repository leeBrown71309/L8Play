//! Artwork sources of a game, without any account: Steam's public store CDN for Steam games,
//! overlaid with images from the local cache (SteamGridDB or the user's own files). Fields hold
//! either an `https://` URL or an absolute local file path. The UI falls back to generated art
//! when an image is missing.

use serde::Serialize;
use ts_rs::TS;

use crate::artwork::ArtworkKind;
use crate::platforms::Platform;

const STEAM_CDN: &str = "https://cdn.cloudflare.steamstatic.com/steam/apps";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArtworkUrls {
    /// Vertical 2:3 cover (600×900).
    pub cover: Option<String>,
    /// Wide hero (3840×1240 / 1920×620).
    pub hero: Option<String>,
    /// Transparent logo.
    pub logo: Option<String>,
    /// Store header (460×215), used when a hero is missing.
    pub header: Option<String>,
    /// Square icon (only from the local cache).
    pub icon: Option<String>,
}

impl ArtworkUrls {
    pub fn for_game(platform: Platform, platform_game_id: Option<&str>) -> Self {
        match (platform, platform_game_id) {
            (Platform::Steam, Some(app_id)) if app_id.chars().all(|c| c.is_ascii_digit()) => Self {
                cover: Some(format!("{STEAM_CDN}/{app_id}/library_600x900.jpg")),
                hero: Some(format!("{STEAM_CDN}/{app_id}/library_hero.jpg")),
                logo: Some(format!("{STEAM_CDN}/{app_id}/logo.png")),
                header: Some(format!("{STEAM_CDN}/{app_id}/header.jpg")),
                icon: None,
            },
            _ => Self::default(),
        }
    }

    /// Local cached images win over the CDN.
    pub fn overlay(&mut self, local: &std::collections::HashMap<ArtworkKind, String>) {
        for (kind, path) in local {
            let slot = match kind {
                ArtworkKind::Cover => &mut self.cover,
                ArtworkKind::Hero => &mut self.hero,
                ArtworkKind::Logo => &mut self.logo,
                ArtworkKind::Icon => &mut self.icon,
            };
            *slot = Some(path.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_games_get_cdn_urls() {
        let urls = ArtworkUrls::for_game(Platform::Steam, Some("814380"));
        assert_eq!(
            urls.cover.as_deref(),
            Some("https://cdn.cloudflare.steamstatic.com/steam/apps/814380/library_600x900.jpg")
        );
    }

    #[test]
    fn other_games_have_no_remote_art_yet() {
        assert_eq!(ArtworkUrls::for_game(Platform::Epic, Some("Fennec")), ArtworkUrls::default());
        assert_eq!(ArtworkUrls::for_game(Platform::Steam, Some("../evil")), ArtworkUrls::default());
    }
}
