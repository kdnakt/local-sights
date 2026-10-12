# Releasing Local Sights

A release is a tag `vX.Y.Z` on a commit of `main`. Pushing the tag starts the
[Release workflow](../.github/workflows/release.yml), which builds the macOS
(Apple Silicon) `.dmg` and publishes it on
[GitHub Releases](https://github.com/kdnakt/local-sights/releases) right away
(not as a draft). Installing it is described in [install.md](install.md).

The app is ad-hoc signed in CI (the build step passes
`APPLE_SIGNING_IDENTITY: "-"` to Tauri, so `codesign` signs it without a
certificate) and not notarized; it is not signed with an Apple Developer ID.
The workflow verifies that signature before publishing, and uses only the
automatic `GITHUB_TOKEN`: no AWS credentials, Apple certificates or other
secrets.

## What you need

- Node.js 22.18 or later (`npm run check:version` imports a TypeScript file
  directly with Node's built-in type stripping).
- Rust stable, for `cargo update -w`.
- Permission to push tags to the repository.

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
and the Release workflow runs the second before building anything.

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
   - Optional, recommended before the first release or after changing the
     bundle settings: in the **Actions** tab, run **Release dry run** ("Run
     workflow") on the branch or on `main`. It builds the `.dmg` on the macOS
     runner without a tag and without publishing anything, and keeps it as the
     `local-sights-dmg-dry-run` artifact for 7 days. A failure here is found
     before you tag.
5. Tag the merged commit of `main` and push the tag:

   ```bash
   git switch main
   git pull --ff-only
   npm run check:version -- --tag v0.2.0
   git tag v0.2.0
   git push origin v0.2.0
   ```

6. In the repository's **Actions** tab, follow the **Release** run for the tag
   (`verify` → `test` → `build-and-publish`). When it is green, open
   **Releases**: `v0.2.0` is published (not a draft) with one `aarch64` `.dmg`,
   and the notes name the commit it was built from. Earlier releases stay as
   they are.
7. Install the published `.dmg` on your own Mac following
   [install.md](install.md), launch it, and check that **About Local Sights**
   shows the new version and that a fetch works. On the first launch, macOS
   must show the "could not verify" warning and the app must open with the
   steps in install.md alone (**Done**, then **Open Anyway** in **Privacy &
   Security**), without Terminal. If macOS instead says the app "is damaged
   and can't be opened", the `.app` lacks the ad-hoc signature: treat the
   release as defective (the signature check in `build-and-publish` should
   have failed), do not hand it out, and release a fixed patch version.

## When something fails

- **The tag has the wrong form** (for example `v1.2`), **the versions do not
  match**, or **the tagged commit is not on `main`**: the `verify` job fails
  with the reason, and nothing is built or published.
- **A test or the build fails**: `build-and-publish` does not run, or stops
  before publishing. The Release is created only after the `.dmg` has been
  built and checked, so a failed run leaves no Release behind.
- **The signature check fails** (`codesign --verify` rejects the built `.app`,
  or `codesign -dvv` reports no `Signature=adhoc`): `build-and-publish` stops
  right after the build, before the `.dmg` is checked, and nothing is
  published. The build step must run with `APPLE_SIGNING_IDENTITY: "-"`.

To fix it, change `main` through a pull request as usual. Then either release
the next patch version (simplest), or, since no Release exists for the failed
tag, move the tag to the fixed commit:

```bash
git push --delete origin v0.2.0
git tag -d v0.2.0
git switch main && git pull --ff-only
git tag v0.2.0
git push origin v0.2.0
```

## Running the workflow again for the same tag

Re-running the Release workflow (for example after a network error) never
creates a second Release. If the tag's Release already exists, the run
replaces its `.dmg` with the newly built one (`gh release upload --clobber`)
and leaves the title and notes as they are. If it does not exist yet, the run
creates and publishes it.

Do not re-run for a tag whose Release people already installed from, unless
the rebuilt `.dmg` is meant to replace it: release a new patch version instead.
