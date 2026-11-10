//! Save snapshots: verified zip archives of a game's save locations, with safe restore.

mod archive;
mod engine;
mod export;
mod files;
mod imported_restore;
mod migrate;
mod model;
mod repository;

#[cfg(test)]
mod tests;

pub use engine::{CreateRequest, ImportOutcome, ImportRequest, RetentionPolicy, RetentionReport, SnapshotEngine};
pub use export::{ExportReport, export_all_snapshots};
pub use files::sha256_file;
pub use imported_restore::{RestoreTarget, TargetSource, expand_environment};
pub use migrate::migrate_legacy_root;
pub use model::{
    CloudState, CreateOutcome, CurrentSaveState, GameSaveStatus, RestoreReport, SnapshotKind, SnapshotStatus,
    SnapshotStorage, SnapshotSummary,
};
pub use repository::SnapshotRepository;
