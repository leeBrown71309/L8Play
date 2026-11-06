# Backlog

Ideas and open work that are **not** done yet. Nothing here is committed to a release; priorities are decided as we go.
Items are grouped, not ranked. Move an item to the roadmap (`architecture/08-roadmap.md`) when it is scheduled.

## Waiting on a decision or an external step

- **Store accounts (Epic, Xbox, GOG; Steam sign-in).** The Settings › Game Libraries › Store accounts cards are an inert preview. Feasibility notes:
  - Steam: owned library through the user's own Web API key works today; an OpenID sign-in would only give the SteamID.
  - GOG: owned games can be read from the local GOG Galaxy database without signing in.
  - Epic: no official public API; the unofficial launcher API may break the terms of use.
  - Xbox / Microsoft: official Microsoft sign-in with a free Azure app registration, but the API does not list a full library.
  - Install buttons can use each store's deep link (`steam://install/…`, GOG Galaxy and Epic launcher links, Microsoft Store links); no paid service is needed.
- **Google verification.** `drive.file` is a non-sensitive scope. Check the Verification Center in Google Cloud Console; brand verification needs a public home page, a privacy policy and a domain the maintainer owns.
- **Windows code signing.** Removes the SmartScreen warning; certificates are paid, so only if wanted.

## Features

1. **Saves**
   - Import: single `.zip` or loose save file (the importer reads folders only); `.rar` / `.7z` archives.
   - Import: Steam cover for saves-only games without a SteamGridDB key (Steam store search by title to find the App ID).
   - Import: automatic "join to installed game" after a library scan (today the details page offers it when it sees a game with the same title).
   - Import: restore of imported snapshots from console mode (today it points to the desktop interface).
   - Export a snapshot as `.zip`; rename a snapshot; compare two snapshots (changed files).
   - Per-snapshot storage choice already exists (PC, PC and Drive, Drive only); show it in more places.
2. **Library**
   - Collections and tags (to finish, co-op, mood) with filters on desktop and in console mode.
   - Play statistics: time per week, most played, streaks (the data already exists).
   - GOG Galaxy owned-games import (see above).
3. **Console mode**
   - Fade-out transitions for views and dialogs (only entrance animations exist).
   - Artwork change from the console; on-screen keyboard for text fields.
   - Start in console mode option; launch-and-return behaviour while a game runs.
4. **Accessibility**
   - Keyboard shortcuts everywhere, high-contrast mode, adjustable text size (matters on a TV).
5. **Notifications**
   - Remind before launching a game that has no snapshot; warn when Drive is nearly full.
6. **Support**
   - "Export a problem report" button: logs and versions, no secrets, so bug reports are clean.
7. **Repair tools**
   - A "repair library" action (re-scan, re-fetch artwork, rebuild missing records), generalizing the artwork cache repair done at startup.

## Quality and release engineering

- GitHub Actions workflow that builds, signs and publishes releases (secrets already documented in `release-and-signing.md`).
- First-run onboarding checked end to end on a clean machine.
- End-to-end tests for the main flows (add game, snapshot, restore, sync conflict).
- Replace screenshots in the README with images that contain no third-party artwork.
- Purge old pull-request refs (GitHub support) if the previous author e-mail must disappear completely.
