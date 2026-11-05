-- Xbox app and GOG games. SQLite cannot widen a CHECK constraint in place, so `platforms` and
-- `installations` are rebuilt. Foreign keys are checked at commit only, so dropping and recreating
-- `platforms` (which `games` and `library_roots` point to) is valid as long as every id is back.
PRAGMA defer_foreign_keys = ON;

CREATE TEMP TABLE platforms_kept AS SELECT id, display_name FROM platforms;
DROP TABLE platforms;
CREATE TABLE platforms (
  id            TEXT PRIMARY KEY CHECK (id IN ('steam', 'epic', 'xbox', 'gog', 'standalone')),
  display_name  TEXT NOT NULL
);
INSERT INTO platforms (id, display_name) SELECT id, display_name FROM platforms_kept;
DROP TABLE platforms_kept;
INSERT INTO platforms (id, display_name) VALUES
  ('xbox', 'Xbox'),
  ('gog', 'GOG');

-- Same for `installations` (the new sources). Nothing references this table.
CREATE TABLE installations_new (
  id               INTEGER PRIMARY KEY,
  game_id          INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
  library_root_id  INTEGER REFERENCES library_roots(id) ON DELETE SET NULL,
  install_path     TEXT NOT NULL,
  executable_path  TEXT,
  launch_uri       TEXT,
  launch_args      TEXT,
  size_bytes       INTEGER,
  is_installed     INTEGER NOT NULL DEFAULT 1,
  source           TEXT NOT NULL CHECK (source IN ('steam_manifest', 'epic_manifest', 'xbox_manifest', 'gog_registry', 'manual')),
  detected_at      TEXT NOT NULL,
  UNIQUE (game_id, install_path)
);
INSERT INTO installations_new
  (id, game_id, library_root_id, install_path, executable_path, launch_uri, launch_args, size_bytes, is_installed, source, detected_at)
  SELECT id, game_id, library_root_id, install_path, executable_path, launch_uri, launch_args, size_bytes, is_installed, source, detected_at
  FROM installations;
DROP TABLE installations;
ALTER TABLE installations_new RENAME TO installations;
