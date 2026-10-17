// Tests for the ad-hoc signing checks (261012-adhoc-signing). The first
// part exercises the pure functions of signing.ts with short synthetic
// texts; the "repository" part applies them to the real workflow and
// documentation files, one test per requirement.
import { describe, expect, it } from "vitest";
import {
  findSignatureCheckStep,
  findUnsignedClaims,
  readTauriBuildSigningIdentity,
  terminalFallbackOnly,
} from "./signing";
// The real files, read at transform time with Vite's `?raw` import (as
// src/commandRegistration.test.ts does); the project has no @types/node.
import releaseYml from "../../.github/workflows/release.yml?raw";
import releaseDryRunYml from "../../.github/workflows/release-dry-run.yml?raw";
import installMd from "../../docs/install.md?raw";
import releasingMd from "../../docs/releasing.md?raw";
import readmeMd from "../../README.md?raw";

// Minimal workflow fragments. The real release.yml is read only by the
// "repository" tests below.
const BUILD_STEP = [
  "      - name: build the .dmg (ad-hoc signed)",
  "        env:",
  '          APPLE_SIGNING_IDENTITY: "-"',
  "        run: npm run tauri build -- --bundles dmg",
];

const CHECK_STEP = [
  "      - name: verify the signature",
  "        env:",
  "          DMG: ${{ steps.dmg.outputs.path }}",
  "        run: |",
  '          hdiutil attach -readonly -mountpoint "$mnt" "$DMG"',
  '          codesign --verify --deep --strict --verbose=2 "$mnt/Local Sights.app"',
  '          codesign -dvv "$mnt/Local Sights.app" 2>&1 | grep "Signature=adhoc"',
];

const DMG_STEP = [
  "      - name: exactly one Apple Silicon .dmg",
  "        run: |",
  "          dmgs=(target/release/bundle/dmg/*.dmg)",
];

const PUBLISH_STEP = [
  "      - name: publish the Release",
  "        env:",
  "          GH_TOKEN: ${{ github.token }}",
  "        run: gh release create",
];

function workflow(steps: string[][]): string {
  return [
    "name: Release",
    "jobs:",
    "  build-and-publish:",
    "    runs-on: macos-latest",
    "    env:",
    "      TAG: ${{ github.ref_name }}",
    "    steps:",
    "      - uses: actions/checkout@v7",
    ...steps.flat(),
    "",
  ].join("\n");
}

describe("readTauriBuildSigningIdentity", () => {
  // 1
  it("returns APPLE_SIGNING_IDENTITY from the env of the step that runs tauri build", () => {
    const text = workflow([BUILD_STEP, DMG_STEP, CHECK_STEP, PUBLISH_STEP]);
    expect(readTauriBuildSigningIdentity(text)).toBe("-");

    const unquoted = text.replace('APPLE_SIGNING_IDENTITY: "-"', "APPLE_SIGNING_IDENTITY: -");
    expect(readTauriBuildSigningIdentity(unquoted)).toBe("-");
  });

  // 2
  it("returns undefined when the build step has no such env, or no step runs tauri build", () => {
    const withoutEnv = workflow([
      ["      - name: build the .dmg", "        run: npm run tauri build -- --bundles dmg"],
      PUBLISH_STEP,
    ]);
    expect(readTauriBuildSigningIdentity(withoutEnv)).toBeUndefined();

    const otherEnv = workflow([
      [
        "      - name: build the .dmg",
        "        env:",
        "          CARGO_TERM_COLOR: always",
        "        run: npm run tauri build -- --bundles dmg",
      ],
    ]);
    expect(readTauriBuildSigningIdentity(otherEnv)).toBeUndefined();

    expect(readTauriBuildSigningIdentity(workflow([CHECK_STEP, DMG_STEP]))).toBeUndefined();
  });
});

describe("findSignatureCheckStep", () => {
  // 3
  it("finds the codesign --verify step between the .dmg step and the publish step", () => {
    const steps = [BUILD_STEP, DMG_STEP, CHECK_STEP, PUBLISH_STEP];
    expect(findSignatureCheckStep(workflow(steps))).toEqual({
      ok: true,
      value: { name: "verify the signature", checksAdhoc: true },
    });

    const withoutAdhoc = CHECK_STEP.filter((line) => !line.includes("Signature=adhoc"));
    expect(
      findSignatureCheckStep(workflow([BUILD_STEP, DMG_STEP, withoutAdhoc, PUBLISH_STEP])),
    ).toEqual({
      ok: true,
      value: { name: "verify the signature", checksAdhoc: false },
    });
  });

  // 4
  it.each([
    ["no codesign step", [BUILD_STEP, DMG_STEP, PUBLISH_STEP], "no step runs codesign --verify"],
    ["no build step", [DMG_STEP, CHECK_STEP, PUBLISH_STEP], "no step runs tauri build"],
    [
      "no .dmg step",
      [BUILD_STEP, CHECK_STEP, PUBLISH_STEP],
      "no step checks target/release/bundle/dmg",
    ],
    ["no publish step", [BUILD_STEP, DMG_STEP, CHECK_STEP], "no step publishes with gh release"],
    [
      "codesign before the build",
      [CHECK_STEP, BUILD_STEP, DMG_STEP, PUBLISH_STEP],
      "before the tauri build step",
    ],
    [
      // The 2026-10-12 v0.1.1 failure: Tauri had already deleted the .app.
      "codesign right after the build, before the .dmg is picked",
      [BUILD_STEP, CHECK_STEP, DMG_STEP, PUBLISH_STEP],
      "before the .dmg is picked",
    ],
    [
      "codesign after publishing",
      [BUILD_STEP, DMG_STEP, PUBLISH_STEP, CHECK_STEP],
      "after the Release is published",
    ],
  ])("fails with a reason: %s", (_label, steps, reason) => {
    const result = findSignatureCheckStep(workflow(steps));
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toContain(reason);
    }
  });
});

// 261015-windows-support (FR2.3, FR2.6): the release workflow builds the .dmg
// and the Windows installer in jobs of their own and publishes in a third.
// signing.ts reads the steps of the whole file in order, so the checks hold
// as long as the macOS build job comes before the Windows one and the
// publish job comes last.
const WINDOWS_BUILD_STEP = [
  "      - name: build the NSIS installer",
  "        run: npm run tauri build -- --bundles nsis",
];

const DOWNLOAD_STEP = [
  "      - uses: actions/download-artifact@v8",
  "        with:",
  "          name: release-dmg",
];

function job(name: string, runsOn: string, steps: string[][]): string[] {
  return [
    `  ${name}:`,
    `    runs-on: ${runsOn}`,
    "    steps:",
    "      - uses: actions/checkout@v7",
    ...steps.flat(),
  ];
}

function splitWorkflow(jobs: string[][]): string {
  return ["name: Release", "jobs:", ...jobs.flat(), ""].join("\n");
}

const MACOS_JOB = job("build-macos", "macos-latest", [BUILD_STEP, DMG_STEP, CHECK_STEP]);
const WINDOWS_JOB = job("build-windows", "windows-latest", [WINDOWS_BUILD_STEP]);
const PUBLISH_JOB = job("publish", "ubuntu-latest", [DOWNLOAD_STEP, PUBLISH_STEP]);

describe("split build and publish jobs", () => {
  // 16
  it("reads the signing identity from the macOS job when it comes before the Windows job", () => {
    const text = splitWorkflow([MACOS_JOB, WINDOWS_JOB, PUBLISH_JOB]);
    expect(readTauriBuildSigningIdentity(text)).toBe("-");
  });

  // 17
  it("finds the signature check in the macOS job before the publish job's gh release", () => {
    const text = splitWorkflow([MACOS_JOB, WINDOWS_JOB, PUBLISH_JOB]);
    expect(findSignatureCheckStep(text)).toEqual({
      ok: true,
      value: { name: "verify the signature", checksAdhoc: true },
    });
  });

  // 18
  it("reads no identity when the Windows job's tauri build comes before the macOS job", () => {
    const text = splitWorkflow([WINDOWS_JOB, MACOS_JOB, PUBLISH_JOB]);
    expect(readTauriBuildSigningIdentity(text)).toBeUndefined();
    // The ordering check itself still holds: codesign is after the first
    // tauri build and before gh release.
    expect(findSignatureCheckStep(text)).toMatchObject({ ok: true });
  });

  // 19
  it("fails when the publish job comes before the macOS job", () => {
    const text = splitWorkflow([PUBLISH_JOB, MACOS_JOB, WINDOWS_JOB]);
    const result = findSignatureCheckStep(text);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toContain("after the Release is published");
    }
  });
});

describe("findUnsignedClaims", () => {
  // 5
  it("returns the lines of sentences that read as no signature at all", () => {
    const text = [
      "# Install",
      "",
      "The app is not signed or notarized by Apple.",
      "Check whether Unsigned apps are allowed.",
      "",
      "# The release is unsigned.",
      "A signed app.",
    ].join("\n");
    expect(findUnsignedClaims(text)).toEqual([3, 4, 6]);
  });

  // 6
  it("leaves out sentences about ad-hoc signing, notarization or a signing certificate", () => {
    const text = [
      "The app is ad-hoc signed, not notarized by Apple.",
      "It is not signed",
      "with a Developer ID certificate. Older releases were unsigned.",
      "",
      "An ad-hoc signed app is not an unsigned app.",
    ].join("\n");
    expect(findUnsignedClaims(text)).toEqual([3]);
    expect(findUnsignedClaims("Ad-hoc signed (not notarized) macOS build.")).toEqual([]);
  });
});

describe("terminalFallbackOnly", () => {
  const fallback = [
    'If macOS says the app "is damaged and can\'t be opened", remove the',
    "quarantine attribute in Terminal:",
    "",
    "```bash",
    'xattr -dr com.apple.quarantine "/Applications/Local Sights.app"',
    "```",
  ];
  const guiSteps = ["## First launch", "", "1. Click **Open Anyway**.", ""];

  // 7
  it("accepts xattr that appears only after the is-damaged paragraph", () => {
    expect(terminalFallbackOnly([...guiSteps, ...fallback].join("\n"))).toEqual({
      ok: true,
      value: { fallbackLine: 5, xattrLines: [9] },
    });
  });

  // 8
  it.each([
    [
      "xattr before the fallback",
      ["1. Run `xattr -dr com.apple.quarantine` first.", ...guiSteps, ...fallback],
      "before",
    ],
    [
      "no is-damaged paragraph",
      [...guiSteps, "Run `xattr` if needed."],
      'no paragraph mentions "is damaged"',
    ],
    ["no xattr at all", [...guiSteps, ...fallback.slice(0, 2)], "xattr does not appear"],
  ])("fails with a reason: %s", (_label, lines, reason) => {
    const result = terminalFallbackOnly(lines.join("\n"));
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toContain(reason);
    }
  });
});

/** GitHub's heading anchor: lowercase, punctuation dropped, spaces to hyphens. */
function headingSlug(heading: string): string {
  return heading
    .toLowerCase()
    .replace(/[^\w\s-]/g, "")
    .replace(/\s+/g, "-");
}

/** The text of one `## ` section of a Markdown file, up to the next `## `. */
function section(markdown: string, heading: string): string {
  const start = markdown.indexOf(`\n${heading}\n`);
  expect(start, `${heading} is present`).toBeGreaterThanOrEqual(0);
  const rest = markdown.slice(start + 1 + heading.length);
  const end = rest.indexOf("\n## ");
  return end < 0 ? rest : rest.slice(0, end);
}

// One test per requirement, against the files as they are in the repository.
describe("repository", () => {
  // 9 (FR1.1)
  it("release.yml runs tauri build with APPLE_SIGNING_IDENTITY set to - (ad-hoc)", () => {
    expect(readTauriBuildSigningIdentity(releaseYml)).toBe("-");
  });

  // 10 (FR1.3)
  it("release-dry-run.yml is left as it was: no signing identity", () => {
    expect(readTauriBuildSigningIdentity(releaseDryRunYml)).toBeUndefined();
    expect(releaseDryRunYml).not.toContain("APPLE_SIGNING_IDENTITY");
  });

  // 11 (FR2.1, FR2.2)
  it("release.yml verifies the ad-hoc signature of the .app in the .dmg before publishing", () => {
    const step = findSignatureCheckStep(releaseYml);
    expect(step).toEqual({
      ok: true,
      value: { name: "the .app in the .dmg carries a valid ad-hoc signature", checksAdhoc: true },
    });
    expect(releaseYml).toContain('codesign --verify --deep --strict --verbose=2 "$app"');
    // The .app is read from the mounted .dmg that is published, not from
    // target/release/bundle/macos/, which Tauri deletes after building the .dmg.
    expect(releaseYml).toContain("DMG: ${{ steps.dmg.outputs.path }}");
    expect(releaseYml).toContain('app="$mnt/Local Sights.app"');
    expect(releaseYml).not.toContain('app="target/release/bundle/macos/Local Sights.app"');
    expect(releaseYml).toContain("::error::");
  });

  // 12 (FR1.4, FR3.1, FR3.4)
  it.each([
    ["docs/install.md", installMd],
    ["docs/releasing.md", releasingMd],
    ["README.md", readmeMd],
    [".github/workflows/release.yml", releaseYml],
  ])("%s no longer reads as if the app had no signature at all", (_path, text) => {
    expect(findUnsignedClaims(text)).toEqual([]);
  });

  // 13 (FR3.2, FR3.3)
  it("docs/install.md opens the app from the GUI and keeps xattr only as the fallback", () => {
    const result = terminalFallbackOnly(installMd);
    expect(result).toMatchObject({ ok: true });
    if (result.ok) {
      const lines = installMd.split("\n");
      const openAnyway = lines.findIndex((line) => line.includes("**Open Anyway**")) + 1;
      expect(openAnyway).toBeGreaterThan(0);
      expect(openAnyway).toBeLessThan(result.value.fallbackLine);
    }
  });

  // 14 (FR3.2: the renamed heading is still linked correctly)
  it("docs/install.md links only to headings that exist", () => {
    const slugs = [...installMd.matchAll(/^#+ (.+)$/gm)].map((m) => headingSlug(m[1] ?? ""));
    const anchors = [...installMd.matchAll(/\]\(#([^)]+)\)/g)].map((m) => m[1] ?? "");
    expect(anchors).toContain("first-launch-the-could-not-verify-warning");
    for (const anchor of anchors) {
      expect(slugs).toContain(anchor);
    }
  });

  // 15 (FR3.5)
  it("docs/releasing.md's post-release check covers the first-launch dialog", () => {
    const steps = section(releasingMd, "## Steps");
    expect(steps).toContain("could not verify");
    expect(steps).toContain("**Open Anyway**");
    expect(steps).toContain("is damaged");
    expect(section(releasingMd, "## When something fails")).toContain("Signature=adhoc");
  });

  // 20 (261015-windows-support FR2.3, FR2.6): the macOS build job is read
  // first, the Windows build job after it, and publishing waits for both.
  it("release.yml builds the .dmg before the Windows installer and publishes only after both", () => {
    const macos = releaseYml.indexOf("\n  build-macos:\n");
    const windows = releaseYml.indexOf("\n  build-windows:\n");
    const publish = releaseYml.indexOf("\n  publish:\n");
    expect(macos).toBeGreaterThan(0);
    expect(windows).toBeGreaterThan(macos);
    expect(publish).toBeGreaterThan(windows);
    expect(releaseYml.slice(publish)).toContain(
      "needs: [verify, test, build-macos, build-windows]",
    );
    expect(releaseYml).toContain("--bundles nsis");
    expect(releaseYml.match(/gh release/g)?.length).toBeGreaterThan(0);
    expect(releaseYml.indexOf("gh release")).toBeGreaterThan(publish);
  });
});
