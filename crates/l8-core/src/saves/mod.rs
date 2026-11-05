//! Save-location detection.
//!
//! Layer 1 (this phase): the Ludusavi CLI, run as a read-only sidecar in preview mode, answers
//! "which files belong to this game?". Those files are turned into a small set of save locations
//! (folders, or single files when the folder would be too broad), validated, and stored. Users can
//! add their own folders or reject a detected one. Layers 2–3 (PCGamingWiki, local discovery)
//! (phase 5: PCGamingWiki and bounded local discovery, via "Find saves") feed the same repository.

mod detect;
mod finder;
mod locations;
mod ludusavi;
pub mod pcgamingwiki;
mod repository;

pub use detect::{DetectionOutcome, detect_save_locations, games_needing_detection, remembered_title};
pub use finder::{CandidateReason, FindSavesOutcome, SaveCandidate, WikiStatus, find_save_candidates};
pub use locations::{
    DerivedLocation, LocationKind, broad_roots_from_environment, derive_locations, normalize_path, validate_user_folder,
};
pub use ludusavi::{LudusaviCli, sidecar_path};
pub use repository::{SaveLocation, SaveLocationRepository};
