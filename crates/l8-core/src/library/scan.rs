//! Runs every library provider and merges the results.

use crate::artwork::HttpGet;
use crate::db::Database;
use crate::error::CoreResult;
use crate::library::{LibraryRepository, PlatformScan, ScanReport};
use crate::platforms::LibraryProvider;
use crate::platforms::steam_owned::{OwnedLibraryStatus, fetch_owned_games};

/// Scans each provider independently: one failing launcher never blocks the others, and a
/// launcher that cannot be read never marks its games as uninstalled.
pub fn scan_libraries(db: &Database, providers: &[&dyn LibraryProvider]) -> CoreResult<ScanReport> {
    let repository = LibraryRepository::new(db);
    let mut report = ScanReport::default();
    for provider in providers {
        let platform = provider.platform();
        match provider.discover() {
            Ok(Some(discovery)) => {
                let counts = repository.merge_discovery(platform, &discovery)?;
                report.added += counts.added;
                report.updated += counts.updated;
                report.uninstalled += counts.uninstalled;
                for warning in &discovery.warnings {
                    tracing::warn!(?platform, warning, "discovery warning");
                }
                report.warnings.extend(discovery.warnings.iter().cloned());
                let found = u32::try_from(discovery.games.len()).unwrap_or(u32::MAX);
                report.platforms.push(PlatformScan { platform, found: Some(found), error: None });
            }
            Ok(None) => report.platforms.push(PlatformScan { platform, found: None, error: None }),
            Err(error) => {
                tracing::warn!(?platform, %error, "discovery failed");
                report.platforms.push(PlatformScan { platform, found: None, error: Some(error.to_string()) });
            }
        }
    }
    report.total = u32::try_from(repository.list()?.len()).unwrap_or(u32::MAX);
    tracing::info!(
        added = report.added,
        uninstalled = report.uninstalled,
        total = report.total,
        "library scan finished"
    );
    Ok(report)
}

/// Result of listing the Steam games the account owns but has not installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedSync {
    pub status: OwnedLibraryStatus,
    /// Games added to the library by this pass.
    pub added: u32,
}

/// Adds the owned, not installed Steam games to the library. Runs after the local scan and never fails
/// it: a switched-off setting, a missing Steam account, a private profile or a network problem
/// only changes the returned status. Nothing already in the library is removed.
pub fn sync_owned_steam_library(
    db: &Database,
    http: &dyn HttpGet,
    steam_id: Option<&str>,
    api_key: Option<&str>,
    enabled: bool,
) -> OwnedSync {
    let skipped = |status| OwnedSync { status, added: 0 };
    if !enabled {
        return skipped(OwnedLibraryStatus::Disabled);
    }
    let Some(steam_id) = steam_id else { return skipped(OwnedLibraryStatus::Unavailable) };
    let owned = match fetch_owned_games(http, steam_id, api_key) {
        Ok(owned) => owned,
        Err(status) => return skipped(status),
    };
    match LibraryRepository::new(db).merge_owned_steam(&owned) {
        Ok(added) => {
            tracing::info!(listed = owned.len(), added, "owned Steam games listed");
            OwnedSync { status: OwnedLibraryStatus::Listed, added }
        }
        Err(error) => {
            tracing::warn!(%error, "owned Steam games could not be saved");
            skipped(OwnedLibraryStatus::Unavailable)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::artwork::http::HttpFailure;
    use crate::error::CoreError;
    use crate::platforms::{DiscoveredGame, Discovery, Platform};

    struct FakeProvider {
        platform: Platform,
        result: fn() -> CoreResult<Option<Discovery>>,
    }

    impl LibraryProvider for FakeProvider {
        fn platform(&self) -> Platform {
            self.platform
        }
        fn discover(&self) -> CoreResult<Option<Discovery>> {
            (self.result)()
        }
    }

    fn one_epic_game() -> CoreResult<Option<Discovery>> {
        Ok(Some(Discovery {
            games: vec![DiscoveredGame {
                platform_game_id: "Fennec".into(),
                title: "Alan Wake 2".into(),
                install_path: PathBuf::from(r"D:\Epic\AlanWake2"),
                executable_path: None,
                size_bytes: None,
                last_played_at: None,
                playtime_seconds: None,
            }],
            ..Discovery::default()
        }))
    }

    struct FakeSteamProfile(Result<String, HttpFailure>);

    impl HttpGet for FakeSteamProfile {
        fn get_text(&self, _url: &str, _bearer: Option<&str>) -> Result<String, HttpFailure> {
            self.0.clone()
        }
        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>, HttpFailure> {
            Err(HttpFailure::NotFound)
        }
    }

    #[test]
    fn owned_games_are_listed_without_ever_failing_the_scan() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let id = Some("76561198000000000");
        let public = FakeSteamProfile(Ok(concat!(
            "<gamesList><games><game><appID>367520</appID><name>Hollow Knight</name>",
            "<hoursOnRecord>3.5</hoursOnRecord></game></games></gamesList>"
        )
        .into()));

        // Off: nothing is requested and nothing changes.
        let off = sync_owned_steam_library(&db, &public, id, None, false);
        assert_eq!((off.status, off.added), (OwnedLibraryStatus::Disabled, 0));
        assert!(LibraryRepository::new(&db).list()?.is_empty());

        // No Steam account on this PC.
        assert_eq!(sync_owned_steam_library(&db, &public, None, None, true).status, OwnedLibraryStatus::Unavailable);

        let on = sync_owned_steam_library(&db, &public, id, None, true);
        assert_eq!((on.status, on.added), (OwnedLibraryStatus::Listed, 1));
        let again = sync_owned_steam_library(&db, &public, id, None, true);
        assert_eq!((again.status, again.added), (OwnedLibraryStatus::Listed, 0));

        let private = FakeSteamProfile(Ok("<gamesList><error>This profile is private.</error></gamesList>".into()));
        assert_eq!(sync_owned_steam_library(&db, &private, id, None, true).status, OwnedLibraryStatus::Private);
        let offline = FakeSteamProfile(Err(HttpFailure::Other("offline".into())));
        let failed = sync_owned_steam_library(&db, &offline, id, None, true);
        assert_eq!((failed.status, failed.added), (OwnedLibraryStatus::Unavailable, 0));
        assert_eq!(LibraryRepository::new(&db).list()?.len(), 1, "failures never remove games");
        Ok(())
    }

    #[test]
    fn one_failing_provider_does_not_block_others() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let broken =
            FakeProvider { platform: Platform::Steam, result: || Err(CoreError::Validation("unreadable".into())) };
        let epic = FakeProvider { platform: Platform::Epic, result: one_epic_game };
        let report = scan_libraries(&db, &[&broken, &epic])?;
        assert_eq!(report.added, 1);
        assert_eq!(report.total, 1);
        assert!(report.platforms[0].error.is_some());
        assert_eq!(report.platforms[1].found, Some(1));
        Ok(())
    }
}
