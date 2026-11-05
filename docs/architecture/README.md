# L8Play — Architecture (Phase 0)

The technical plan for **L8Play**, a free Windows gaming hub: universal game launcher, save detection and snapshot manager, automatic artwork, and optional Google Drive backup. Built with Tauri 2, Rust, React and TypeScript. The visual source of truth is the L8Play design system (Ember theme).

**Status:** approved on 2026-11-04 (see Decisions). Phases 1 (desktop foundation), 2 (game discovery and launch) 3 (local save management), 4 (artwork and metadata), 5 (extended detection), 6 (cloud sync) and 7 (finalization) delivered; awaiting hands-on testing. See 10-security-review.md, ../release-and-signing.md and ../google-drive-setup.md.

| # | Document | Content |
| --- | --- | --- |
| 1 | [01-feasibility.md](01-feasibility.md) | Verdict, module-by-module feasibility, Ludusavi integration choice |
| 2 | [02-system-architecture.md](02-system-architecture.md) | Layers, data locations, key flows (discovery, launch, detection, snapshot, restore, sync, AI) |
| 3 | [03-dependencies-and-licenses.md](03-dependencies-and-licenses.md) | Every dependency with its licence; data licences that constrain the product |
| 4–5 | [04-external-services.md](04-external-services.md) | Free-service inventory, fallbacks, quotas and terms per API |
| 6 | [05-database-schema.md](05-database-schema.md) | SQLite schema (migration 0001) |
| 7 | [06-rust-backend-structure.md](06-rust-backend-structure.md) | Cargo workspace and module tree |
| 8 | [07-frontend-structure.md](07-frontend-structure.md) | React project structure and rules |
| 9 | [08-roadmap.md](08-roadmap.md) | Phases 0–7 with exit criteria and spikes |
| 10 | [09-risks.md](09-risks.md) | Zero-cost risks, technical risks, assumptions |

## Decisions (approved 2026-11-04)

1. **Save data sources:** free, non-AI sources only for now: the Ludusavi CLI as a detection-only sidecar plus the Ludusavi manifest, PCGamingWiki and local discovery. Our own snapshot engine.
2. **AI (Gemini, Ollama):** deferred. Not part of the current roadmap. The `AiProvider` and `SearchSource` traits stay in the design, so AI can be added later without refactoring.
3. **Code signing:** SignPath Foundation (free for open-source projects). This requires a public repository under an OSI-approved licence before the first signed release (phase 7).
4. **Google Cloud project:** free project with **no billing account**, used only for the OAuth client; `drive.file` scope; token lifetime checked in the phase 6 spike.
5. **i18n:** every user-facing string is externalized with Lingui from phase 1, in English and French.
6. **Phase order:** optimized to reach a usable version fast: library and launch, then saves, then artwork, then cloud (see [08-roadmap.md](08-roadmap.md)).
7. **Licensing:** the app stays free and non-commercial, compatible with PCGamingWiki's CC BY-NC-SA data.

## Sources consulted (2026-11-04)

- Ludusavi: [repository](https://github.com/mtkennerly/ludusavi) (MIT), [command-line docs](https://github.com/mtkennerly/ludusavi/blob/master/docs/help/command-line.md), [API input schema](https://github.com/mtkennerly/ludusavi/blob/master/docs/schema/api-input.yaml)
- Ludusavi manifest: [repository and README](https://github.com/mtkennerly/ludusavi-manifest)
- Google Drive: [scope classification](https://developers.google.com/workspace/drive/api/guides/api-specific-auth), [usage limits](https://developers.google.com/workspace/drive/api/guides/limits), [unverified apps](https://support.google.com/cloud/answer/7454865)
- Gemini: [API terms](https://aistudio.google.com/docs/terms); free-tier analyses ([DEV](https://dev.to/hassann/gemini-api-free-tier-in-2026-limits-models-and-when-you-get-billed-4081), [aipromptshub](https://aipromptshub.co/limits/gemini-rate-limits-2026))
- PCGamingWiki: [API page](https://www.pcgamingwiki.com/wiki/PCGamingWiki:API) (blocked automated fetch), [API forum thread](https://community.pcgamingwiki.com/topic/4947-api-issues), [licence per Wikipedia](https://en.wikipedia.org/wiki/PCGamingWiki)
- SteamGridDB: [official Node client](https://github.com/SteamGridDB/node-steamgriddb), [npm package](https://www.npmjs.com/package/steamgriddb)
- Epic manifests and URI: [game-scanner crate source](https://docs.rs/crate/game-scanner/1.1.0/source/src/epicgames/item.rs), [game-detector crate source](https://docs.rs/crate/game-detector/latest/source/src/epic_games.rs), [blog write-up](https://jayd.ml/games/2020/05/16/epic-games-store-steam-libraries.html)
- OAuth "Testing" token expiry (community): [rclone forum](https://forum.rclone.org/t/7-day-reconnect-of-google-drive-oauth/50148), [n8n forum](https://community.n8n.io/t/can-one-use-google-sheets-longer-than-7-days-in-test-mode-without-manual-reconnecting/275424)
- Code signing: [Microsoft code signing options](https://learn.microsoft.com/windows/apps/package-and-deploy/code-signing-options), [Azure Artifact Signing pricing](https://azure.microsoft.com/pricing/details/trusted-signing/)
