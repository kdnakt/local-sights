# Installing Local Sights on a Mac

This guide installs the released app (a `.dmg` from GitHub Releases) on a Mac,
for example a company PC. To build from source for development, see
[Run the app](../README.md#run-the-app) in the README instead.

## What you need

- A Mac with Apple Silicon (an M-series chip). Check with Apple menu →
  **About This Mac**: the **Chip** line starts with "Apple M". Intel Macs and
  Windows are not supported.
- The same AWS preparation as on your development machine: an AWS CLI-style
  configuration (`~/.aws/config`) with a default region for the profile you use,
  and a way to sign in (for SSO profiles, `aws sso login --profile <name>`). See
  [Prerequisites](../README.md#prerequisites) in the README. The app never
  reads, stores or logs your credentials; never put credentials in a document,
  a chat or this repository.

## Before you install: check your company's rules

The app is **ad-hoc signed** (signed in CI without a certificate) and is
**not signed with an Apple Developer ID certificate, and not notarized by
Apple**. macOS therefore cannot verify who made it and warns on the first
launch (see [First launch](#first-launch-the-could-not-verify-warning)).
Before installing it on a company PC, check for yourself:

1. Whether your company's device management (MDM) blocks apps from
   unidentified developers, or apps that are not notarized by Apple.
2. Whether installing software needs an application or approval first.

If such apps are not allowed, do not work around the rule. Stop here: the
release process has to be changed to sign the app with an Apple Developer ID
and notarize it first. If an application is needed, get the approval before
you continue.

## Download

1. Open the releases page: <https://github.com/kdnakt/local-sights/releases>.
2. Pick the version (the newest one is at the top, marked **Latest**).
3. Under **Assets**, download the `.dmg` whose name contains `aarch64` (for
   example `Local-Sights_0.1.0_aarch64.dmg`). Do not rely on the exact file
   name; there is one `.dmg` per release.

## Install

1. Open the downloaded `.dmg` (double-click it). A window shows
   `Local Sights.app`.
2. Drag `Local Sights.app` into one of these folders:
   - **`/Applications`** (the **Applications** folder in the Finder sidebar).
     macOS may ask for an administrator name and password.
   - **`~/Applications`** (the `Applications` folder inside your home folder),
     when you have no administrator rights. If it does not exist yet, create it
     in the Finder: **Go** → **Home**, then **File** → **New Folder** and name it
     `Applications`.
3. Eject the `.dmg` (the eject button next to it in the Finder sidebar) and
   delete the downloaded file if you like.

## First launch: the "could not verify" warning

Because the app is ad-hoc signed and not notarized by Apple, macOS warns on
the first launch: "Apple could not verify "Local Sights" is free of malware
that may harm your Mac or compromise your privacy" (on macOS 14 and earlier:
"can't be opened because Apple cannot check it for malicious software" or
"from an unidentified developer"). Open it once as follows, without Terminal;
later launches do not ask again.

**macOS 15 Sequoia and later**

1. Double-click `Local Sights.app`. When the warning appears, click **Done**.
   Do not move it to the Bin.
2. Open **System Settings** → **Privacy & Security**, and scroll down to the
   **Security** section. Next to ""Local Sights" was blocked to protect your
   Mac", click **Open Anyway**, and confirm with your password or Touch ID.
3. Double-click the app again and click **Open Anyway** in the dialog.

**macOS 14 Sonoma and earlier**

1. In the Finder, Control-click (or right-click) `Local Sights.app` and choose
   **Open**.
2. Click **Open** in the dialog.

**If macOS says the app "is damaged and can't be opened"**

That dialog offers no **Open Anyway**. It appears for an app without any
signature, which a current release does not ship: the Release workflow
verifies the ad-hoc signature before publishing. Expect it only from an old
release built before ad-hoc signing was added; install the newest release
instead. If it appears with a current release, report it as a defect of that
release (see [releasing.md](releasing.md)).

As a last resort, remove the download mark (quarantine attribute) in Terminal
and open the app again. Use the folder you installed into:

```bash
xattr -dr com.apple.quarantine "/Applications/Local Sights.app"
# or, for an install without administrator rights:
xattr -dr com.apple.quarantine "$HOME/Applications/Local Sights.app"
```

## Check the installed version

- In the app: the app menu → **About Local Sights** shows the version.
- In the Finder: select `Local Sights.app`, then **File** → **Get Info**; the
  **Version** line shows it.

## Update to a new version

There is no automatic update.

1. Quit Local Sights (app menu → **Quit Local Sights**, or Cmd+Q).
2. Download the new version's `.dmg` as in [Download](#download).
3. Open it and drag `Local Sights.app` into the same folder as before. When the
   Finder asks, choose **Replace**.
4. Open the app. If the "could not verify" warning appears again, follow
   [First launch](#first-launch-the-could-not-verify-warning).

Your settings and the disk cache are kept: they are stored outside the app, in
`~/Library/Application Support/dev.local-sights.app/` (settings) and
`~/Library/Caches/dev.local-sights.app/` (cache), which replacing the app does
not touch.

## Go back to an earlier version

Earlier releases stay on the releases page.

1. Quit Local Sights.
2. On <https://github.com/kdnakt/local-sights/releases>, find the earlier
   version and download its `aarch64` `.dmg`.
3. Install it the same way as an update (drag and **Replace**), then check the
   version as in [Check the installed version](#check-the-installed-version).

An earlier version is not guaranteed to read the settings or the cache written
by a newer one. If the older version reports that the cache cannot be read, or
behaves oddly with it:

1. Quit Local Sights.
2. Delete the cache folder `~/Library/Caches/dev.local-sights.app/` (in the
   Finder: **Go** → **Go to Folder…**, enter `~/Library/Caches/`, and move
   `dev.local-sights.app` to the Bin). Only cached logs are lost; they are
   fetched again from AWS the next time.
3. If the settings are the problem, also delete
   `~/Library/Application Support/dev.local-sights.app/settings.json`. It holds
   only the disk cache on/off setting, which you can set again in the app.
