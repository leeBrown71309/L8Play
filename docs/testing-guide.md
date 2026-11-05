# Hands-on testing guide (phases 1 to 8)

What was verified by automated tests is listed in each phase's notes in `architecture/08-roadmap.md`. This guide is what only a person on a real PC can check. **Back up important saves yourself before the first restore test, and test restore on a game you do not care about.**

Prerequisites per phase are marked **Needs**.

## Phase 1 — Desktop shell

- Start the app: custom title bar, drag, double-click maximize, minimize/maximize/close buttons, Windows 11 snap layouts.
- Switch theme (Ember / Rosé) and language (English / Français) in Settings › Appearance; restart and check both are remembered.
- Collapse and expand the navigation rail with its toggle; restart and check it is remembered.
- Check the database exists at `%APPDATA%\com.l8play.app\l8.db` and logs at `%LOCALAPPDATA%\com.l8play.app\logs`.

## Phase 2 — Library and launching

- **Needs** Steam and/or Epic installed with games.
- Games appear with correct titles, install paths and (Steam) covers; the playtime shown matches Steam.
- Play a game from the grid, the list, the details page, the dashboard and the quick rail: it launches through its launcher; "Playing now" appears; a toast shows the session length when you quit.
- **Epic Games**: games installed through the Epic launcher (for example Overcooked! 2, Rocket League) appear with platform Epic; Play launches them through the launcher.
- **Xbox app and GOG**: "Scan libraries" (and the startup scan) also lists games installed by the Xbox app (the `XboxGames` folder of each drive, or the folder named by the drive's `.GamingRoot` file, for example Minecraft Dungeons II) and GOG games (read from the `GOG.com` registry keys, for example Moonlighter). A game you added by hand that turns out to belong to one of them keeps its id, artwork, saves and snapshots and switches from Standalone to Xbox or GOG; it is never duplicated. Xbox games start through their package (the Start menu way), GOG games through their `.exe`. Detection is read-only; EA app, Ubisoft Connect and Battle.net are not detected (add those games with "Scan a folder").
- **Games you own but haven’t installed (Steam)**: Settings › Game Libraries › "Show games I own but haven’t installed" (on by default). A scan then reads the game list of your **public** Steam profile (`steamcommunity.com`; no password or API key) and adds the games you own without an installation: they appear with the Steam tag, "Not installed", and their Play button reads Install (it opens Steam's install page, `steam://install/<id>`). Tools, dedicated servers, soundtracks, SDKs and demos are skipped, and nothing already in the library is removed or renamed. Steam now often hides the list from apps without a key (its anonymous pages answer with a sign-in page), so the reliable way is your **own free Steam Web API key**: Settings › Game Libraries › "Use my Steam Web API key" (it opens when Steam asks for a key), open the key page (steamcommunity.com/dev/apikey, any domain name works, no payment), paste the 32-character key and press Save key. It is stored in the Windows Credential Manager (entry `L8Play`/`steam-web-api`), never shown again and never written to the logs (check `%LOCALAPPDATA%\com.l8play.app\logs` for the key text: it must not be there); saving it rescans at once. Your Steam profile's **Game details** must be public (Steam › Profile › Edit profile › Privacy settings) or Steam answers with an empty list ("Your Steam profile’s game details are private…"). A wrong key shows "Steam refused the key"; Remove the key goes back to the anonymous attempt. Turn the setting off to stop all requests. Epic, GOG and Xbox are not listed (they need a sign-in).
- "Add a game" › **Scan a folder**: pick the folder that holds your games (for example `D:\Workspace\Games`). Each game folder is listed with its programs (installers, crash handlers and redistributables are hidden) and the most likely one is pre-selected; change it for games that have several (e.g. `tlou-i.exe` vs `launcher.exe`), rename or untick games, then add. Scan the same folder again: games already in the library are not offered again, only new ones. "Add a game" › **Choose one program** keeps the single `.exe` picker.
- Add a game with its `.exe` ("Add a game"); favourite a game; search with Ctrl+K; filter by platform and by installed.
- Uninstall (or move) a game and rescan: it is marked not installed but keeps its history.

## Phase 3 — Local saves

- **Needs** a game Ludusavi knows (most popular games).
- Open a game › Saves: locations are detected (Settings › Save Detection shows the Ludusavi tool as Available).
- Create a snapshot with a name; check the zip appears under `Documents\L8Play\Snapshots\<game>\`.
- Change the save in the game, create another snapshot, **close the game**, restore the first one: the dialog explains what happens, a "Safety copy before restore" snapshot appears, and the game loads the older save.
- Try Create/Restore while the game runs: both are refused with a clear message.
- Quit a game: an automatic snapshot appears a few seconds later (unless nothing changed). Turn on "Before launching" and check one is taken on Play.
- Pin a snapshot, set "automatic snapshots kept" to 5, play several times: only unpinned automatic ones are pruned.
- Verify integrity on a snapshot; then damage a zip with a text editor and verify again: it is flagged as damaged and restore refuses it.
- Change the snapshot folder in Settings › Backup: new snapshots go there, old ones stay.

## Phase 4 — Artwork and metadata

- Pick an image in the Artwork tab: it is applied at once (toast), the preview, the page banner, the logo and the game's icon update. The page banner shows hero, logo and now the icon; the quick-launch rail on the right shows each game's **icon** (the cover when a game has no icon).

- Without any key: Steam games show Steam art, others show generated art; Artwork tab › "Use my own image…" works with a PNG/JPG; "Back to default" undoes it.
- Properties tab: rename a game (e.g. a misread title), set developer/year; run "Scan libraries": the edit survives.
- **Needs** a free SteamGridDB account and API key (steamgriddb.com › Preferences › API): paste it in Settings › Artwork, "Test the key", then in a game's Artwork tab search, pick an image per type (cover, hero, logo, icon). Add an Epic/standalone game and check it gets art automatically (or press "Find now"). Remove the key and check everything still works.
- Check the key appears in Windows **Credential Manager** under `L8Play/steamgriddb` and is not in any file under `%APPDATA%\com.l8play.app` or in the logs.

## Phase 5 — Find saves

- **Needs** a game Ludusavi does not know (e.g. an indie/older game, or a game added by `.exe`). Internet for PCGamingWiki.
- Game › Saves › "Find saves": candidates show the evidence (wiki template or matching folder name) and a "Read the wiki page" link; "Use this folder" adds it; it then appears under Save locations and snapshots work.
- Disconnect the network and search again: a notice says the wiki was unreachable and local results still show.
- Try to add a broad folder (e.g. your Documents) with "Add folder": it is refused.

## Launching the app for testing

Start `target\release\l8-play.exe` (or the installed app) **yourself, from Explorer or the Start menu**. Do not rely on an app started from inside another sandboxed program (for example a packaged automation tool): Windows can redirect such a process's `AppData` writes into that program's private package folder, so it sees a different, separate copy of your database and artwork. Data written that way does not appear when you start the app normally.

## Phase 6 — Google Drive (most setup)

- **Needs** a build that contains the project's Google client (`L8_GOOGLE_CLIENT_ID` / `L8_GOOGLE_CLIENT_SECRET` provided at build time, see `release-and-signing.md`). Without it, use **Advanced: use my own Google project** as described in `google-drive-setup.md`.
- Cloud Storage shows **Connect Google Drive** directly (no form). The text says L8Play only accesses the files it creates in an **L8Play** folder. Press it: your browser opens, choose your account. Google may show "unverified app": Advanced › Continue. The tab says it is connected and the app shows your email and storage bar.
- Open **Advanced: use my own Google project**: it states which client is used (built-in or yours); while connected it asks you to disconnect first. Disconnect, save a custom client, check it is used, press **Use the default client**, check you are back on the built-in one. None of this touches local snapshots or Drive files.
- Check the secret never shows in the browser address bar of the Google page, in `%LOCALAPPDATA%\com.l8play.app\logs`, or anywhere in the app window.
- Check a **L8Play** folder (no space) appears in Drive with a sub-folder per game; new snapshots show "Waiting for Drive" then a "Drive" badge. "Sync now" works; the Queue list is empty when done.
- Go offline during an upload (or kill the app mid-upload), restart: the upload resumes/retries (queue shows Failed with a retry time, then succeeds).
- Delete a synced snapshot locally: it stays listed as "Drive only"; Restore downloads it (verified) and restores.
- **Retention reaches Drive.** Settings › Backup: "Keep on this PC" (last N automatic snapshots per game) and "Keep in Google Drive" (last M, or "same as this PC"). After the next automatic snapshot, the oldest ones leave this PC, and those past the Drive limit are also deleted from Drive within ~30 s (check the game's folder in Drive). Drive never keeps fewer than the PC: a lower value behaves like the PC limit. Manual and pinned snapshots, safety copies and snapshots in a sync conflict are never removed. Offline or signed out: the Drive deletion waits and runs after the next successful sync. "Delete from PC and Drive" in the delete dialog removes the Drive copy too; plain "Delete" keeps it.
- **Needs a second PC (or a second Windows user with a separate app data folder):** install, add the same client, connect the same account: Drive snapshots are imported for games present on that PC. Restore one.
- Conflict test: on both PCs, play and snapshot the same game while **offline**, then go online and sync both: a conflict appears (toast + Cloud page + warning in the quick rail). Try each choice on test data: Keep this PC's / Use the other's / Keep both. Nothing is deleted from Drive.
- Revoke the app at <https://myaccount.google.com/permissions> and wait for the next sync: the app shows "Sign-in needed" (and a Windows notification when the window is hidden); "Sign in again" restores it.
- Disconnect: access is revoked, local snapshots and Drive files remain.
- Check the client secret and refresh token appear in Credential Manager only (`L8Play/google_*`).

- **Where a snapshot is kept.** With Google Drive connected, Create snapshot offers **On this PC and in Google Drive** (default), **On this PC only** (never uploaded, even by Sync now or after reconnecting) and **In Google Drive only** (created on this PC, uploaded, and its local archive is removed only once Drive has confirmed the upload; if the upload keeps failing the local copy stays and the upload is retried). Without Drive only **On this PC** is offered, with a link to Cloud Storage. A Drive-only snapshot is restored by downloading it first.

## Phase 7 — Finalization

- Close the window with the X: the app stays in the notification area (tray); left-click the icon reopens it; right-click menu has Open / Sync now / Quit; Quit really exits.
- Turn "Keep running in the notification area" off: the X quits the app.
- Settings › General › Start with Windows: sign out/in (or restart): the app starts hidden in the tray. Turn it off afterwards.
- With the window hidden, quit a game: an automatic snapshot is still taken. Windows notifications appear for a failed backup, a conflict or a Drive sign-in problem (and not when the window is focused).
- Welcome tour: appears on a fresh profile (delete `%APPDATA%\com.l8play.app\l8.db` to simulate), can be skipped with Esc, and is replayable from Settings › About.
- Installer: run `bun tauri build`, install the generated `*-setup.exe` (per-user, no admin prompt), start from the Start menu, then uninstall; check `%APPDATA%\com.l8play.app` is kept (your data) and that SmartScreen warns only because the installer is unsigned (see `release-and-signing.md`).

## Phase 8 — In-app updates

- **Needs** an installed build that contains the updater (0.1.3 or later) and internet access.
- **First updater release (one time):** install `L8Play_0.1.3_x64-setup.exe` by hand. Version 0.1.2 has no updater and cannot update itself; every later release is offered inside the app.
- Open **Settings › Updates**. The current version is shown and the status reads **Up to date** or **Update available**. **Check for updates** works even when automatic checks are off.
- **End-to-end:** publish a newer release (for example 0.1.4) with its installer, `.sig` and `latest.json` (see `release-and-signing.md`). Within six hours, or immediately with **Check for updates**, the section shows the new version, its publication date and notes. The Settings button in the navigation rail gets a dot and a non-blocking toast appears once per version.
- Press **Update now**: the progress bar streams the download, then L8Play restarts on the new version. Check the version in **Settings › About**. Nothing is downloaded or installed until you press the button.
- **Errors:** disconnect the network and check: the section shows "GitHub could not be reached" with **Retry**. Publish a manifest with a bad signature (or tamper with the installer) and press **Update now**: the download is rejected with "The update signature is invalid" and nothing is installed.
- **Busy warning:** start a game (or leave a Drive upload in the queue) and press **Update now**: a dialog warns that the restart may interrupt it, with **Later** and **Update anyway**.
- **Automatic checks:** turn **Check for updates automatically** off, restart the app and confirm the status stays **Not checked yet** (no request is made). Manual checks still work; turn it back on afterwards.
- **Browser preview:** run `bun run dev`, then open `http://localhost:1420/?update=available#/settings?section=updates`. Use `?update=uptodate`, `?update=checking`, `?update=downloading`, `?update=ready`, `?update=error` or `?update=idle` to force the other states; the data is simulated and nothing is downloaded.
- **Key safety:** the private updater key and its password live outside the repository (see `release-and-signing.md`). Confirm neither appears in the repository, the installer, `%LOCALAPPDATA%\com.l8play.app\logs`, or the app window.
