# 06 — Rust module structure

A Cargo workspace with two members: `l8-core` (all business logic, no Tauri) and `src-tauri` (the app shell). Each file has one responsibility; providers sit behind traits.

```
Cargo.toml                         # [workspace] members = ["crates/l8-core", "src-tauri"]
crates/l8-core/
  Cargo.toml
  migrations/
    0001_init.sql
  src/
    lib.rs                         # public service API, re-exports
    error.rs                       # CoreError (thiserror), mapped to CommandError at the edge
    clock.rs                       # Clock trait (testable time)
    domain/                        # plain types, no IO
      ids.rs                       # GameId, SnapshotId (UUID v7), DeviceId
      game.rs  installation.rs  artwork.rs  save_location.rs
      snapshot.rs  cloud.rs  settings.rs
    db/
      mod.rs                       # connection pool (r2d2-free: one writer + reader connections), pragmas
      migrate.rs                   # backup copy + rusqlite_migration
      repo/
        games.rs  installations.rs  artwork.rs  save_locations.rs  save_rules.rs
        snapshots.rs  cloud.rs  sync.rs  sessions.rs  settings.rs  activity.rs  http_cache.rs
    fs/
      safe_path.rs                 # canonicalize, allowed-roots policy, traversal & reparse-point checks
      staging.rs                   # same-volume staging/quarantine dirs, guaranteed cleanup (Drop guard)
      atomic.rs                    # write-temp + flush + rename; MoveFileEx semantics
      hashing.rs                   # streaming SHA-256
      archive.rs                   # zip write/read with zip-slip protection
      quiescence.rs                # stable-mtime window + exclusive-open probe
      known_folders.rs             # Documents, Saved Games, AppData (SHGetKnownFolderPath)
    http/
      client.rs                    # reqwest with User-Agent, timeouts, redaction-safe logging
      rate_limit.rs                # per-provider governor limiter + Retry-After
      cache.rs                     # ETag / expiry cache backed by http_cache
    secrets/
      credential_store.rs          # trait SecretStore + keyring (Credential Manager) impl
    platforms/
      mod.rs                       # trait LibraryProvider { fn discover() -> Vec<DiscoveredGame> }
      steam/ locate.rs  libraries.rs  launch.rs
      epic/  manifests.rs  launch.rs
      manual.rs                    # user-added executables, bounded scan folders
      merge.rs                     # reconcile discoveries with DB (never delete)
    launcher/
      launch.rs                    # LaunchDescriptor → URI / process spawn
      process_monitor.rs           # sysinfo polling, exe-under-install-dir matching
      sessions.rs                  # session lifecycle, playtime accounting
    artwork/
      mod.rs                       # trait ArtworkProvider; ArtworkService (queue, cache, selection)
      steamgriddb.rs               # API v2 client
      steam_cdn.rs                 # fallback for Steam AppIDs
      matcher.rs                   # candidate scoring (id match > exact title > normalized > fuzzy)
      cache.rs                     # files on disk, dedupe by hash, invalidation
    saves/
      path_template.rs             # Ludusavi placeholders → concrete paths
      detection/
        mod.rs                     # trait SaveDetector; layered pipeline + confidence scoring
        ludusavi_cli.rs            # sidecar invocation (preview + --api), JSON parsing, stderr logging
        ludusavi_manifest.rs       # manifest download (ETag), YAML lookup by title / Steam id
        pcgamingwiki.rs            # cargoquery + wikitext template parsing
        local_discovery.rs         # bounded heuristic search
        ai_research.rs             # orchestrates ai:: providers over retrieved sources
        validation.rs              # existence, allowed roots, not a system dir, contains files
      snapshot/
        create.rs                  # staging → hash → zip → verify → rename → DB
        verify.rs                  # archive + per-file hash verification
        restore.rs                 # safety snapshot → stage → quarantine → swap → rollback
        retention.rs               # policy evaluation (never pinned, never manual unless opted in)
        scheduler.rs               # before-launch / after-exit / scheduled triggers
    cloud/
      mod.rs                       # trait CloudStorage { upload, download, list, delete?, quota }
      google_drive/
        oauth.rs                   # PKCE + loopback listener; tokens via SecretStore
        client.rs                  # files.create / get / list, about.get
        upload.rs                  # resumable uploads with progress
      sync/
        queue.rs                   # sync_operations worker, backoff
        reconcile.rs               # remote listing ↔ local snapshots
        conflict.rs                # lineage-based divergence detection
    ai/
      mod.rs                       # trait AiProvider { extract_save_paths(sources, game) -> Candidates }
      ollama.rs  gemini.rs
      prompt.rs                    # fixed prompt; JSON schema for the answer
      sources.rs                   # trait SearchSource; PCGamingWiki + allowlisted official pages
    jobs/
      mod.rs                       # JobId, progress reporting trait (implemented by src-tauri)
    settings.rs
  tests/                           # integration tests (temp dirs, fake providers, wiremock)
    snapshot_create.rs  snapshot_restore.rs  restore_rollback.rs  zip_slip.rs
    steam_discovery.rs  epic_discovery.rs  ludusavi_parsing.rs  conflict_detection.rs

src-tauri/
  Cargo.toml  tauri.conf.json  capabilities/default.json
  binaries/ludusavi-x86_64-pc-windows-msvc.exe   # sidecar (pinned version + LICENSE)
  src/
    main.rs                        # entry; calls lib::run()
    lib.rs                         # Builder, plugins, setup, single-instance
    state.rs                       # AppState: Arc<Services>
    dto.rs                         # #[derive(TS)] DTOs exported to src/lib/bindings
    error.rs                       # CommandError { code, message, details }
    events.rs                      # typed event names + payloads
    jobs.rs                        # background job runner bridging l8-core progress → events
    commands/
      library.rs  games.rs  launch.rs  artwork.rs  saves.rs  snapshots.rs
      cloud.rs  ai.rs  settings.rs  system.rs
```

## Conventions

- **Errors:** `CoreError` variants per domain; no `unwrap()` outside tests. Every failure is logged once at the edge, with no secrets.
- **Async:** network code is async (tokio); file-heavy work runs in `spawn_blocking`; the DB uses one writer connection behind a mutex plus read connections.
- **Cancellation:** long jobs take a `CancellationToken`. A cancelled snapshot leaves no partial file.
- **Tauri capabilities:** the WebView gets no generic `fs` or `shell` permission. Only our own commands are exposed, and the sidecar is invoked from Rust, never from JS.
- **Tests:** backup and restore tests use real temp directories: create → verify → corrupt a byte → detect; restore with a forced mid-way failure → roll back → original bytes intact; zip-slip archive → rejected; game "running" → refused.
