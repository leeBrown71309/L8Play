//! The unified game library: merges launcher discoveries and manual games into SQLite.
//!
//! Games are never deleted by a scan. A game that disappears from its launcher is marked
//! "not installed" so its history (and, later, its snapshots) stays.

mod artwork;
mod exe_scan;
mod model;
mod repository;
mod scan;
mod slug;

pub use artwork::ArtworkUrls;
pub use exe_scan::{ExecutableChoice, FolderScanReport, GameFolderCandidate, scan_game_folder};
pub use model::{GameDetail, GameSummary, LaunchTarget, PlatformScan, ScanReport, WatchedGame};
pub use repository::{AddedGame, LibraryRepository, MetadataPatch};
pub use scan::{OwnedSync, scan_libraries, sync_owned_steam_library};
