# 04 — Free-service inventory and API limitations

Every service is optional except the local file system. "Fallback" is what the app does when the service is missing, unconfigured, rate-limited or down.

## Inventory

| Service | Used for | Account / key | Cost | Fallback |
| --- | --- | --- | --- | --- |
| Steam client (local files) | Library discovery, launching | None. Steam credentials are never requested. | Free | Manual games |
| Epic Games Launcher (local files) | Library discovery, launching | None. Epic credentials are never requested. | Free | Manual games |
| Ludusavi manifest (GitHub raw) | Save-location knowledge | None | Free | Last cached copy; layers 2–3 |
| Ludusavi CLI (bundled sidecar) | Save-location resolution | None | Free | Layers 2–3 |
| PCGamingWiki MediaWiki API | Save locations, identifiers, official links | None | Free | Cached responses; layers 3–4 |
| SteamGridDB API v2 | Covers, heroes, logos, icons | Free SteamGridDB account; the user generates their own API key | Free | Steam CDN images (Steam games), then generated placeholder art |
| Steam store CDN | Artwork fallback for Steam games | None | Free | Placeholder art |
| Steam Web API (`api.steampowered.com`) | Owned but not installed Steam games (read-only, optional setting) | The user's **own free Steam Web API key**, typed in the app and kept in the Windows Credential Manager; the SteamID64 comes from the local `loginusers.vdf`. Steam passwords are never requested. | Free | Anonymous Steam Community pages, then the list is simply not shown |
| Steam Community public profile | Same list without a key (best effort; Steam often answers with its sign-in page) | None | Free | The list is not shown |
| Google Drive API v3 | Cloud copies of snapshots | User's Google account; the OAuth client is defined in a Google Cloud project **with no billing account** | Free | Local-only snapshots (fully functional) |
| Ollama (localhost) | Optional AI extraction | None | Free (local hardware) | Detection without AI |
| Gemini API (free tier) | Optional AI extraction | User's own AI Studio key | Free tier only; never billed | Ollama, or no AI |

## Limitations per service

### Google Drive API

- **Scope:** `drive.file` only. Google classifies it as **non-sensitive**: basic OAuth verification only, and no CASA security audit, which only the restricted `drive` scope needs. The app can see only files it created. That's exactly the "L8Play" folder we need, and the user's other files stay invisible to it.
- **Quotas (Google's usage-limits page):** 1,000,000 quota units per minute per project; 325,000 per minute per user per project. Exceeding them returns `403 User rate limit exceeded` / `429`, which we handle with exponential backoff.
- **Cost:** "All standard use of the Google Drive API is available at no additional cost." The same page states overages are "planned to incur charges to your Google Cloud billing account later in 2026". **Mitigation:** the Cloud project must never have a billing account; the app also self-limits far below the quota (one upload at a time, batched metadata).
- **Storage:** the user's own Google storage (15 GB free, shared with Gmail and Photos). The app reads `about.get(storageQuota)` and warns before an upload would exceed it. Upload limit: 750 GB/day (irrelevant at save sizes).
- **OAuth for a desktop app:** installed-app flow, PKCE, loopback redirect `http://127.0.0.1:<random-port>`. The desktop client secret is not confidential by design. Tokens live in Credential Manager.
- **Publishing status:** refresh tokens of apps in "Testing" are reported to expire after 7 days (community reports; not yet confirmed in Google's docs), and Testing is capped at 100 test users. The plan is to publish the OAuth app "In production" with `drive.file` only. **Phase 6 spike** confirms token lifetime before Drive ships.

### SteamGridDB API v2

- Base URL `https://www.steamgriddb.com/api/v2`, `Authorization: Bearer <key>`. The endpoints used (search/autocomplete, games by Steam App ID, grids, heroes, logos and icons by `game` or `steam` id, with `styles` / `dimensions` / `mimes` / `nsfw` / `humor` filters) come from SteamGridDB's official client libraries. The documentation page is a script-rendered app that couldn't be read automatically, so a **Phase 3 spike** verifies each endpoint with a real key before coding.
- No published rate limit, but limiting exists (official clients handle "rate limited" responses). Policy: one request at a time, at most about 1 request/s, `Retry-After` honoured, and every result (including "no art found") cached with a retry date.
- The key belongs to the user. It's never shipped in the binary.

### PCGamingWiki API

- MediaWiki API at `https://www.pcgamingwiki.com/w/api.php`. **Verified live on 2026-11-04:** anonymous clients are *not* allowed to run `action=cargoquery` ("You don't have permission to run arbitrary Cargo queries"), so the app does not use it. Lookups use `https://www.pcgamingwiki.com/api/appid.php?appid=<Steam id>` (a 302 redirect to the game's page; a 200 means unknown), `action=query&titles=…&redirects=1` (exact title; `missing: true` when absent) and `action=opensearch` (accepted only when a result's normalized name equals the title). `action=parse&prop=wikitext` then returns the page whose `{{Game data/saves|Windows|…}}` templates list the save paths (placeholders such as `{{p|appdata}}` and `{{P|uid}}`).
- No authentication. No published rate limit was found. Policy: a descriptive `User-Agent` with contact URL, `maxlag=5`, one request at a time, and responses cached for 30 days (manual refresh available).
- Content licence: CC BY-NC-SA 3.0 (see 03).

### Ludusavi manifest

- `https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml`. Update check with `ETag` / `If-None-Match` (a 304 means unchanged), at most once a day.
- Tools must implement `schema.yaml`, not the strict schema, so new values don't break them. They should also support secondary `.ludusavi.yaml` manifests in a game's install folder.
- The CLI's `--api` flag gives JSON on stdout; stderr may still contain warnings. Some failures produce no JSON, so the wrapper checks for empty stdout and logs stderr. The `api` subcommand accepts `findTitle` requests (by names, Steam ID, GOG ID, normalized or fuzzy matching).

### Gemini API (optional)

- Free tier: no card needed. Per-model limits are no longer published in the docs; they appear per project in AI Studio and change over time. The app treats limits as unknown, sends at most one research request at a time, and stops on the first quota error.
- **Data terms (unpaid services):** submitted content and responses are used to improve Google products, and human reviewers may read them. Google advises not to send sensitive or personal information. The app sends only public source text and game identifiers.
- **Region restriction:** "You may use only Paid Services when making API Clients available to users in the European Economic Area, Switzerland, or the United Kingdom." **Gemini is therefore disabled by default for users in those regions**, with Ollama offered instead.
- Users must be 18+.
- The app never enables billing and never asks for payment details.

### Ollama (optional)

- Local HTTP API on `127.0.0.1:11434`. Nothing leaves the machine.
- Needs a model the user downloads. Recommended: a 7–8B instruction model with a permissive licence (for example Qwen2.5-7B-Instruct, Apache-2.0; check each model's licence because they differ). Needs about 8 GB RAM; runs slowly on CPU-only machines.

### Steam owned games (Web API, then Community pages)

- **Supported route:** `GET https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?key=<user key>&steamid=<SteamID64>&include_appinfo=1&include_played_free_games=1&format=json` (`response.games[]`: `appid`, `name`, `playtime_forever` in minutes). HTTP 401/403 = `invalid_key`; an empty `response` object = game details private (`private`). Same host rules as the others: https, redirects only inside the host, 4 MiB cap, 20 s timeout. The key is a query parameter: it is never logged or put in an error (see the security review).
- **Fallback (no key or the API failed):** the anonymous Community pages below. When they answer with the sign-in page the status is `needs_key`.
- Statuses: `listed` | `private` | `needs_key` | `invalid_key` | `unavailable` | `disabled`.

#### Anonymous Community pages

- Setting `libraries.showOwnedNotInstalled` (on by default; off = no request). After the local scan, one GET to `https://steamcommunity.com/profiles/<SteamID64>/games?tab=all&xml=1` (legacy XML: `appID`, `name`, `hoursOnRecord`), then, only if that is neither XML nor a login wall, `https://steamcommunity.com/profiles/<id>/games/?tab=all` whose page embeds the list as HTML-escaped JSON in `data-profile-gameslist` (`rgGames`). Only `steamcommunity.com` over https, redirects only within that host (at most 3), 4 MiB cap, 20 s timeout, at most 20,000 games, one request per second shared pace.
- **Verified live on 2026-11-05:** both URLs answer with Steam's "Sign In" page to a non-browser client, even for a profile whose privacy state is public, which the app reports as `needs_key`; hence the Web API route above. The XML and page parsers are covered by fixtures; they could not be exercised against a public-games profile on this machine.
- Result: games upserted with platform `steam`, `platform_game_id` = AppID, no installation row (so "not installed"); never deleted when they leave the list. Status exposed to the UI: `listed` | `private` | `unavailable` | `disabled`.

### Steam / Epic local integrations

- Steam: `libraryfolders.vdf` and `appmanifest_*.acf` are stable, widely parsed formats. Launch with `steam://rungameid/<appid>`.
- Epic: `*.item` manifests and the `com.epicgames.launcher://apps/<AppName>?action=launch&silent=true` URI are **undocumented** but used by several open-source launchers. Parsing is defensive: unknown fields are ignored, and a malformed file is skipped and logged, never fatal.
