//! Save snapshots: verified zip archives of a game's save locations, with safe restore.

mod archive;
mod engine;
mod files;
mod migrate;
mod model;
mod repository;

#[cfg(test)]
mod tests;

pub use engine::{CreateRequest, RetentionPolicy, RetentionReport, SnapshotEngine};
pub use files::sha256_file;
pub use migrate::migrate_legacy_root;
pub use model::{
    CloudState, CreateOutcome, CurrentSaveState, GameSaveStatus, RestoreReport, SnapshotKind, SnapshotStatus,
    SnapshotStorage, SnapshotSummary,
};
pub use repository::SnapshotRepository;
