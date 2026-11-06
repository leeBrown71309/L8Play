-- Saves-only games and imported snapshots.
--   games.is_archived    -> the game is not installed and never will be launched from L8Play: only its saves are
--                           managed (imported from an older backup folder). Library scans never touch these rows.
--   snapshots.label      -> a name the user gave the snapshot (e.g. the folder "after ending A" it was imported from).
--   snapshots.origin     -> 'imported' for snapshots converted from a legacy backup, 'local' for the rest.
ALTER TABLE games ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0, 1));
ALTER TABLE snapshots ADD COLUMN label TEXT;
ALTER TABLE snapshots ADD COLUMN origin TEXT NOT NULL DEFAULT 'local' CHECK (origin IN ('local', 'imported'));
