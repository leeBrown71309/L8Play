//! Turns "which games are running now" samples into play sessions.

use std::collections::{HashMap, HashSet};

use crate::error::CoreResult;
use crate::library::LibraryRepository;

/// A game must be absent for this many consecutive samples before its session ends, so a
/// launcher → game hand-off or a quick restart does not split one session in two.
const MISSES_BEFORE_EXIT: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionChange {
    Started { game_id: i64 },
    Ended { game_id: i64, duration_seconds: i64 },
}

#[derive(Debug)]
struct ActiveSession {
    session_id: i64,
    misses: u8,
}

/// Keeps the open sessions between samples and persists starts/ends through the repository.
#[derive(Debug, Default)]
pub struct SessionTracker {
    active: HashMap<i64, ActiveSession>,
    /// Games launched from L8Play and not yet seen running.
    launched_by_app: HashSet<i64>,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Remembers that the next session of this game was started from L8Play.
    pub fn note_launch(&mut self, game_id: i64) {
        self.launched_by_app.insert(game_id);
    }

    pub fn running(&self) -> Vec<i64> {
        self.active.keys().copied().collect()
    }

    /// Applies one sample of running games.
    pub fn update(
        &mut self,
        repository: &LibraryRepository<'_>,
        running: &HashSet<i64>,
    ) -> CoreResult<Vec<SessionChange>> {
        let mut changes = Vec::new();
        for &game_id in running {
            if let Some(session) = self.active.get_mut(&game_id) {
                session.misses = 0;
                continue;
            }
            let by_app = self.launched_by_app.remove(&game_id);
            let session_id = repository.start_session(game_id, by_app)?;
            self.active.insert(game_id, ActiveSession { session_id, misses: 0 });
            changes.push(SessionChange::Started { game_id });
        }

        let mut ended = Vec::new();
        for (&game_id, session) in &mut self.active {
            if !running.contains(&game_id) {
                session.misses += 1;
                if session.misses >= MISSES_BEFORE_EXIT {
                    ended.push((game_id, session.session_id));
                }
            }
        }
        for (game_id, session_id) in ended {
            self.active.remove(&game_id);
            let duration_seconds = repository.end_session(session_id)?;
            changes.push(SessionChange::Ended { game_id, duration_seconds });
        }
        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::db::Database;
    use crate::platforms::{DiscoveredGame, Discovery, Platform};

    fn library_with_one_game(db: &Database) -> CoreResult<i64> {
        let repository = LibraryRepository::new(db);
        repository.merge_discovery(
            Platform::Steam,
            &Discovery {
                games: vec![DiscoveredGame {
                    platform_game_id: "814380".into(),
                    title: "Sekiro".into(),
                    install_path: PathBuf::from(r"D:\Games\Sekiro"),
                    executable_path: None,
                    size_bytes: None,
                    last_played_at: None,
                    playtime_seconds: None,
                }],
                ..Discovery::default()
            },
        )?;
        Ok(repository.list()?[0].id)
    }

    #[test]
    fn a_session_starts_and_ends_after_two_misses() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let game_id = library_with_one_game(&db)?;
        let repository = LibraryRepository::new(&db);
        let mut tracker = SessionTracker::new();
        tracker.note_launch(game_id);

        let running = HashSet::from([game_id]);
        assert_eq!(tracker.update(&repository, &running)?, vec![SessionChange::Started { game_id }]);
        assert!(tracker.update(&repository, &running)?.is_empty());
        assert!(tracker.update(&repository, &HashSet::new())?.is_empty(), "one miss is tolerated");
        let changes = tracker.update(&repository, &HashSet::new())?;
        assert!(matches!(changes.as_slice(), [SessionChange::Ended { game_id: id, .. }] if *id == game_id));

        let by_app: bool =
            db.connection()?.query_row("SELECT launched_by_app FROM game_sessions", [], |row| row.get(0))?;
        assert!(by_app);
        Ok(())
    }

    #[test]
    fn a_brief_gap_keeps_one_session() -> CoreResult<()> {
        let db = Database::open_in_memory()?;
        let game_id = library_with_one_game(&db)?;
        let repository = LibraryRepository::new(&db);
        let mut tracker = SessionTracker::new();
        let running = HashSet::from([game_id]);
        tracker.update(&repository, &running)?;
        tracker.update(&repository, &HashSet::new())?;
        tracker.update(&repository, &running)?;
        let sessions: i64 = db.connection()?.query_row("SELECT COUNT(*) FROM game_sessions", [], |row| row.get(0))?;
        assert_eq!(sessions, 1);
        Ok(())
    }
}
