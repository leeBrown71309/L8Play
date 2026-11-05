# 10 — Risk assessment

Likelihood and impact: L = low, M = medium, H = high.

## Zero-cost risks (could violate the core constraint)

| # | Risk | L / I | Mitigation |
| --- | --- | --- | --- |
| Z1 | Google Drive API overage billing, which Google's limits page says is "planned … later in 2026" | M / H | No billing account on the Google Cloud project, ever (documented in setup). One transfer at a time, well under quota. On quota errors the app backs off and pauses. |
| Z2 | Gemini free tier: data used for product improvement and human review; unpaid use not allowed for API clients offered to EEA / UK / CH users; limits unpublished and changing | H / M | Off by default. Ollama is the recommended AI. Gemini needs the user's own key and an explicit terms screen, is blocked for EEA / UK / CH, and the app never enables billing. |
| Z3 | Windows code signing costs money | H / L | Ship unsigned (SmartScreen "More info → Run anyway" on first install), or apply to SignPath Foundation (free for qualifying OSS) if the repo goes public. |
| Z4 | Google OAuth verification for broader distribution | M / M | `drive.file` is non-sensitive: basic verification only, which needs a privacy-policy page (free on GitHub Pages). For personal use, the user's own Cloud project is enough. |
| Z5 | PCGamingWiki NC licence | L / M | Keep the product free and non-commercial; attribute in About and docs. |
| Z6 | A dependency changes licence | L / M | `cargo deny` and a JS licence check in CI with an allowlist (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, OFL, Unlicense, CC0). |

## Technical risks

| # | Risk | L / I | Mitigation |
| --- | --- | --- | --- |
| T1 | Data loss during restore | L / **H** | Mandatory safety snapshot first (abort if it fails), same-volume staging, quarantine + per-file rename, rollback on error, post-restore hash check, tests with injected failures. |
| T2 | Copying a save while the game writes it | M / H | Process check + 5 s stable-mtime window + exclusive-open probe; auto snapshots retry instead of copying a half-written file. |
| T3 | Wrong save location backed up (too broad, e.g. all of Documents) | M / H | Validator rejects user-profile roots, system folders, install roots without save-like files and paths > N files / > X GB without confirmation. AI candidates always need confirmation. |
| T4 | Epic manifest or URI changes (undocumented) | M / M | Defensive parsing, manual fallback, fixture tests from real files. |
| T5 | Ludusavi CLI output changes between versions | M / M | Pin the sidecar version; parse against the published API schema; contract tests on recorded outputs; upgrade deliberately. |
| T6 | SteamGridDB rate limiting or wrong matches | M / L | Serialized queue, cached negatives, Steam-ID matching first, manual picker. |
| T7 | Refresh token expiry or revocation | M / M | Detect `invalid_grant`, set `needs_reauth`, keep queueing locally, prompt to reconnect. |
| T8 | Sync conflicts across devices | M / H | Immutable uniquely-named remote files, lineage-based detection, explicit three-choice dialog, every version kept. |
| T9 | Saves in the Steam Cloud folder fighting with Steam's own sync | M / M | Restore warns that Steam Cloud may re-sync; we never write to Steam Cloud metadata. |
| T10 | Windows path edge cases (long paths, OneDrive-redirected Documents, reparse points, non-ASCII user names) | M / M | Known-folder APIs instead of string guesses; `\\?\`-aware canonicalization; tests with Unicode and long paths. |
| T11 | Custom title bar loses Windows 11 snap layouts | M / L | Spike in phase 1; fall back to native decorations styled with the theme if needed. |
| T12 | WebView2 missing on old systems | L / L | Tauri installer bootstrapper. |
| T13 | Antivirus flags an unsigned app spawning a sidecar | L / M | Sidecar signed only if we sign; keep the CLI's official binary unmodified; document the false-positive workaround. |
| T14 | Local AI too slow or too weak on modest PCs | M / L | AI is the last resort and optional; show expected hardware; timeouts. |
| T15 | Scope creep and long phases | M / M | Phase gates with your approval; spikes before commitments. |

## Assumptions to confirm

- Target is Windows 10 22H2+ / Windows 11, x64, single user per Windows account.
- You will create a free Google Cloud project (no billing) for the OAuth client. A step-by-step guide comes with phase 6.
- You will create a free SteamGridDB account and API key for artwork.
