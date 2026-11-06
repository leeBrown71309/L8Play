//! Looks the guessed titles up in the Ludusavi game database to propose their official names.

use crate::saves::LudusaviCli;
use crate::saves::importer::model::ImportPlan;

/// Fills `suggested_title` of every game whose folder name (or a name found inside it) matches a game
/// in the Ludusavi database. Never fails: an unreadable database simply leaves the guesses as they are.
pub fn suggest_titles(cli: &LudusaviCli, plan: &mut ImportPlan) {
    if !cli.is_available() {
        tracing::warn!("Ludusavi sidecar missing: backup titles are not matched to the game database");
        return;
    }
    for game in &mut plan.games {
        let candidates = std::iter::once(&game.guessed_title).chain(game.alt_titles.iter());
        for candidate in candidates {
            match cli.find_by_name(candidate) {
                Ok(Some(found)) => {
                    game.suggested_title = Some(found);
                    break;
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(%error, title = %candidate, "game database lookup failed");
                    return;
                }
            }
        }
    }
}
