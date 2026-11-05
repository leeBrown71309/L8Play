-- Drive copies that must be deleted (retention, "Delete from PC and Drive"). Kept apart from
-- sync_operations because the snapshot row is gone by the time the deletion runs, and Drive can be
-- offline or signed out for a while: the entry stays until Drive confirms the file is gone.
CREATE TABLE cloud_deletions (
  id               INTEGER PRIMARY KEY,
  account_id       INTEGER NOT NULL REFERENCES cloud_accounts(id) ON DELETE CASCADE,
  snapshot_id      TEXT NOT NULL,              -- no foreign key: the snapshot may already be removed
  remote_id        TEXT NOT NULL,
  attempts         INTEGER NOT NULL DEFAULT 0,
  last_error       TEXT,
  next_attempt_at  TEXT,
  created_at       TEXT NOT NULL,
  UNIQUE (account_id, remote_id)
);
