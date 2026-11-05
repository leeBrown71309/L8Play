//! Game artwork: SteamGridDB (optional, the user's own free API key), a re-creatable disk cache
//! and manual overrides. Steam games keep using Steam's public CDN unless the user picks another
//! image; everything works without a key.

pub mod http;
pub mod kind;
pub mod secrets;
pub mod service;
pub mod steamgriddb;
pub mod store;

#[cfg(test)]
mod tests;

pub use http::{HttpGet, UreqHttp};
pub use kind::ArtworkKind;
pub use secrets::{CredentialManagerStore, MemorySecretStore, SecretStore};
pub use service::{ArtworkFetchReport, ArtworkService};
pub use steamgriddb::ArtworkCandidate;
pub use store::{ArtworkSelection, active_art_by_game, resolve_public_path};
