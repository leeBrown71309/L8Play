-- Where a snapshot is meant to live, chosen when it is created:
--   both  -> this PC, and Google Drive once connected (the default, and what automatic snapshots use)
--   local -> this PC only: the sync queue never uploads it
--   cloud -> Google Drive only: kept on this PC until Drive confirms the upload, then the local archive is removed
ALTER TABLE snapshots ADD COLUMN storage TEXT NOT NULL DEFAULT 'both' CHECK (storage IN ('both', 'local', 'cloud'));
