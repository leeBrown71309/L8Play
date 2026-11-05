//! L8Play core: all business logic, independent of Tauri so it can be tested with `cargo test`.
//!
//! - Phase 1: persistence (SQLite + migrations), settings, log redaction.
//! - Phase 2: library discovery (Steam, Epic, standalone), launching, play sessions.
//! - Phase 3: save detection (Ludusavi sidecar) and the snapshot engine.
//! - Phase 4: artwork (SteamGridDB, local cache, manual overrides) and metadata editing.

pub mod artwork;
pub mod clock;
pub mod cloud;
pub mod db;
pub mod error;
pub mod launcher;
pub mod library;
pub mod platforms;
pub mod redact;
pub mod saves;
pub mod settings;
pub mod snapshots;

pub use error::{CoreError, CoreResult};
