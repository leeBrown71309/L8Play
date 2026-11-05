# 02 — System architecture

## Layers

```
┌──────────────────────────── React (WebView2) ────────────────────────────┐
│ features/*  →  hooks (TanStack Query)  →  lib/tauri (typed invoke/events) │
│ components/* (design system)        stores/* (Zustand: UI state only)     │
└───────────────────────────────▲──────────────────────────┬───────────────┘
                     typed events│                          │typed commands
┌────────────────────────────────┴── src-tauri (thin) ─────▼───────────────┐
│ commands/*  — validate input, map DTOs, call services, map errors         │
│ events.rs   — progress / state events      jobs.rs — background workers   │
└───────────────────────────────▲──────────────────────────┬───────────────┘
                                │ services (traits)        │
┌───────────────────────────────┴──── l8-core (no Tauri) ──▼───────────────┐
│ library   launcher   artwork   saves::detection   saves::snapshot         │
│ cloud::sync   ai::research   settings                                     │
│ ── providers (replaceable) ──────────────────────────────────────────────│
│ platforms::{steam,epic,manual}  artwork::{steamgriddb,steam_cdn}          │
│ saves::{ludusavi,pcgamingwiki,local,ai}  cloud::google_drive              │
│ ai::{ollama,gemini}                                                      │
│ ── infrastructure ───────────────────────────────────────────────────────│
│ db (rusqlite + migrations)  fs (safe paths, staging, atomic, hashing, zip)│
│ http (client, rate limiter, cache)  secrets (Credential Manager)  process │
└──────────────────────────────────────────────────────────────────────────┘
        │ SQLite file          │ snapshot archives        │ external APIs
   %APPDATA%\com.l8play.app (user-chosen folder)      Steam/Epic local files,
                                                       SteamGridDB, PCGW, Drive,
                                                       Ollama (localhost), Gemini
```

Rules:

- `l8-core` has no Tauri dependency, so all business logic is unit- and integration-testable with `cargo test`.
- `src-tauri` only wires: commands, events, state, plugins, background jobs.
- React never touches the file system or network directly; it calls typed commands and listens to typed events.
- Each external integration implements a trait (`LibraryProvider`, `ArtworkProvider`, `SaveDetector`, `CloudStorage`, `AiProvider`, `SearchSource`). Services depend on traits, never on concrete providers.

## Data locations

| What | Where | Notes |
| --- | --- | --- |
| Database | `%APPDATA%\com.l8play.app\l8.db` | WAL mode, migrations on start, `VACUUM INTO` backup before upgrading an existing database. |
| Artwork cache | `%LOCALAPPDATA%\com.l8play.app\artwork\<game-id>\<kind>-<hash>.<ext>` | Re-creatable; safe to clear. |
| HTTP cache | `%LOCALAPPDATA%\com.l8play.app\cache\` | Ludusavi manifest + ETag, PCGamingWiki responses. |
| Snapshots | Default `%USERPROFILE%\Documents\L8Play\Snapshots\<game-slug>\<snapshot-id>.zip`; user-configurable | User-visible on purpose, so backups are recoverable without the app. |
| Secrets | Windows Credential Manager, target names `L8Play/<provider>` | OAuth refresh token, SteamGridDB key, Gemini key. Never in the DB or logs. |
| Logs | `%LOCALAPPDATA%\com.l8play.app\logs\` (rotating) | Tokens, keys and auth headers are redacted at the logging layer. |

## Key flows

### Discovery (Module A)

1. `SteamProvider`: read `HKCU\Software\Valve\Steam\SteamPath` → parse `steamapps\libraryfolders.vdf` → every library → `appmanifest_<appid>.acf` → `(appid, name, installdir, SizeOnDisk, StateFlags)`.
2. `EpicProvider`: read `*.item` JSON in `%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests` → `(AppName, DisplayName, InstallLocation, LaunchExecutable)`; skip non-games (`bIsApplication == false`, Unreal Engine entries).
3. `ManualProvider`: user-added games, plus optional user-configured scan folders (one level deep, executables only, never whole drives).
4. Results are merged into `games` + `installations`; a game missing from a later scan becomes `is_installed = 0`, it is never deleted (its snapshots stay).

### Launch and session (Module B)

1. Optional pre-launch snapshot (setting).
2. Launch through a `LaunchDescriptor` (URI or executable + args + working dir).
3. `ProcessMonitor` polls with `sysinfo` (2 s while a launch is pending or a session is active, otherwise idle) and matches processes whose executable lives under the installation folder.
4. On exit: wait for quiescence (see below), then the post-exit automatic snapshot, then cloud upload if enabled. Every step emits events for the UI.

### Save detection pipeline (Module D)

```
game ─► L1 Ludusavi (CLI preview --api, manifest lookup)
      ─► L2 PCGamingWiki (cargoquery by Steam AppID → page wikitext → save-location templates)
      ─► L3 Local discovery (known roots × normalized title / developer / app id, recent mtimes)
      ─► L4 AI research (only if L1–L3 found nothing; optional; sources retrieved first)
           each candidate ─► Validator (path exists? within allowed roots? contains files? not a system dir?)
                         ─► confidence score ─► auto-accept (≥ threshold, verified sources only) or ask user
                         ─► save_locations + save_rules (local knowledge base)
```

AI never produces a path that is used without validation, and AI-sourced candidates always require user confirmation.

### Snapshot creation (Module E)

1. Preconditions: save locations verified; game not running; **quiescence**: no file changed for 5 s and every file opens with exclusive read sharing.
2. Copy files read-only into `<snapshots>\.staging\<id>\` (originals are never opened for writing).
3. Hash each file (SHA-256), write `manifest.json` (relative path, location id, size, mtime, hash).
4. Build `<id>.zip.tmp` (deflate), flush to disk, reopen and re-verify every entry's hash.
5. Atomic rename to `<id>.zip`, then a single DB transaction inserts `snapshots` + `snapshot_files`. Status becomes `ready` only after the transaction commits.
6. Emit a success event (the UI shows the toast). Any failure leaves no partial snapshot: staging is removed, nothing is renamed.

### Restore (Module E)

1. Verify the archive hash against the DB; abort if it doesn't match.
2. Game must not be running.
3. Create a `safety` snapshot of the current save (same engine); **abort the restore if this fails**.
4. Extract into a staging folder **on the same volume** as each target.
5. Move current files into a quarantine folder (same volume), then move staged files into place (`rename`, atomic per file).
6. On any error, roll back from quarantine. On success, quarantine is deleted only after the post-restore hash check passes.
7. Archive entries are checked against zip-slip: no absolute paths, no `..`, and every target must resolve inside a registered save location.

### Cloud sync (Module F)

- Remote layout: `L8Play/<game-slug>/<snapshot-id>.zip`, with `appProperties` = `{snapshotId, gameKey, sha256, deviceId, parentSnapshotId, createdAt}`.
- Snapshot files are **immutable and uniquely named**, so an upload never overwrites anything. If a remote file with the same id has a different hash, it's an integrity incident and nothing changes.
- Conflicts are about the **current save across devices**. Each snapshot records the snapshot the save matched when it was taken (`parent_snapshot_id`). Two heads that are not ancestors of each other mean divergence → a `sync_conflicts` row → the conflict dialog. Resolving creates a new snapshot; nothing is deleted.
- Queue: `sync_operations` with retries (exponential backoff on 403 rate limit / 429 / 5xx), resumable uploads for files > 5 MB, and `about.get` for storage quota.
- Local backups are never deleted because a cloud copy exists. Retention is a separate, explicit policy.

### AI research (Module D, layer 4)

1. Retrieve sources first: PCGamingWiki page wikitext, then any official links listed in the game's PCGamingWiki infobox. Only allowlisted domains are fetched.
2. Send the model **only the retrieved text plus the game title and identifiers** (no local paths, user names or file contents).
3. Require a JSON answer (`candidates[]: {pathTemplate, evidenceUrl, evidenceQuote}`). A candidate without a verbatim quote from a retrieved source is discarded.
4. Validate locally, then ask the user to confirm. Verified rules are cached in `save_rules`.

## Frontend ↔ backend contract

- Commands: `async fn` returning `Result<Dto, CommandError>`. `CommandError` is a typed enum (`NotFound`, `Validation`, `Conflict`, `ExternalService`, `Io`, `Cancelled`) serialized as `{ code, message, details? }`.
- DTO types are exported to TypeScript with `ts-rs`, into `src/lib/bindings/`. They are generated, never edited by hand.
- Long operations return a job id immediately and report through events `job://progress`, `job://done` and `job://failed` (payload carries the job id). Domain events (`game://started`, `game://exited`, `snapshot://created`, `sync://state`) refresh the matching TanStack Query caches.
