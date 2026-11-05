# 03 — Dependencies and licences

Versions are pinned at implementation time to the current stable release. Licences marked ✓ were confirmed from the project's own repository during Phase 0; the others are each project's long-standing published licence and get re-checked with `cargo deny` and `license-checker` in CI before release.

## Runtime and tooling

| Component | Purpose | Licence | Cost |
| --- | --- | --- | --- |
| Tauri 2 (+ official plugins) | Desktop shell, IPC, bundling | MIT / Apache-2.0 | Free |
| Microsoft Edge WebView2 Runtime | Web rendering on Windows | Proprietary freeware, redistributable (preinstalled on Windows 10/11) | Free |
| Rust stable toolchain | Backend | MIT / Apache-2.0 | Free |
| Bun | Package manager, scripts | MIT | Free |
| Node-compatible Vite toolchain | Frontend build | MIT | Free |
| Ludusavi CLI (sidecar) ✓ | Save-location resolution (preview only) | MIT, redistribution with licence notice | Free |
| Ludusavi manifest ✓ | Save-location knowledge | Tooling MIT; data compiled from PCGamingWiki (see below) | Free |
| Ollama (optional, user-installed) | Local AI runtime | MIT | Free (uses the user's hardware) |
| WiX / NSIS via Tauri bundler | Installer | MIT/BSD-style / zlib | Free |

## Rust crates (`l8-core`, `src-tauri`)

| Crate | Purpose | Licence |
| --- | --- | --- |
| `tauri`, `tauri-plugin-{dialog,opener,notification,shell,single-instance,log,updater,process}` | App, sidecar, native dialogs, notifications, signed updates and restart | MIT / Apache-2.0 |
| `rusqlite` (`bundled`) | SQLite access | MIT (SQLite itself: public domain) |
| `rusqlite_migration` | Versioned migrations | Apache-2.0 |
| `serde`, `serde_json`, `serde_yaml_ng` | Serialization (manifest is YAML) | MIT / Apache-2.0 |
| `thiserror`, `anyhow` (app edge only) | Errors | MIT / Apache-2.0 |
| `tokio` | Async runtime (already used by Tauri) | MIT |
| `reqwest` (`rustls-tls`, `json`, `stream`) | HTTP | MIT / Apache-2.0 |
| `oauth2` | OAuth 2.0 + PKCE | MIT / Apache-2.0 |
| `keyring` | Windows Credential Manager | MIT / Apache-2.0 |
| `steamlocate` | Steam install + libraries + app manifests | MIT |
| `winreg` | Registry reads | MIT |
| `sysinfo` | Process monitoring | MIT |
| `sha2`, `hex` | SHA-256 | MIT / Apache-2.0 |
| `zip` | Snapshot archives | MIT |
| `walkdir`, `globset` | Bounded directory walks, manifest globs | MIT / Unlicense, MIT / Apache-2.0 |
| `dunce` | Canonical Windows paths without `\\?\` surprises | CC0 / MIT / Apache-2.0 |
| `uuid`, `time` | Ids, timestamps | MIT / Apache-2.0 |
| `tracing`, `tracing-subscriber`, `tracing-appender` | Logs with redaction | MIT |
| `ts-rs` | Export DTOs to TypeScript | MIT |
| `governor` | Client-side rate limiting per provider | MIT |
| Dev: `tempfile`, `assert_fs`, `wiremock` | Tests | MIT / Apache-2.0 |

`tauri-specta` was considered for typed bindings. It's still pre-1.0 for Tauri 2, so `ts-rs` plus a small hand-written command map is the stable choice.

## Frontend packages

| Package | Purpose | Licence |
| --- | --- | --- |
| `react`, `react-dom` | UI | MIT |
| `typescript` (strict) | Types | Apache-2.0 |
| `vite`, `@vitejs/plugin-react` | Build | MIT |
| `tailwindcss` v4 | Styling with design tokens | MIT |
| shadcn/ui (copied source) + Radix UI primitives | Accessible primitives, restyled to L8 tokens | MIT |
| `lucide-react` | Icons | ISC |
| `zustand` | UI state | MIT |
| `@tanstack/react-query` | Caching command results, invalidation from events | MIT |
| `react-router` | Routing | MIT |
| `@tauri-apps/api`, plugin JS packages | IPC | MIT / Apache-2.0 |
| `sonner` | Toasts | MIT |
| Optional: `@lingui/*` | i18n (English + French) | MIT |
| Dev: `vitest`, `@testing-library/react`, `eslint`, `prettier` | Quality | MIT |
| Fonts: Outfit, JetBrains Mono (bundled locally, no CDN at runtime) | Typography from the design system | SIL Open Font License 1.1 |

The design system uses its own icon set. The implementation will use Lucide equivalents, as the technical brief asks, mapped one-to-one in `src/components/icons.ts`.

## Data licences that constrain the product

| Data | Licence | Consequence |
| --- | --- | --- |
| PCGamingWiki content (direct API use and, indirectly, the Ludusavi manifest) | CC BY-NC-SA 3.0, as reported by Wikipedia (PCGamingWiki's own page returned 403 to automated fetching; re-check by hand) | Free, non-commercial use is fine. Attribution in the About screen and docs. Rules cached from PCGW stay under the same licence if they are ever shared. **A paid edition would be incompatible.** |
| SteamGridDB artwork | User-uploaded art; the API requires a personal key | Stored only in the local cache for personal display; never redistributed with the app. |
| Steam store CDN images (fallback) | Valve / publishers' copyright | Cached locally for personal display only, as other launchers do; never bundled or redistributed. |
