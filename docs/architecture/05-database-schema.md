# 05 — SQLite schema

SQLite, WAL mode, `foreign_keys = ON`, migrations in `crates/l8-core/migrations/NNNN_name.sql`, applied by `rusqlite_migration` at start-up after a file copy of the DB. Large files (archives, artwork) live on disk; the DB stores paths, hashes and metadata. Timestamps are UTC ISO-8601 text; durations are integer seconds; sizes are integer bytes.

## Entity overview

```
platforms 1─* games 1─* installations
                │ 1─* game_external_ids
                │ 1─* artwork_assets        artwork_lookups (negative cache)
                │ 1─* save_locations ─* save_rules (knowledge base)
                │ 1─* snapshots 1─* snapshot_files
                │        └─* cloud_objects *─1 cloud_accounts 1─* sync_operations
                │ 1─* sync_conflicts
                │ 1─* game_sessions
                └ 1─* ai_research_runs 1─* ai_candidates
devices   settings   library_roots   http_cache   activity_log
```

## DDL (migration 0001)

```sql
CREATE TABLE platforms (
  id            TEXT PRIMARY KEY CHECK (id IN ('steam', 'epic', 'standalone')),
  display_name  TEXT NOT NULL
);

CREATE TABLE games (
  id                INTEGER PRIMARY KEY,
  platform_id       TEXT NOT NULL REFERENCES platforms(id),
  platform_game_id  TEXT,                       -- Steam AppID, Epic AppName; NULL for standalone
  title             TEXT NOT NULL,
  sort_title        TEXT NOT NULL,
  slug              TEXT NOT NULL UNIQUE,       -- folder name for snapshots and cloud
  developer         TEXT,
  release_year      INTEGER,
  is_favorite       INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0, 1)),
  is_hidden         INTEGER NOT NULL DEFAULT 0 CHECK (is_hidden IN (0, 1)),
  user_edited       INTEGER NOT NULL DEFAULT 0, -- metadata manually corrected; never overwritten by scans
  last_played_at    TEXT,
  playtime_seconds  INTEGER NOT NULL DEFAULT 0,
  added_at          TEXT NOT NULL,
  updated_at        TEXT NOT NULL,
  UNIQUE (platform_id, platform_game_id)
);
CREATE INDEX idx_games_last_played ON games(last_played_at DESC);
CREATE INDEX idx_games_sort_title  ON games(sort_title);

CREATE TABLE game_external_ids (
  game_id      INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
  provider     TEXT NOT NULL,   -- 'steam', 'steamgriddb', 'pcgamingwiki', 'ludusavi', 'gog'
  external_id  TEXT NOT NULL,
  PRIMARY KEY (game_id, provider)
);
CREATE INDEX idx_external_lookup ON game_external_ids(provider, external_id);

CREATE TABLE library_roots (
  id           INTEGER PRIMARY KEY,
  platform_id  TEXT NOT NULL REFERENCES platforms(id),
  path         TEXT NOT NULL UNIQUE,
  kind         TEXT NOT NULL CHECK (kind IN ('steam_library', 'epic_manifests', 'scan_folder')),
  enabled      INTEGER NOT NULL DEFAULT 1,
  added_by     TEXT NOT NULL CHECK (added_by IN ('detected', 'user'))
);

CREATE TABLE installations (
  id               INTEGER PRIMARY KEY,
  game_id          INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
  library_root_id  INTEGER REFERENCES library_roots(id) ON DELETE SET NULL,
  install_path     TEXT NOT NULL,
  executable_path  TEXT,
  launch_uri       TEXT,
  launch_args      TEXT,
  size_bytes       INTEGER,
  is_installed     INTEGER NOT NULL DEFAULT 1,
  source           TEXT NOT NULL CHECK (source IN ('steam_manifest', 'epic_manifest', 'manual')),
  detected_at      TEXT NOT NULL,
  UNIQUE (game_id, install_path)
);

CREATE TABLE artwork_assets (
  id                 INTEGER PRIMARY KEY,
  game_id            INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
  kind               TEXT NOT NULL CHECK (kind IN ('cover', 'hero', 'logo', 'icon', 'background')),
  provider           TEXT NOT NULL,             -- 'steamgriddb', 'steam_cdn', 'user', 'generated'
  provider_asset_id  TEXT,
  source_url         TEXT,
  local_path         TEXT NOT NULL,
  width              INTEGER,
  height             INTEGER,
  mime               TEXT,
  sha256             TEXT NOT NULL,
  is_active          INTEGER NOT NULL DEFAULT 1,
  is_user_selected   INTEGER NOT NULL DEFAULT 0,
  fetched_at         TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_artwork_active ON artwork_assets(game_id, kind) WHERE is_active = 1;

CREATE TABLE artwork_lookups (                  -- avoids repeated requests, incl. "nothing found"
  game_id          INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
  provider         TEXT NOT NULL,
  kind             TEXT NOT NULL,
  status           TEXT NOT NULL CHECK (status IN ('found', 'not_found', 'error')),
  last_attempt_at  TEXT NOT NULL,
  next_retry_at    TEXT,
  PRIMARY KEY (game_id, provider, kind)
);

CREATE TABLE save_rules (                       -- local knowledge base of verified rules
  id             INTEGER PRIMARY KEY,
  game_key       TEXT NOT NULL,                 -- 'steam:814380', 'title:sekiro shadows die twice'
  path_template  TEXT NOT NULL,                 -- Ludusavi-style placeholders: <winAppData>/Sekiro/*
  source         TEXT NOT NULL CHECK (source IN ('ludusavi', 'pcgamingwiki', 'local_discovery', 'ai', 'user')),
  source_ref     TEXT,                          -- URL / revision / manifest ETag
  licence_note   TEXT,
  created_at     TEXT NOT NULL,
  UNIQUE (game_key, path_template)
);

CREATE TABLE save_locations (
  id                INTEGER PRIMARY KEY,
  game_id           INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
  rule_id           INTEGER REFERENCES save_rules(id) ON DELETE SET NULL,
  path_template     TEXT NOT NULL,
  resolved_path     TEXT NOT NULL,
  kind              TEXT NOT NULL CHECK (kind IN ('file', 'directory', 'glob')),
  source            TEXT NOT NULL,
  confidence        REAL NOT NULL CHECK (confidence BETWEEN 0 AND 1),
  status            TEXT NOT NULL CHECK (status IN ('candidate', 'verified', 'rejected')),
  confirmed_by_user INTEGER NOT NULL DEFAULT 0,
  detected_at       TEXT NOT NULL,
  last_verified_at  TEXT,
  UNIQUE (game_id, resolved_path)
);

CREATE TABLE devices (
  id          TEXT PRIMARY KEY,                 -- UUID, generated on first launch
  name        TEXT NOT NULL,                    -- computer name
  is_current  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE snapshots (
  id                     TEXT PRIMARY KEY,      -- UUID v7 (time-ordered)
  game_id                INTEGER NOT NULL REFERENCES games(id),
  name                   TEXT NOT NULL,
  description            TEXT,
  kind                   TEXT NOT NULL CHECK (kind IN ('manual', 'auto_exit', 'auto_launch', 'scheduled', 'safety', 'conflict_copy')),
  status                 TEXT NOT NULL CHECK (status IN ('creating', 'ready', 'corrupt', 'missing_local', 'deleted')),
  device_id              TEXT NOT NULL REFERENCES devices(id),
  parent_snapshot_id     TEXT REFERENCES snapshots(id),  -- lineage for conflict detection
  archive_path           TEXT,                  -- NULL when only in the cloud
  archive_size           INTEGER NOT NULL,
  archive_sha256         TEXT NOT NULL,
  total_files            INTEGER NOT NULL,
  total_bytes            INTEGER NOT NULL,
  playtime_seconds       INTEGER,
  game_version           TEXT,
  is_pinned              INTEGER NOT NULL DEFAULT 0,
  created_at             TEXT NOT NULL,
  verified_at            TEXT
);
CREATE INDEX idx_snapshots_game_created ON snapshots(game_id, created_at DESC);
CREATE INDEX idx_snapshots_status ON snapshots(status);

CREATE TABLE snapshot_files (
  snapshot_id       TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
  rel_path          TEXT NOT NULL,              -- path inside the archive
  save_location_id  INTEGER REFERENCES save_locations(id) ON DELETE SET NULL,
  original_path     TEXT NOT NULL,              -- resolved path at capture time
  size_bytes        INTEGER NOT NULL,
  sha256            TEXT NOT NULL,
  modified_at       TEXT NOT NULL,
  PRIMARY KEY (snapshot_id, rel_path)
);

CREATE TABLE cloud_accounts (
  id              INTEGER PRIMARY KEY,
  provider        TEXT NOT NULL CHECK (provider IN ('google_drive')),
  account_email   TEXT NOT NULL,
  root_folder_id  TEXT,                         -- remote id of "L8Play"
  credential_ref  TEXT NOT NULL,                -- Credential Manager target name; never the token
  status          TEXT NOT NULL CHECK (status IN ('connected', 'needs_reauth', 'disconnected')),
  connected_at    TEXT NOT NULL,
  UNIQUE (provider, account_email)
);

CREATE TABLE cloud_objects (
  id           INTEGER PRIMARY KEY,
  account_id   INTEGER NOT NULL REFERENCES cloud_accounts(id) ON DELETE CASCADE,
  snapshot_id  TEXT NOT NULL REFERENCES snapshots(id),
  remote_id    TEXT NOT NULL,
  remote_path  TEXT NOT NULL,
  size_bytes   INTEGER NOT NULL,
  sha256       TEXT NOT NULL,
  uploaded_at  TEXT NOT NULL,
  status       TEXT NOT NULL CHECK (status IN ('present', 'missing', 'integrity_mismatch')),
  UNIQUE (account_id, snapshot_id),
  UNIQUE (account_id, remote_id)
);

CREATE TABLE sync_operations (
  id               INTEGER PRIMARY KEY,
  account_id       INTEGER NOT NULL REFERENCES cloud_accounts(id) ON DELETE CASCADE,
  snapshot_id      TEXT NOT NULL REFERENCES snapshots(id),
  direction        TEXT NOT NULL CHECK (direction IN ('upload', 'download')),
  status           TEXT NOT NULL CHECK (status IN ('queued', 'running', 'done', 'failed', 'paused', 'cancelled')),
  attempts         INTEGER NOT NULL DEFAULT 0,
  bytes_done       INTEGER NOT NULL DEFAULT 0,
  bytes_total      INTEGER NOT NULL,
  resumable_uri    TEXT,                        -- Drive resumable session
  last_error       TEXT,
  next_attempt_at  TEXT,
  created_at       TEXT NOT NULL,
  updated_at       TEXT NOT NULL
);
CREATE INDEX idx_sync_queue ON sync_operations(status, next_attempt_at);

CREATE TABLE sync_conflicts (
  id                   INTEGER PRIMARY KEY,
  game_id              INTEGER NOT NULL REFERENCES games(id),
  local_snapshot_id    TEXT NOT NULL REFERENCES snapshots(id),
  remote_snapshot_id   TEXT NOT NULL REFERENCES snapshots(id),
  detected_at          TEXT NOT NULL,
  resolution           TEXT CHECK (resolution IN ('use_local', 'use_remote', 'keep_both')),
  resolved_at          TEXT
);
CREATE INDEX idx_conflicts_open ON sync_conflicts(game_id) WHERE resolved_at IS NULL;

CREATE TABLE game_sessions (
  id               INTEGER PRIMARY KEY,
  game_id          INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
  started_at       TEXT NOT NULL,
  ended_at         TEXT,
  launched_by_app  INTEGER NOT NULL
);
CREATE INDEX idx_sessions_game ON game_sessions(game_id, started_at DESC);

CREATE TABLE ai_research_runs (
  id          INTEGER PRIMARY KEY,
  game_id     INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
  provider    TEXT NOT NULL CHECK (provider IN ('ollama', 'gemini')),
  model       TEXT NOT NULL,
  sources     TEXT NOT NULL,                    -- JSON array of {url, retrievedAt, sha256}
  raw_result  TEXT,                             -- model JSON output, for audit
  status      TEXT NOT NULL CHECK (status IN ('done', 'failed', 'rejected')),
  created_at  TEXT NOT NULL
);

CREATE TABLE ai_candidates (
  id                 INTEGER PRIMARY KEY,
  run_id             INTEGER NOT NULL REFERENCES ai_research_runs(id) ON DELETE CASCADE,
  path_template      TEXT NOT NULL,
  evidence_url       TEXT NOT NULL,
  evidence_quote     TEXT NOT NULL,
  validation_status  TEXT NOT NULL CHECK (validation_status IN ('pending', 'exists', 'not_found', 'rejected', 'confirmed'))
);

CREATE TABLE http_cache (
  cache_key    TEXT PRIMARY KEY,                -- provider + normalized request
  etag         TEXT,
  body_path    TEXT NOT NULL,                   -- file in %LOCALAPPDATA%\L8Play\cache
  fetched_at   TEXT NOT NULL,
  expires_at   TEXT NOT NULL
);

CREATE TABLE settings (
  key         TEXT PRIMARY KEY,
  value       TEXT NOT NULL,                    -- JSON
  updated_at  TEXT NOT NULL
);

CREATE TABLE activity_log (
  id           INTEGER PRIMARY KEY,
  kind         TEXT NOT NULL,                   -- 'snapshot_created', 'uploaded', 'restored', 'conflict', ...
  game_id      INTEGER REFERENCES games(id) ON DELETE SET NULL,
  snapshot_id  TEXT REFERENCES snapshots(id) ON DELETE SET NULL,
  message      TEXT NOT NULL,
  created_at   TEXT NOT NULL
);
CREATE INDEX idx_activity_created ON activity_log(created_at DESC);
```

## Notes

- `games` rows are never deleted by a scan; uninstalled games keep their snapshots and cloud copies.
- `snapshots.status = 'deleted'` is a soft delete until the archive file is removed (two-phase: DB first, file second, with a start-up reconciler for interrupted deletions).
- A snapshot that exists only in Drive (downloaded from another device) has `archive_path = NULL` until it is downloaded.
- **Saves-only games (migration 0005).** `games.is_archived = 1` marks a game whose saves were imported from an older backup folder and that is not installed. It has no `installations` row, so library scans never touch it, and it is shown in the library as "Saves only". When the same game is installed later, `LibraryRepository::merge_archived_into` moves its snapshots to the installed game and deletes the saves-only row.
- **Imported snapshots.** `snapshots.origin = 'imported'` and `snapshots.label` (the name taken from the backup's sub-folder). They use the regular archive format; their single location is marked `unresolved` in `l8-snapshot.json` (optional `hint` = restore folder from a note shipped with the backup), and the destination is chosen when they are restored (`SnapshotEngine::restore_imported`, which lays files over the chosen folder and never deletes anything). `kind` stays `manual`, because the `kind` CHECK constraint cannot be extended without rebuilding the table.
- Secrets never enter the DB. `cloud_accounts.credential_ref` only names the Credential Manager entry.
