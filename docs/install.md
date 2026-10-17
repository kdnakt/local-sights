# Installing Local Sights

This guide installs the released app from GitHub Releases on a company PC:
the `.dmg` on a Mac, or the `-setup.exe` installer on a Windows PC. Read the
section for your computer: [macOS](#macos) or [Windows](#windows). To build
from source for development, see [Run the app](../README.md#run-the-app) in
the README instead.

Both need the same AWS preparation as on your development machine: an AWS
CLI-style configuration (`~/.aws/config` on a Mac, `%USERPROFILE%\.aws\config`
on Windows) with a default region for the profile you use, and a way to sign
in (for SSO profiles, `aws sso login --profile <name>`, or the app's
**Sign in with SSO** button, which needs AWS CLI v2). See
[Prerequisites](../README.md#prerequisites) in the README. The app never
reads, stores or logs your credentials; never put credentials in a document, a
chat or this repository.

## macOS

### What you need

- A Mac with Apple Silicon (an M-series chip). Check with Apple menu →
  **About This Mac**: the **Chip** line starts with "Apple M". Intel Macs are
  not supported. For a Windows PC, see [Windows](#windows).
- The AWS preparation described at the top of this guide.

### Before you install: check your company's rules

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

### Download

1. Open the releases page: <https://github.com/kdnakt/local-sights/releases>.
2. Pick the version (the newest one is at the top, marked **Latest**).
3. Under **Assets**, download the `.dmg` whose name contains `aarch64` (for
   example `Local-Sights_0.2.0_aarch64.dmg`). Do not rely on the exact file
   name; there is one `.dmg` per release (next to the Windows installer).

### Install

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

### First launch: the "could not verify" warning

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

### Check the installed version

- In the app: the app menu → **About Local Sights** shows the version.
- In the Finder: select `Local Sights.app`, then **File** → **Get Info**; the
  **Version** line shows it.

### Update to a new version

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

### Go back to an earlier version

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

## Windows

### What you need (Windows)

- A PC with a 64-bit (x64) processor running a version of Windows 11 that
  Microsoft still supports. Check with **Settings** → **System** → **About**:
  **System type** reads "64-bit operating system, x64-based processor", and
  **Windows specifications** names the edition and version. Windows 10 and
  Windows on Arm (arm64) are not supported.
- The AWS preparation described at the top of this guide. The AWS
  configuration files are read from your user folder:
  `%USERPROFILE%\.aws\config` and `%USERPROFILE%\.aws\credentials` (or the
  files named by `AWS_CONFIG_FILE` and `AWS_SHARED_CREDENTIALS_FILE`).
- For **Sign in with SSO** from the app: AWS CLI v2, see
  [AWS CLI v2 on Windows](#aws-cli-v2-on-windows).

### Before you install: check your company's rules (Windows)

The Windows installer and the app in it are **not signed with a code-signing
certificate**. Windows therefore cannot verify who made the app and shows a
SmartScreen warning on the first launch (see
[First launch](#first-launch-the-windows-protected-your-pc-warning)). Before
installing it on a company PC, check for yourself:

1. Whether **Smart App Control** is on. When it is on, Windows does not run an
   app that is not signed with a certificate it trusts, and offers no way to
   run it anyway. Check it in **Windows Security** → **App & browser control**
   → **Smart App Control settings**: it must read **Off** (or **Evaluation**,
   which still allows the app). It can only be turned off, never back on
   without reinstalling Windows, so do not change it just for this app; ask
   first.
2. Whether your company's device management or policy blocks apps that are
   not signed with a certificate, or apps installed outside the Microsoft
   Store.
3. Whether installing software needs an application or approval first.

If such apps are not allowed, do not work around the rule. Stop here: the
release process has to be changed to sign the installer with a code-signing
certificate first. If an application is needed, get the approval before you
continue.

### Download (Windows)

1. Open the releases page: <https://github.com/kdnakt/local-sights/releases>.
2. Pick the version (the newest one is at the top, marked **Latest**).
3. Under **Assets**, download the installer whose name ends with
   `x64-setup.exe` (for example `Local-Sights_0.2.0_x64-setup.exe`). Do not
   rely on the exact file name; there is one Windows installer per release
   (next to the macOS `.dmg`).

The browser may warn that the download "isn't commonly downloaded". Keep it
(for example **…** → **Keep** in Microsoft Edge, then **Keep anyway**).

### Install (Windows)

1. Double-click the downloaded `-setup.exe`. If SmartScreen shows "Windows
   protected your PC", open it as described in
   [First launch](#first-launch-the-windows-protected-your-pc-warning); the
   same steps apply to the installer.
2. Follow the installer. It needs no administrator rights: the app is
   installed for your user account only, under
   `%LOCALAPPDATA%\Local Sights\`, and gets a Start menu entry.
3. If the PC has no WebView2 runtime (the browser engine the app draws its
   window with; Windows 11 ships it), the installer downloads and installs it
   first. This is the only download the installer makes.
4. Delete the downloaded `-setup.exe` if you like.

### First launch: the "Windows protected your PC" warning

Because the app is not signed with a code-signing certificate, Microsoft
Defender SmartScreen warns the first time the installer, and possibly the
installed app, is started: "Windows protected your PC. Microsoft Defender
SmartScreen prevented an unrecognized app from starting." Open it as follows;
later launches do not ask again.

1. In the dialog, click **More info** (in Japanese: 詳細情報). The publisher
   line reads "Unknown publisher"; that is expected.
2. Click **Run anyway** (in Japanese: 実行).

If the dialog shows neither **More info** nor **Run anyway**, or says the app
"was blocked by your administrator" or "is blocked by Smart App Control", the
PC does not allow apps that are not signed with a trusted certificate: see
[Before you install](#before-you-install-check-your-companys-rules-windows)
and do not work around it.

### AWS CLI v2 on Windows

**Sign in with SSO** in the app starts `aws sso login` of AWS CLI v2. Install
it with the official MSI installer from
<https://awscli.amazonaws.com/AWSCLIV2.msi> (see the
[AWS CLI installation guide](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html));
installing it needs administrator rights, so ask for them if you have none.

The app looks for the CLI in this order and starts the first one it finds:

1. `C:\Program Files\Amazon\AWSCLIV2\aws.exe` (where the official installer
   puts it; `C:\Program Files` is read from the `ProgramFiles` variable).
2. `aws.exe` in each folder of your `PATH`, in order, when the CLI was
   installed some other way. An app started from the Start menu gets the
   `PATH` of your user account; sign out and in again after changing it.

If neither is found, the sign-in dialog says the AWS CLI was not found; you
can also run `aws sso login --profile <name>` in a terminal and press
**Reload** in the app. The CLI is started without a command prompt window,
and it is stopped, together with any process it started, when you press
**Cancel**, after five minutes without approval, or when you close the app.

### Settings and cache on Windows

The settings and the optional disk cache live outside the installed app, under
your user profile:

| What              | Where                                  |
| ----------------- | -------------------------------------- |
| Settings          | `%APPDATA%\dev.local-sights.app\`      |
| Disk cache (logs) | `%LOCALAPPDATA%\dev.local-sights.app\` |

(`%APPDATA%` is `C:\Users\<you>\AppData\Roaming`, `%LOCALAPPDATA%` is
`C:\Users\<you>\AppData\Local`; paste either name into the File Explorer
address bar to open it.) The app leaves the access rights of these folders at
Windows's defaults for your user profile: readable by your own account,
administrators and the system, not by other users of the PC. Cached logs may
contain confidential information, so keep the cache off on a PC shared with
others or clear it from the app's settings.

### Check the installed version (Windows)

- **Settings** → **Apps** → **Installed apps**, find **Local Sights**: the
  version is shown under its name.
- The app itself has no **About** item on Windows (it has no menu bar there),
  so the installed apps list is the place to look.

### Update to a new version (Windows)

There is no automatic update.

1. **Close Local Sights first** (the close button, or Alt+F4). An installer
   run while the app is open cannot replace its files and asks you to close
   the app or fails.
2. Download the new version's `-setup.exe` as in
   [Download](#download-windows).
3. Run it. It replaces the installed version in place (no need to uninstall
   first) and keeps the Start menu entry.
4. Open the app. If SmartScreen warns again, follow
   [First launch](#first-launch-the-windows-protected-your-pc-warning).

Your settings and the disk cache are kept: they are stored outside the app
(see [Settings and cache on Windows](#settings-and-cache-on-windows)), which
replacing the app does not touch.

### Go back to an earlier version (Windows)

Earlier releases stay on the releases page.

1. Close Local Sights.
2. On <https://github.com/kdnakt/local-sights/releases>, find the earlier
   version and download its `x64-setup.exe`.
3. Run it. The installer accepts an earlier version over a newer one, so no
   uninstall is needed. Then check the version as in
   [Check the installed version](#check-the-installed-version-windows).

An earlier version is not guaranteed to read the settings or the cache written
by a newer one. If the older version reports that the cache cannot be read, or
behaves oddly with it:

1. Close Local Sights.
2. Delete the cache folder `%LOCALAPPDATA%\dev.local-sights.app\` (paste
   `%LOCALAPPDATA%` into the File Explorer address bar, then delete the
   `dev.local-sights.app` folder). Only cached logs are lost; they are fetched
   again from AWS the next time.
3. If the settings are the problem, also delete
   `%APPDATA%\dev.local-sights.app\settings.json`. It holds only the disk
   cache on/off setting, which you can set again in the app.

### Uninstall (Windows)

1. Close Local Sights.
2. **Settings** → **Apps** → **Installed apps**, find **Local Sights**, open
   its **…** menu and choose **Uninstall**. No administrator rights are needed.
3. The settings and the cache are left behind; delete the two folders listed
   in [Settings and cache on Windows](#settings-and-cache-on-windows) if you
   want them gone.
