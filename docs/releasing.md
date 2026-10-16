# Releasing Local Sights

A release is a tag `vX.Y.Z` on a commit of `main`. Pushing the tag starts the
[Release workflow](../.github/workflows/release.yml), which builds the macOS
(Apple Silicon) `.dmg` and the Windows (x64) installer and publishes both on
one [GitHub Release](https://github.com/kdnakt/local-sights/releases) right
away (not as a draft), only when both could be built. Installing them is
described in [install.md](install.md).

The macOS app is ad-hoc signed in CI (the build step passes
`APPLE_SIGNING_IDENTITY: "-"` to Tauri, so `codesign` signs it without a
certificate) and not notarized; it is not signed with an Apple Developer ID.
The workflow verifies that signature before publishing. The Windows installer
is not signed with a code-signing certificate (SmartScreen warns on the first
launch; install.md explains how to open it). The workflow uses only the
automatic `GITHUB_TOKEN`: no AWS credentials, Apple certificates, Windows
certificates or other secrets.

## What you need

- Node.js 22.18 or later (`npm run check:version` imports a TypeScript file
  directly with Node's built-in type stripping).
- Rust stable, for `cargo update -w`.
- Permission to push tags to the repository.
- For the checks before and after tagging: a Mac with Apple Silicon and a
  Windows 11 (x64) PC to install on.

## Versions

Versions follow [Semantic Versioning](https://semver.org/) and start at
`v0.1.0`. The tag must be exactly `v` + `MAJOR.MINOR.PATCH` (no pre-release or
build suffix, no leading zeros). The same version must appear in:

| File                        | Where                                               |
| --------------------------- | --------------------------------------------------- |
| `Cargo.toml`                | `version` in `[workspace.package]`                  |
| `Cargo.lock`                | the `local-sights` and `local-sights-core` packages |
| `package.json`              | `version` (and `package-lock.json`, kept in step)   |
| `src-tauri/tauri.conf.json` | `version`                                           |

`npm run check:version` checks them; `npm run check:version -- --tag vX.Y.Z`
also checks the tag. The normal CI runs the first form on every pull request,
and the Release workflow runs the second before building anything. Both
installers carry the same version in their file names
(`Local-Sights_X.Y.Z_aarch64.dmg`, `Local-Sights_X.Y.Z_x64-setup.exe`).

The next release is `v0.2.0`: the first one with the Windows installer, together
with the faster fetching merged before it. There is no `v0.1.2`. A tag that has
been published is never moved or reused.

## Steps

1. On a short-lived branch from `main`, raise the version (here to `0.2.0`):
   - `Cargo.toml`: `version = "0.2.0"` under `[workspace.package]`.
   - `package.json` and `package-lock.json`:
     `npm version 0.2.0 --no-git-tag-version`.
   - `src-tauri/tauri.conf.json`: `"version": "0.2.0"`.
2. Update `Cargo.lock` for the workspace crates: `cargo update -w`.
3. Check: `npm run check:version` must print the new version.
4. Commit, open a pull request to `main`, wait until every CI job is green,
   and squash-merge it.
5. Before tagging, try the build and the installers without publishing
   anything: in the **Actions** tab, run **Release dry run** ("Run workflow")
   on `main`. It builds the `.dmg` on the macOS runner and the Windows
   installer on the Windows runner, without a tag and without publishing, and
   keeps them as the `local-sights-dmg-dry-run` and
   `local-sights-windows-dry-run` artifacts for 7 days. Download both,
   install them on your Mac and on your Windows PC following
   [install.md](install.md), and check that the app starts, lists the log
   groups and fetches logs on each. (The dry-run `.dmg` is not ad-hoc signed,
   so on the Mac expect the "is damaged" dialog and use the Terminal fallback
   in install.md; the Windows installer is the same as the one the release
   makes.) A failure here is found before a tag exists; fix it through a pull
   request and run the dry run again.
6. Tag the merged commit of `main` and push the tag:

   ```bash
   git switch main
   git pull --ff-only
   npm run check:version -- --tag v0.2.0
   git tag v0.2.0
   git push origin v0.2.0
   ```

7. In the repository's **Actions** tab, follow the **Release** run for the tag
   (`verify` → `test` → `build-macos` and `build-windows` → `publish`). When it
   is green, open **Releases**: `v0.2.0` is published (not a draft) with two
   assets, one `aarch64` `.dmg` and one `x64-setup.exe`, and the notes name the
   commit it was built from. Earlier releases stay as they are.
8. Install the published `.dmg` on your own Mac following
   [install.md](install.md), launch it, and check that **About Local Sights**
   shows the new version and that a fetch works. On the first launch, macOS
   must show the "could not verify" warning and the app must open with the
   steps in install.md alone (**Done**, then **Open Anyway** in **Privacy &
   Security**), without Terminal. If macOS instead says the app "is damaged
   and can't be opened", the `.app` lacks the ad-hoc signature: treat the
   release as defective (the signature check in `build-macos` should have
   failed), do not hand it out, and release a fixed patch version.
9. Install the published `x64-setup.exe` on your Windows 11 PC following
   install.md: SmartScreen shows "Windows protected your PC", **More info** →
   **Run anyway** opens it, the installer needs no administrator rights, the
   installed apps list shows the new version, and a fetch works.

## When something fails

- **The tag has the wrong form** (for example `v1.2`), **the versions do not
  match**, or **the tagged commit is not on `main`**: the `verify` job fails
  with the reason, and nothing is built or published.
- **A test fails**: neither build job runs, and nothing is published.
- **One of the builds fails** (`build-macos` or `build-windows`): the other
  build may finish, but `publish` needs both and does not run, so nothing is
  published. A Release is created only after both installers have been built
  and checked, so a failed run leaves no Release behind.
- **The signature check fails** (`codesign --verify` rejects the `.app` inside
  the built `.dmg`, or `codesign -dvv` reports no `Signature=adhoc`):
  `build-macos` stops after the `.dmg` is built and `publish` does not run, so
  nothing is published. The build step must run with
  `APPLE_SIGNING_IDENTITY: "-"`.

To fix it, change `main` through a pull request as usual. Then release the
next patch version (simplest), or, **only when no Release exists for the
failed tag** (the run stopped before `publish`), move the tag to the fixed
commit:

```bash
git push --delete origin v0.2.0
git tag -d v0.2.0
git switch main && git pull --ff-only
git tag v0.2.0
git push origin v0.2.0
```

**A published release that turns out to be broken** (for example the Windows
app does not start, or a fetch fails on one OS) is never fixed by moving or
rebuilding its tag: people may already have installed from it, and a tag that
has been published stays as it is. Fix `main` through a pull request and
release the next patch version (`v0.2.1` after `v0.2.0`). Until then, point
people to the previous version, which stays on the releases page.

## Running the workflow again for the same tag

Re-running the Release workflow (for example after a network error) never
creates a second Release. If the tag's Release already exists, the run
replaces its `.dmg` and its Windows installer with the newly built ones
(`gh release upload --clobber`) and leaves the title and notes as they are. If
it does not exist yet, the run creates and publishes it.

Do not re-run for a tag whose Release people already installed from, unless
the rebuilt installers are meant to replace them: release a new patch version
instead.
