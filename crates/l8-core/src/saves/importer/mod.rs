//! Import of saves from older backup folders, without the game being installed.
//!
//! People kept manual backups long before L8Play (`Save Games/<game>/…` on a Drive folder). The
//! importer reads such a folder (`scan`), proposes games, names and snapshots for the user to
//! review, and converts each folder into a regular verified snapshot of a saves-only game
//! (`commit`). Nothing in the source folder is ever modified.

mod commit;
mod enrich;
mod hints;
mod layout;
mod model;
mod noise;
mod scan;
mod titles;

#[cfg(test)]
mod tests;

pub use commit::import_games;
pub use enrich::suggest_titles;
pub use model::{
    ImportGameRequest, ImportPlan, ImportProgress, ImportReport, ImportedGame, PlannedGame, PlannedLayout,
    PlannedSnapshot, ScanMode,
};
pub use scan::{plan_game, scan_folder};
pub use titles::{clean_title, normalize_title};
