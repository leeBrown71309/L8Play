# Releasing L8Play and getting it signed for free

L8Play is free, open source (MIT, see `LICENSE`) and has no paid service in its build or release chain.

## Build the installer

```bash
bun install
bun run fetch:ludusavi        # downloads the pinned Ludusavi release and verifies its SHA-256
bun run check                 # typecheck, lint, format check, tests
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
bun tauri build               # produces target/release/bundle/nsis/L8Play_<version>_x64-setup.exe (+ .sig when signing is configured)
```

A release that installed copies can update also needs `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (see "In-app updates" below). Without them the build still works, but it emits no updater signature.

The installer is per-user (`installMode: currentUser`): it needs no administrator rights and installs under `%LOCALAPPDATA%`.

## The Google client built into releases

Users connect Google Drive with one click because the build contains the project's Google OAuth client. It is **never written in the source or committed**: the build reads two variables.

| Variable | Value |
| --- | --- |
| `L8_GOOGLE_CLIENT_ID` | the OAuth client id (`….apps.googleusercontent.com`) |
| `L8_GOOGLE_CLIENT_SECRET` | its client secret |

**Local build.** Copy `.env.example` to `.env` (git-ignored) and fill both lines, or export the variables in the shell before `bun tauri build`. A real environment variable wins over `.env`. Cargo rebuilds `l8-core` when either changes.

**GitHub Actions.** Add two repository secrets (Settings → Secrets and variables → Actions) named `L8_GOOGLE_CLIENT_ID` and `L8_GOOGLE_CLIENT_SECRET`, then expose them to the build step only:

```yaml
- run: bun tauri build
  env:
    L8_GOOGLE_CLIENT_ID: ${{ secrets.L8_GOOGLE_CLIENT_ID }}
    L8_GOOGLE_CLIENT_SECRET: ${{ secrets.L8_GOOGLE_CLIENT_SECRET }}
```

**Without them** (a contributor's build, or a fork) everything compiles and works; Cloud Storage then asks for the user's own client (`google-drive-setup.md`, "Advanced").

**Things to know**
- A desktop client's secret is not confidential by Google's own design: anyone can extract it from the executable. It only identifies the app; it cannot read anyone's Drive without that person's consent. It is still kept out of the source, the logs (the log writer masks it), browser URLs and the WebView.
- Create the client as type **Desktop app**, in a Google Cloud project **without a billing account**, with only the `drive.file` scope.
- **Verification (to do later).** Until the consent screen is verified by Google, sign-in shows an "unverified app" warning and the project is limited to about 100 users. Verification needs a logo, a privacy policy page and a short demo video; it is not done yet. Users get past the warning with **Advanced → Continue**.

## Public repository checklist

1. Initialise the repository and push it to GitHub (public).
2. Keep `LICENSE` (MIT) at the root; keep `src-tauri/binaries/ludusavi-third-party-notices.txt` bundled (Ludusavi is MIT; its notices are shipped with the app).
3. PCGamingWiki-derived knowledge is CC BY-NC-SA 3.0: the project is non-commercial, and the About page credits the sources.
4. Do not commit `src-tauri/binaries/ludusavi-*.exe` (it is in `.gitignore`; `fetch:ludusavi` restores it).
5. Run `cargo install cargo-audit && cargo audit` and `bun audit` before each release.

## Code signing with SignPath Foundation (free for open source)

SignPath Foundation signs open-source projects at no cost. Requirements and flow:

1. The repository must be public and use an OSI-approved licence (MIT: done).
2. Apply at <https://signpath.org/> ("Apply for free code signing") with the repository URL, a short project description and the maintainer's contact.
3. Once approved, connect the repository's GitHub Actions workflow to SignPath (their GitHub integration signs the artifact produced by the build job), and publish the signed `*-setup.exe` as a GitHub Release asset.
4. Until the project is approved, the unsigned installer triggers Windows SmartScreen ("unknown publisher"). That is expected: users can choose "More info → Run anyway".

## In-app updates (Tauri updater + GitHub Releases)

L8Play uses `tauri-plugin-updater` with a fixed endpoint:
`https://github.com/leeBrown71309/L8Play/releases/latest/download/latest.json`.
Nothing is paid: GitHub Releases hosts the manifest and the installer, and the signing key pair is generated locally.

### The signing key (back it up)

- The key pair was generated once with `bun tauri signer generate -w "%USERPROFILE%\.tauri\l8play-updater.key"`. The private key lives **outside the repository** in `%USERPROFILE%\.tauri\l8play-updater.key`; its password is kept only in a password manager (never in a file on disk).
- **Back up the key file and keep its password in the password manager.** Losing either one means you can never sign an update that installed copies will accept. Generate a new pair only if you accept that every user must install a build manually again.
- The private key is never committed, printed, logged, put in `.env` or compiled into the app. The public key is not secret and is in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.

### Build a signed release

Shortcut: `pwsh -File scripts/release-build.ps1 -Version <x.y.z> -NotesFile release-notes.md` asks for the key password (kept in memory only), builds, and writes `latest.json`. It also deletes the raw `targetelease\l8-play.exe` afterwards, so search tools only find the installed app (`%LOCALAPPDATA%\L8Play`). The manual steps it performs are listed below.

1. Bump the version in `package.json`, `src-tauri/tauri.conf.json`, the root `Cargo.toml` and the `l8-core` dependency line in `src-tauri/Cargo.toml`.
2. Make sure the Google client variables are available (`.env`, see above).
3. Set the signing variables **in the shell only** (never in a file inside the repo):
   ```powershell
   $env:TAURI_SIGNING_PRIVATE_KEY = "$env:USERPROFILE\.tauri\l8play-updater.key"
   $secure = Read-Host "Signing key password" -AsSecureString
   $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [System.Net.NetworkCredential]::new("", $secure).Password
   ```
4. Stop any running `l8-play.exe`, then:
   ```bash
   bun tauri build
   ```
5. `bundle.createUpdaterArtifacts` makes Tauri write both:
   - `target/release/bundle/nsis/L8Play_<version>_x64-setup.exe`
   - `target/release/bundle/nsis/L8Play_<version>_x64-setup.exe.sig`

### Build `latest.json`

```bash
bun scripts/make-update-manifest.ts --version 0.1.3 --notes-file release-notes.md
```

The script reads the `.sig` file, uses the matching GitHub asset URL and writes `latest.json`:

```json
{
  "version": "0.1.3",
  "notes": "…",
  "pub_date": "2026-11-05T12:00:00.000Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<contents of the .sig file>",
      "url": "https://github.com/leeBrown71309/L8Play/releases/download/v0.1.3/L8Play_0.1.3_x64-setup.exe"
    }
  }
}
```

`notes` is plain text; the app strips tags and escapes it. Use the same neutral notes as the GitHub release. The script refuses an empty signature, an invalid version, insecure URLs and empty notes, so a broken manifest is not published.

### Publish

1. Create a **published** GitHub release tagged `v<version>` (not a draft, not a pre-release: `releases/latest` ignores both).
2. Attach the installer, its `.sig` and `latest.json`.
3. `releases/latest/download/latest.json` then points to the manifest. Keep older releases: installed copies keep working, and users can still download an installer.
4. Ask before deleting an older release.

### GitHub Actions secrets

If a release is built in CI, add these repository secrets (Settings → Secrets and variables → Actions):

| Secret | Value |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | the **content** of the private key file |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | its password |
| `L8_GOOGLE_CLIENT_ID` | the OAuth client id |
| `L8_GOOGLE_CLIENT_SECRET` | its client secret |

Pass them to the build step only:

```yaml
- run: bun tauri build
  env:
    TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
    L8_GOOGLE_CLIENT_ID: ${{ secrets.L8_GOOGLE_CLIENT_ID }}
    L8_GOOGLE_CLIENT_SECRET: ${{ secrets.L8_GOOGLE_CLIENT_SECRET }}
```

### What the updater enforces

- HTTPS only, one fixed GitHub endpoint; no telemetry and no user data beyond the request itself.
- The installer's minisign signature must match the embedded public key; the signature's trusted comment also carries the version.
- The plugin only accepts a **higher** version: a manifest with an older or equal version is ignored, so a compromised manifest cannot downgrade an install.
- On Windows the NSIS installer runs in `passive` mode and restarts the app when done.

### First updater-enabled version

Version 0.1.2 has no updater. Users must install **0.1.3** manually once; from then on, updates are offered in the app. Do not delete the 0.1.3 assets until every tester has installed it.
