# 01 — Technical feasibility assessment

Status: Phase 0 proposal, awaiting approval. Research date: 2026-11-04.

## Verdict

The product is feasible as a free, local-first Tauri 2 application. Every module can run without paid services. Five points need a decision or an early technical spike before the module that depends on them is built (see [09-risks.md](09-risks.md)):

1. **Gemini free tier** sends prompts to Google for product improvement and human review. Google's terms forbid offering unpaid Gemini to users in the EEA, Switzerland or the UK. **Recommendation:** Ollama (local) as the default AI provider; Gemini opt-in only, with the user's own key, and disabled in those regions.
2. **Google Drive API billing.** Standard use is free. Google's usage-limits page says quota overages are "planned to incur charges to your Google Cloud billing account later in 2026". **Recommendation:** the Google Cloud project used for OAuth must never have a billing account attached; the app treats quota errors as retryable and never escalates.
3. **Google OAuth publishing status.** Apps left in "Testing" are widely reported to get refresh tokens that expire after 7 days. Using only the non-sensitive `drive.file` scope keeps verification to the basic (free) level. **Spike:** confirm that a `drive.file`-only app in "In production" keeps long-lived refresh tokens.
4. **Code signing.** A Windows code-signing certificate is a recurring cost. **Recommendation:** ship unsigned builds (SmartScreen warns on first run), or apply to SignPath Foundation's free OSS program if the repository becomes public under an OSI licence.
5. **PCGamingWiki content licence** (CC BY-NC-SA 3.0, as reported by Wikipedia; PCGamingWiki blocked direct fetching). The Ludusavi manifest is compiled from it. Using it is compatible with a free, non-commercial app. Any future paid edition would be incompatible.

## Module-by-module

| Module | Feasible? | Approach | Main uncertainty |
| --- | --- | --- | --- |
| A — Game library | Yes | Steam: registry `SteamPath` → `steamapps/libraryfolders.vdf` → `appmanifest_*.acf` (the `steamlocate` crate covers this). Epic: JSON `*.item` files in `C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests`. Manual: user picks an `.exe`. No recursive drive scan by default. | Epic's manifest format is undocumented and could change. |
| B — Launcher | Yes | Steam `steam://rungameid/<appid>`. Epic `com.epicgames.launcher://apps/<AppName>?action=launch&silent=true` (used by several OSS launchers; not officially documented). Standalone: spawn the executable. Process monitor matches running executables under the install folder. | Steam/Epic start the game themselves, so exit detection relies on process polling (cheap, every 2–5 s while a session is active). |
| C — Artwork | Yes | SteamGridDB API v2 (free account API key supplied by the user, stored in Credential Manager). Fallback for Steam games: Steam's public store CDN images. Final fallback: generated placeholder art from the design system. | SteamGridDB publishes no rate-limit numbers; its official docs page did not render for verification, so endpoints come from its official client libraries. Spike in Phase 3. |
| D — Save detection | Yes | Layer 1 Ludusavi manifest (MIT tooling, PCGamingWiki data) via the Ludusavi CLI in preview mode as a sidecar; layer 2 PCGamingWiki MediaWiki API; layer 3 heuristic local discovery; layer 4 optional AI over retrieved sources only. Every candidate is validated on disk and needs user confirmation below a confidence threshold. | Ludusavi CLI behaviour per game must be verified on real data (spike). |
| E — Snapshots | Yes | Own Rust engine: read-only copy → staging → SHA-256 → zip → verify → atomic rename → DB commit. Restore always creates a safety snapshot first and rolls back on failure. | Some games keep save files open; quiescence checks (game process gone + stable mtimes + exclusive open) are required. |
| F — Google Drive | Yes, with conditions | OAuth 2.0 installed-app flow with PKCE and a loopback redirect; `drive.file` scope only; resumable uploads; immutable snapshot files; conflict detection by snapshot lineage. | Points 2 and 3 above. Drive storage is the user's shared 15 GB. |
| G — SQLite | Yes | `rusqlite` with bundled SQLite and versioned SQL migrations. Archives stay on disk; the DB keeps metadata only. | — |
| H — UX | Yes | React + Tailwind + shadcn/ui primitives restyled with the L8Play design-system tokens. Custom title bar (Tauri `decorations: false`). | A custom title bar loses some native Windows 11 behaviour (snap-layout flyout on hover over maximize) unless handled specifically. |

## Ludusavi: CLI sidecar vs. own implementation

| Option | Pros | Cons |
| --- | --- | --- |
| **A. Ludusavi CLI as a sidecar, preview mode only (recommended for detection)** | Reuses the full, maintained resolution logic (placeholders, Steam user IDs, store IDs, registry, secondary `.ludusavi.yaml` manifests). MIT licence allows redistribution with the licence notice. Stable machine output (`--api` JSON; schemas published). | Extra binary in the installer. Output schema changes need version pinning. |
| B. Use Ludusavi CLI for backup/restore too | Least code. | Its backup format, retention and naming do not fit named snapshots, cloud lineage, safety copies or our integrity model. |
| C. Re-implement the manifest resolver in Rust | No sidecar. | Re-creates complex logic; risk of silently missing saves. |

Recommendation: **A for detection, our own engine for snapshots.** Ludusavi is only asked "where are this game's files?" and never writes. We also read `manifest.yaml` directly (with `ETag` / `If-None-Match` caching as the manifest README asks) for title and Steam-ID lookups. Option C can replace A later behind the same `SaveDetector` trait if the sidecar becomes a problem.

## What stays free, by design

- No server: everything runs locally; the only remote storage is the user's own Google Drive.
- Every external provider is optional and has a fallback (table in [04-external-services.md](04-external-services.md)).
- No API key is ever shipped in the binary: SteamGridDB and Gemini keys are entered by the user and stored in Windows Credential Manager.
- The app never enables billing anywhere and shows the free-tier terms before a provider is turned on.
