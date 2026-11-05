# Releasing L8Play and getting it signed for free

L8Play is free, open source (MIT, see `LICENSE`) and has no paid service in its build or release chain.

## Build the installer

```bash
bun install
bun run fetch:ludusavi        # downloads the pinned Ludusavi release and verifies its SHA-256
bun run check                 # typecheck, lint, format check, tests
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
bun tauri build               # produces src-tauri/target/release/bundle/nsis/L8Play_<version>_x64-setup.exe
```

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

## Updates (deliberately not automatic yet)

An in-app updater needs a project-owned signing key pair and a place to host the update manifest (GitHub Releases is free). That decision, and keeping the private key safe, belongs to the maintainer, so the updater plugin is not enabled. Until then, updates are a new installer download; the per-user installer upgrades in place and keeps `%APPDATA%\com.l8play.app` (database) untouched.
