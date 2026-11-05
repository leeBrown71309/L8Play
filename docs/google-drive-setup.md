# Connecting Google Drive

Drive sync is optional and free. L8Play never sees anything in your Drive except the files it creates itself (the `drive.file` scope), in a folder named **L8Play**.

## Normal path: one click

1. Open **Cloud Storage** and press **Connect Google Drive**.
2. Your browser opens Google's sign-in page: choose your account and accept.
3. Back in L8Play, your email and storage appear and uploads start.

A folder named **L8Play** appears in your Drive, with one sub-folder per game and one `.zip` per snapshot. The app listens on `http://127.0.0.1:<random port>` only for Google's answer, using the standard authorization-code flow with PKCE.

### "Google hasn't verified this app"

Until the project's Google consent screen has been verified, Google shows a warning page and limits the project to about 100 users. Choose **Advanced**, then **Continue to L8Play (unsafe)**. The warning is about the verification status, not about what L8Play can access: it is limited to the files it creates.

> This path needs a build that contains the project's Google client (see `release-and-signing.md`). A build compiled without it shows **Advanced: use my own Google project** open by default.

## Advanced: use your own Google project

Use this if you prefer to keep everything in a Google Cloud project you control, or if your build has no built-in client. The saves still go only to your own Drive and nobody can be billed.

> Never add a billing account to the project. The Drive API is free for this usage; without a billing account Google cannot charge anything.

1. Open <https://console.cloud.google.com/> and create a project (for example "L8Play").
2. **APIs & Services → Library**: enable **Google Drive API**.
3. **APIs & Services → OAuth consent screen**: user type **External**, app name "L8Play", your email as support and developer contact. Add the scope `https://www.googleapis.com/auth/drive.file` (the non-sensitive one).
4. **Publish the app** ("Publish app" → "In production"). In *Testing* mode Google expires refresh tokens after 7 days, which would sign you out every week. With only the `drive.file` scope no verification review is required for personal use.
5. **APIs & Services → Credentials → Create credentials → OAuth client ID**, application type **Desktop app**. Copy the **client id** and **client secret**.
6. In L8Play open **Cloud Storage → Advanced: use my own Google project**, paste both values (they are stored in the Windows Credential Manager) and press **Save my client**, then **Connect Google Drive**.

A custom client always wins over the built-in one. **Use the default client** removes it. Changing the client requires disconnecting first; disconnecting never deletes your local snapshots or the files already in Drive.

## Using a second PC

Install L8Play on the other PC and connect the same Google account (with the built-in client, or the same custom client). The first sync lists your Drive snapshots for games that exist in that PC's library. Restoring a Drive-only snapshot downloads it, checks its checksum, protects the current save, then restores it.

## If something goes wrong

| Symptom | Cause | Fix |
| --- | --- | --- |
| "Google refused the sign-in" right after connecting (own client) | Wrong client id/secret, or the client is not of type *Desktop app* | Advanced → save the right client |
| Warning page "Google hasn't verified this app" | The consent screen is not verified yet | Advanced → Continue |
| Signed out every week (own client) | The consent screen is still in *Testing* | Publish it to production (step 4) |
| "Sign-in needed" banner | The refresh token was revoked | Cloud Storage → Sign in again |
| Uploads fail with "storage is full" | Your Google storage is full (15 GB shared with Gmail and Photos) | Free space, then Cloud Storage → Retry failed |
| An old **L8 Play** folder (with a space) is in Drive | Created by an earlier test build | Delete it in Drive; L8Play now uses **L8Play** |
