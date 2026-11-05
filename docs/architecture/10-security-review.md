# 10 — Security review (phase 7)

Scope: the whole application as of phase 7. This is a review of the design and the code, not a third-party audit.

## Trust boundaries

| Boundary | Rule | Where it is enforced |
| --- | --- | --- |
| WebView → Rust | The WebView holds no secrets and no file-system access. It can only call the commands registered in `src-tauri/src/lib.rs`; the capability file grants window controls and the native file picker, nothing else (no `fs`, `shell`, `http` or `opener` permission). | `src-tauri/capabilities/default.json` |
| WebView content | CSP: `default-src 'self'`; images only from the app, the scoped asset protocol, Steam's CDN and SteamGridDB's CDN; no `connect-src` to the internet (every network call is made by Rust); `object-src`, `base-uri`, `form-action` and `frame-ancestors` are `'none'`. | `tauri.conf.json` |
| Asset protocol | Enabled only for `$LOCALAPPDATA/com.l8play.app/artwork/**`. | `tauri.conf.json` |
| Network | Rust only. Allow-listed hosts: artwork providers, PCGamingWiki, and Google's OAuth/Drive hosts. HTTPS only, redirects disabled for Google calls, bounded response sizes, one request at a time with a 1 s pace for artwork and wiki. | `artwork/http.rs`, `cloud/transport.rs` |
| Secrets | SteamGridDB key, the user's Steam Web API key (`steam-web-api`, 32 hex characters, validated before it is stored), Google client id/secret and refresh token live in the Windows Credential Manager (`L8Play`/`<name>`). Access tokens stay in memory. Nothing is stored in SQLite or logged; the log writer redacts bearer tokens, `key=` values and every registered secret. The Steam key travels as a query parameter, so: transport failures are scrubbed at the HTTP layer before they become messages, owned-library code logs only the kind of failure (never a URL or text), the key goes from the WebView to Rust once and only a boolean comes back, and a test proves a failing request leaves the key out of logs and errors. | `artwork/secrets.rs`, `artwork/http.rs`, `platforms/steam_owned.rs`, `cloud/oauth.rs`, `redact.rs` |
| Child process | Ludusavi runs read-only (`--preview`, `find --api`), with a pinned SHA-256 checked at fetch time, no console window, and a config folder inside the app data folder. | `saves/ludusavi.rs`, `scripts/fetch-ludusavi.ts` |
| Privileges | No administrator rights anywhere: per-user NSIS install, per-user startup entry, loopback OAuth redirect on `127.0.0.1` only. | `tauri.conf.json`, `cloud/oauth.rs` |

## Backup integrity

- Originals are only opened for reading while a snapshot is made (exclusive-read with sharing-violation detection).
- Archives are written to `.partial`, verified entry by entry, hashed, then renamed; a failed snapshot leaves nothing.
- Restore protects the current save first, stages next to the target, swaps with renames, verifies hashes, and rolls back every location on any error. Archive entry names are validated (zip-slip test), and deletions are limited to `.l8-` paths the engine created.
- Cloud: a local archive's SHA-256 is re-checked before upload; a downloaded archive's SHA-256 is checked before use and discarded on mismatch; nothing in Drive is ever overwritten or deleted by the app; diverged histories become conflicts that need the user's decision.

## Input validation

| Input | Validation |
| --- | --- |
| Save folders chosen by the user, found by PCGamingWiki or by local search | Must be an existing directory, not a broad root (profile, AppData, Documents, drive root, install folder…). Local search and PCGamingWiki only *propose*; confirming goes through the same validation again. |
| PCGamingWiki text | Parsed as data (templates, placeholders). Unknown placeholders (registry keys, other launchers) are dropped, never guessed. Titles containing quotes are not sent to the Cargo query. |
| SteamGridDB candidate URLs (come from the WebView) | `https` and an artwork-host allow-list, re-checked in Rust before download. |
| Images (provider or user file) | Magic-byte check (PNG, JPEG, WebP, ICO), 15 MB limit, content-addressed file names. |
| Google/Drive metadata | Snapshot and device ids must be UUIDs, checksums 64 hex characters; the local game is matched by slug and file names come from the local database, never from remote text. |
| OAuth redirect | `state` must match; requests without it are ignored; PKCE S256 with a 64-byte verifier. |
| Settings | Absolute snapshot folder, bounded retention count, validated metadata lengths. |

## Findings

| # | Finding | Severity | Status |
| --- | --- | --- | --- |
| 1 | `bun audit`: `minimatch` (ReDoS) pulled by ESLint 10, a development tool that never ships in the app | Low (dev-only) | Accepted; `bun audit fix` has no compatible upgrade yet, re-check on the next ESLint release |
| 2 | Google client secret of a desktop client is not confidential by Google's own design; it is still kept in the Credential Manager and out of the browser URL | Info | By design |
| 3 | Conflict resolution marks the losing snapshot as a conflict copy locally only; the Drive copy keeps its original `kind` property, so another device may raise its own conflict | Low | Documented limitation |
| 4 | Tray menu labels are fixed at start-up in the language chosen at that time | Cosmetic | Documented limitation |
| 5 | `cargo audit` was not run (not installed in the build environment) | — | Run `cargo install cargo-audit && cargo audit` before a public release |

## Performance notes

- Start-up work (library scan, save detection, artwork lookup) runs on a background thread; the window is usable immediately.
- Network work is paced and cached (negative cache for artwork, 30-day cache for wiki answers, 10-minute cache for Drive listings in the UI).
- Secondary pages are code-split (`lazy` routes), keeping the first load small.
- Library lists are not virtualized: they are fine up to a few thousand games; add virtualization if a user reports slowness.
