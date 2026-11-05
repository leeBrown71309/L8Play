# 08 — Phased implementation roadmap

Each phase ends with a working, tested build you can try, and needs your approval before the next one starts. A **spike** is a small throwaway experiment that confirms an external behaviour before production code depends on it.

The order was chosen to reach a usable version as early as possible. After phase 3 the app already launches your games and protects their saves locally.

| Phase | Scope | Done when | Spikes first |
| --- | --- | --- | --- |
| **0 — Research & architecture** | This document set | Approved 2026-11-04 | — |
| **1 — Desktop foundation** | Tauri 2 + React + TS + Vite + Tailwind + shadcn; Lingui (EN/FR); Cargo workspace (`l8-core`, `src-tauri`); design tokens → CSS; local fonts; AppWindow (CaptionBar, NavRail, QuickRail, TopBar); routing for the 5 sections with empty states; SQLite + migration 0001; settings persistence (theme, language); logging with redaction; quality scripts (`tsc`, vitest, `cargo clippy`, `cargo test`) | The app opens with the L8Play shell, navigates, switches theme and language and remembers them; the DB is created and migrated; all checks pass | Custom title bar on Windows 11 (drag, snap, maximize) |
| **2 — Game discovery & launch** | Steam (registry → libraries → manifests), Epic (`.item`), manual games, merge into DB, library grid/list with search, filters, sort and favourites, game details, launch via URI or exe, process monitor, sessions and playtime. **Quick artwork:** Steam CDN images for Steam games, generated placeholders for the rest | Your Steam and Epic games appear with correct paths and, for Steam, real covers; Play launches them; start and exit are detected | Epic manifest fields on your machine |
| **3 — Save management (local)** | Ludusavi sidecar (preview / `--api`) + manifest cache (ETag), path templates, validation, save locations UI (confirm, add, reject), snapshot engine (create, verify, restore, safety copy, retention), Create / Restore / Delete dialogs, Saves tab, Save Manager, dashboard protection data, automatic snapshots after exit, before launch and on schedule | **First usable version.** Full backup / restore round trip on real games; corruption detected; interrupted restore rolls back; all backup tests green | Ludusavi CLI output on 10 of your games |
| **4 — Artwork & metadata** | ArtworkProvider trait, SteamGridDB client (your key in Credential Manager), matcher, disk cache + invalidation, Artwork tab (preview, choose, upload, reset), metadata editing, Epic and standalone artwork | Every game gets cover, hero, logo and icon when available, never re-downloaded; manual overrides persist | SteamGridDB endpoints and real rate behaviour with your key |
| **5 — Extended detection (no AI)** | PCGamingWiki layer (cargoquery + wikitext), bounded local discovery, `save_rules` knowledge cache, "Find saves" flow for games Ludusavi doesn't know | A game unknown to Ludusavi gets candidates with evidence links; nothing is used without validation | PCGamingWiki template variety |
| **6 — Cloud sync** | OAuth (PKCE + loopback), Drive client with `drive.file`, folder layout, resumable upload and download, queue with backoff, quota display, lineage-based conflict detection, conflict dialog, recovery from Drive on a new PC | Snapshots appear in "L8Play" in your Drive; a second device's divergent save raises a conflict; nothing is overwritten | Token lifetime of a `drive.file` app published "In production" |
| **7 — Finalization** | Tray icon and background worker, Windows notifications, onboarding flow, performance and security passes, accessibility, NSIS installer, public repository + OSI licence + SignPath Foundation application, optional updater (GitHub Releases) | Signed installer installs cleanly on a fresh Windows 11 VM without admin rights | SignPath onboarding requirements |
| *Later — AI assistance* | Deferred by decision: optional Ollama / Gemini research over retrieved sources, behind the existing `AiProvider` and `SearchSource` traits | — | Free-tier terms at that time |

## Phase 3 notes

- Delivered: Ludusavi v0.31.0 sidecar (pinned SHA-256, `scripts/fetch-ludusavi.ts`), save-location derivation, user folders and rejections, the snapshot engine (zip + per-file SHA-256, verify, restore with safety copy and rollback, retention of automatic snapshots only), the Saves panel on the game page, the Save Manager, the Backup settings and automatic snapshots on exit and before launch.
- Verified live on 2026-11-04: Elden Ring Nightreign (1 location) and Monster Hunter Wilds (2 locations, one file-level inside the install folder) detected; Lossless Scaling and a standalone game correctly reported as unknown.
- Deferred: snapshots on a schedule while playing, and Windows-registry saves (Ludusavi reports them; L8Play only copies files for now). Games Ludusavi does not know are covered by "Add folder" until phase 5.

## Phase 4 notes

- Delivered: `artwork` module (SteamGridDB v2 client, bounded and host-allow-listed HTTP at 1 request/s, disk cache `%LOCALAPPDATA%com.l8play.appartwork`, negative cache with retry dates), SteamGridDB key in the Credential Manager, Artwork tab (candidates, own image, reset), Properties tab (title, developer, year; edits survive scans), Settings › Artwork, automatic lookup for Epic and standalone games at start-up and after adding a game.
- Decisions: the blocking `ureq` client replaces `reqwest` (the core is synchronous, so no async runtime is needed) and a small pacing lock replaces `governor`. Steam games keep Steam's CDN art unless the user picks another image. The automatic matcher only accepts an exact title match; manual search takes the first result and lets the user type another name.
- Not verified yet: SteamGridDB endpoints against a real key (the unit tests use a fake HTTP layer built from the official client libraries' response shape). First live run needs the user's key.

## Phase 5 notes

- Delivered: `saves::pcgamingwiki` (page lookup by Steam app id through PCGamingWiki's redirect service, then exact title, then exact search hit; Cargo queries are denied to anonymous clients, see 04; wikitext parser for `Game data/saves` templates, `{{p|…}}` placeholder expansion), bounded local discovery (Documents, Saved Games, AppData, LocalLow, ProgramData, install folder; two levels deep, save-like files required, install/launcher folders rejected), the `save_rules` cache (30 days, "nothing found" remembered), and the "Find saves" dialog. Candidates carry their evidence; nothing is backed up until the user confirms, and confirmation re-runs the same folder validation.
- Verified against the real wiki on 2026-11-04 with an `#[ignore]`d test (`cargo test -p l8-core live_pcgamingwiki -- --ignored`): Sekiro found by Steam id, Hollow Knight by title, an unknown title returns nothing, and the Sekiro page's save template parses and expands.
- Not done: AI-assisted research stays deferred by decision.

## Phase 6 notes

- Delivered: OAuth 2.0 (code + PKCE S256, loopback redirect, `drive.file` scope only) with the user's own Google client, Drive v3 client (folders, paged queries, resumable chunked uploads with resume, streamed downloads), the sync engine (upload queue with exponential backoff capped at 1 h, resume after crash, pause on sign-in problems, checksum-verified downloads, import of a new PC's history from Drive `appProperties`, lineage-based conflict detection, three resolutions), the Cloud page, per-snapshot cloud badges and "Drive only" snapshots that download on restore.
- Deviation: `oauth2` and `reqwest` were replaced by a small PKCE/loopback implementation on `ureq` (fewer dependencies, synchronous core).
- Not verified here: everything that talks to Google (sign-in, real uploads, quota) needs the user's OAuth client; the engine and the Drive client are covered by tests with an in-memory Drive and scripted HTTP.

## Phase 7 notes

- Delivered: notification-area icon with Open / Sync now / Quit, close-to-tray, start with Windows (current user, hidden), Windows notifications for failures, conflicts and sign-in problems, first-run tour, General / Libraries / Save Detection settings, tighter CSP, lazy-loaded pages, accessibility check (axe, WCAG 2 A/AA, no violations on the main pages in both themes), `LICENSE` (MIT), security review, release and SignPath instructions, NSIS installer build.
- Left to the maintainer: creating the public repository, applying to SignPath Foundation, and the optional auto-updater (needs a project signing key; see docs/release-and-signing.md).

## Order rationale

- Steam artwork from Steam's CDN arrives in phase 2 because it costs almost nothing and makes the library look like the design from day one.
- Local save protection (3) is the product's core value, so it comes right after the library.
- Cloud (6) comes after the local engine because Drive only stores what the local engine already guarantees.

## Definition of done (every phase)

- `bun x tsc -p tsconfig.json --noEmit`, `bun run build`, `cargo clippy -- -D warnings` and `cargo test` pass.
- New services have unit tests; snapshot and restore code has integration tests on temp directories.
- Every user-facing string is in the Lingui catalogues (EN + FR).
- No secret in code, logs or the DB.
- Docs updated: this folder and the root README.
- Commits follow `<type>(<scope>): <message>` on `feature/*` branches. Nothing is committed or pushed without your go-ahead.
