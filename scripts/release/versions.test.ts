import { describe, expect, it } from "vitest";
import {
  checkVersions,
  parseTag,
  readCargoLockVersions,
  readCargoWorkspaceVersion,
  readJsonVersion,
  type VersionSources,
} from "./versions";

// Minimal file contents. The real files are never read by these tests.
function cargoToml(version: string): string {
  return [
    "[workspace]",
    'members = ["crates/local-sights-core", "src-tauri"]',
    "",
    "[workspace.package]",
    `version = "${version}"`,
    'edition = "2024"',
    "",
    "[workspace.dependencies]",
    'tokio = { version = "1", features = ["rt"] }',
    "",
  ].join("\n");
}

function cargoLock(app: string, core: string): string {
  return [
    "version = 4",
    "",
    "[[package]]",
    'name = "local-sights"',
    `version = "${app}"`,
    "dependencies = [",
    ' "local-sights-core",',
    "]",
    "",
    "[[package]]",
    'name = "local-sights-core"',
    `version = "${core}"`,
    "",
    "[[package]]",
    'name = "local-sights-extra"',
    'version = "9.9.9"',
    "",
  ].join("\n");
}

function jsonWithVersion(version: string): string {
  return JSON.stringify({ name: "local-sights", version }, null, 2);
}

function sources(overrides: Partial<VersionSources> = {}): VersionSources {
  return {
    cargoToml: cargoToml("0.1.0"),
    cargoLock: cargoLock("0.1.0", "0.1.0"),
    packageJson: jsonWithVersion("0.1.0"),
    tauriConf: jsonWithVersion("0.1.0"),
    ...overrides,
  };
}

describe("parseTag", () => {
  // 1
  it("returns the version of a vMAJOR.MINOR.PATCH tag", () => {
    expect(parseTag("v0.1.0")).toEqual({ ok: true, value: "0.1.0" });
    expect(parseTag("v12.0.34")).toEqual({ ok: true, value: "12.0.34" });
  });

  // 2
  it.each([
    ["v1.2", "vMAJOR.MINOR.PATCH"],
    ["1.2.3", "must start with"],
    ["release-1", "must start with"],
    ["v01.2.3", "leading zero"],
    ["v1.2.3-rc.1", "pre-release"],
  ])("refuses %s with a reason", (tag, reason) => {
    const result = parseTag(tag);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toContain(reason);
      expect(result.error).toContain("v0.1.0");
    }
  });
});

describe("readCargoWorkspaceVersion", () => {
  // 3
  it("reads version from [workspace.package] only", () => {
    const text = [
      "[package]",
      'version = "7.7.7"',
      "",
      "[workspace.package]",
      'edition = "2024"',
      'version = "0.1.0" # the release version',
      "",
      "[workspace.dependencies]",
      'serde = { version = "1" }',
      'version = "8.8.8"',
    ].join("\n");
    expect(readCargoWorkspaceVersion(text)).toEqual({ ok: true, value: "0.1.0" });
  });

  // 4
  it("fails with a clear message when the section or the version is missing", () => {
    expect(readCargoWorkspaceVersion('[package]\nversion = "0.1.0"\n')).toEqual({
      ok: false,
      error: "Cargo.toml: no [workspace.package] section",
    });
    expect(readCargoWorkspaceVersion('[workspace.package]\nedition = "2024"\n')).toEqual({
      ok: false,
      error: "Cargo.toml: no version in [workspace.package]",
    });
  });
});

describe("readCargoLockVersions", () => {
  // 5
  it("reads local-sights and local-sights-core, not similarly named packages", () => {
    expect(readCargoLockVersions(cargoLock("0.1.0", "0.2.0"))).toEqual({
      ok: true,
      value: { "local-sights": "0.1.0", "local-sights-core": "0.2.0" },
    });
  });

  it("fails when a workspace package is missing from Cargo.lock", () => {
    const withoutCore = cargoLock("0.1.0", "0.1.0").replace(
      'name = "local-sights-core"',
      'name = "local-sights-core-old"',
    );
    expect(readCargoLockVersions(withoutCore)).toEqual({
      ok: false,
      error: "Cargo.lock: no package local-sights-core",
    });
  });
});

describe("readJsonVersion", () => {
  // 6
  it("reads the version field of package.json and tauri.conf.json", () => {
    expect(readJsonVersion(jsonWithVersion("0.1.0"), "package.json")).toEqual({
      ok: true,
      value: "0.1.0",
    });
    expect(readJsonVersion(jsonWithVersion("0.3.1"), "src-tauri/tauri.conf.json")).toEqual({
      ok: true,
      value: "0.3.1",
    });
  });

  it.each([
    ['{ "name": "local-sights" }', 'package.json: no string "version" field'],
    ['{ "version": 1 }', 'package.json: no string "version" field'],
    ['["0.1.0"]', 'package.json: no string "version" field'],
    ["{ not json", "package.json: not valid JSON"],
  ])("fails for %s", (text, error) => {
    expect(readJsonVersion(text, "package.json")).toEqual({ ok: false, error });
  });
});

describe("checkVersions", () => {
  // 7
  it("succeeds when all five versions are the same", () => {
    expect(checkVersions(sources())).toEqual({ ok: true, value: "0.1.0" });
  });

  // 8
  it("fails and names the file and value that differ", () => {
    const result = checkVersions(sources({ packageJson: jsonWithVersion("0.1.1") }));
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toContain("do not match");
      expect(result.error).toContain("package.json: 0.1.1");
      expect(result.error).toContain("Cargo.toml: 0.1.0");
    }

    const lock = checkVersions(sources({ cargoLock: cargoLock("0.1.0", "0.0.9") }));
    expect(lock.ok).toBe(false);
    if (!lock.ok) {
      expect(lock.error).toContain("Cargo.lock (local-sights-core): 0.0.9");
    }
  });

  // 9
  it("fails when the versions agree but the tag differs", () => {
    expect(checkVersions(sources(), "v0.2.0")).toEqual({
      ok: false,
      error: "Tag v0.2.0 does not match version 0.1.0 in the files.",
    });
  });

  // 10
  it("succeeds when the versions and the tag agree", () => {
    expect(checkVersions(sources(), "v0.1.0")).toEqual({ ok: true, value: "0.1.0" });
  });

  // 11
  it("refuses a malformed tag before comparing the files", () => {
    const result = checkVersions(sources({ packageJson: jsonWithVersion("0.1.1") }), "v1.2");
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toContain("v1.2");
      expect(result.error).not.toContain("package.json");
    }
  });

  // 12
  it("reports only file names and versions, never other file contents", () => {
    const secretLike = "AKIAABCDEFGHIJKLMNOP";
    const packageJson = JSON.stringify({ version: "0.1.1", note: secretLike });
    const cargo = `${cargoToml("0.1.0")}\n# token = "${secretLike}"\n`;
    const result = checkVersions(sources({ packageJson, cargoToml: cargo }));
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).not.toContain(secretLike);
      const [heading, ...lines] = result.error.split("\n");
      expect(heading).toBe("Versions do not match:");
      for (const line of lines) {
        expect(line).toMatch(/^ {2}[\w./-]+( \([\w-]+\))?: \S+$/);
      }
    }

    const broken = checkVersions(sources({ tauriConf: `{ "version": "${secretLike}` }));
    expect(broken).toEqual({ ok: false, error: "src-tauri/tauri.conf.json: not valid JSON" });
  });
});
