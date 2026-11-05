# L8Play

A free Windows gaming hub: one library for Steam, Epic and standalone games, automatic save snapshots, and optional Google Drive backup. Built with Tauri 2, Rust, React and TypeScript.

> **Status:** Phases 1–7 are implemented (feature complete, awaiting your hands-on testing): game library, local snapshots with safe restore, artwork and metadata, extended save detection (PCGamingWiki and local search), Google Drive sync with conflict handling, tray, notifications, welcome tour and an installer. Steam and Epic games are found automatically, standalone games can be added by their `.exe`, and games launch through their launcher. Save locations are detected with the bundled Ludusavi (or added by hand), verified zip snapshots are taken when a game closes (optionally before launch), and any snapshot can be restored with an automatic safety copy and full rollback on failure. Phase 4 adds artwork (Steam CDN, optional SteamGridDB with your own free key kept in the Windows Credential Manager, your own images) and metadata editing. Phase 5 adds "Find saves" for games the Ludusavi database does not know, phase 6 optional Google Drive sync (your own free Google project, see [docs/google-drive-setup.md](docs/google-drive-setup.md)), and phase 7 the tray icon, notifications, first-run tour, security review ([docs/architecture/10-security-review.md](docs/architecture/10-security-review.md)) and release notes ([docs/release-and-signing.md](docs/release-and-signing.md)). See [docs/architecture](docs/architecture/README.md) for the full plan.

## Requirements (Windows 10/11, x64)

| Tool | Version | Notes |
| --- | --- | --- |
| [Rust](https://rustup.rs) | stable, MSVC toolchain | `rustup default stable-msvc` |
| Visual Studio Build Tools | 2022 or newer | "Desktop development with C++" workload |
| [Bun](https://bun.sh) | 1.x | package manager and script runner |
| WebView2 Runtime | any | preinstalled on Windows 10/11 |

## Getting started

```bash
bun install
```

```bash
bun tauri dev
```

`bun tauri dev` starts Vite on port 1420 and opens the native window. `bun run dev` alone opens the interface in a browser against an in-memory preview backend, with no database or file access. That mode is useful for design work.

## Scripts

| Command | What it does |
| --- | --- |
| `bun tauri dev` | Run the desktop app with hot reload |
| `bun tauri build` | Build the Windows installer (NSIS, per-user install) |
| `bun run build` | Type-check and build the frontend |
| `bun run typecheck` | TypeScript check (TypeScript 7) |
| `bun run lint` | ESLint |
| `bun run test` | Frontend unit tests (Vitest) |
| `cargo test --workspace` | Rust tests; also regenerates `src/lib/bindings` (ts-rs) |
| `cargo clippy --workspace --all-targets -- -D warnings` | Rust lints |
| `bun run tokens` | Regenerate `src/styles/tokens.css` from `design/tokens.json` |
| `bun run i18n:extract` | Extract UI strings and apply French translations from `scripts/translations/fr.json` |

## Project layout

```
crates/l8-core/      Business logic (no Tauri): SQLite, settings, platforms (Steam, Epic), library, launcher, sessions
src-tauri/           Desktop shell: window, typed commands, logging
src/                 React app
  app/               bootstrap, router, query client
  components/        design-system components (shell, ui, feedback)
  features/          one folder per screen
  lib/bindings/      TypeScript types generated from Rust — do not edit
  lib/tauri/         typed command wrappers + browser preview backend
  locales/           Lingui catalogues (en, fr)
design/              design-system tokens and app icon source
docs/architecture/   Phase 0 architecture, decisions, roadmap, risks
```

## Conventions

- Every user-facing string goes through Lingui (`t` / `<Trans>`) and must exist in English and French. Run `bun run i18n:extract` after adding text, then add the French text to `scripts/translations/fr.json`.
- Colours, radii, shadows and type come only from the design tokens. Tailwind's default palette is disabled.
- Rust types shared with the UI derive `ts_rs::TS` and are exported with `cargo test`.
- Data lives in `%APPDATA%\com.l8play.app\` (database) and `%LOCALAPPDATA%\com.l8play.app\logs\` (logs, secrets redacted).

## Licence

Free and non-commercial. The licence file will be added before the first public release (required for SignPath code signing). Save-location knowledge comes from the Ludusavi manifest and PCGamingWiki (CC BY-NC-SA 3.0).
