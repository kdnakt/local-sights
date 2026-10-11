// Release version checks (pure logic, no Node APIs).
//
// The release version lives in four files that must agree: Cargo.toml
// ([workspace.package] version), Cargo.lock (the two workspace crates),
// package.json and src-tauri/tauri.conf.json. A release tag vX.Y.Z must match
// that version. Every function takes file contents as strings and returns a
// Result; reading the files is left to check-version.mjs.
//
// Error messages are fixed sentences that contain only file names, package
// names and version values, never other file contents.
//
// This file uses erasable TypeScript syntax only, so Node (22.18 or later) can
// import it directly with its built-in type stripping.

export type Result<T> = { ok: true; value: T } | { ok: false; error: string };

/** File contents the version check reads. */
export interface VersionSources {
  cargoToml: string;
  cargoLock: string;
  packageJson: string;
  tauriConf: string;
}

/** The workspace crates whose versions Cargo.lock records. */
export const WORKSPACE_CRATES = ["local-sights", "local-sights-core"] as const;
export type WorkspaceCrate = (typeof WORKSPACE_CRATES)[number];

export const FILE_NAMES = {
  cargoToml: "Cargo.toml",
  cargoLock: "Cargo.lock",
  packageJson: "package.json",
  tauriConf: "src-tauri/tauri.conf.json",
} as const;

const TAG_FORM = "vMAJOR.MINOR.PATCH (for example v0.1.0)";
const NUMERIC_PART = /^\d+$/;
const SECTION_HEADER = /^\s*\[\[?\s*([^\]]+?)\s*\]\]?\s*(#.*)?$/;
const VERSION_LINE = /^\s*version\s*=\s*"([^"]*)"\s*(#.*)?$/;
const NAME_LINE = /^\s*name\s*=\s*"([^"]*)"\s*(#.*)?$/;

function ok<T>(value: T): Result<T> {
  return { ok: true, value };
}

function fail<T>(error: string): Result<T> {
  return { ok: false, error };
}

/** Parses a release tag `vMAJOR.MINOR.PATCH` and returns `MAJOR.MINOR.PATCH`. */
export function parseTag(tag: string): Result<string> {
  const refused = (reason: string) => fail<string>(`Tag ${tag} ${reason}; use ${TAG_FORM}.`);
  if (!tag.startsWith("v")) {
    return refused("must start with v");
  }
  const version = tag.slice(1);
  if (/[-+]/.test(version)) {
    return refused("has a pre-release or build suffix, which is not supported");
  }
  const parts = version.split(".");
  if (parts.length !== 3 || !parts.every((part) => NUMERIC_PART.test(part))) {
    return refused("is not of the form vMAJOR.MINOR.PATCH");
  }
  if (parts.some((part) => part.length > 1 && part.startsWith("0"))) {
    return refused("has a leading zero in a number");
  }
  return ok(version);
}

/** Reads `version` from the `[workspace.package]` section of Cargo.toml. */
export function readCargoWorkspaceVersion(cargoToml: string): Result<string> {
  let section: string | undefined;
  let sectionFound = false;
  for (const line of cargoToml.split(/\r?\n/)) {
    const header = SECTION_HEADER.exec(line);
    if (header) {
      section = header[1];
      sectionFound ||= section === "workspace.package";
      continue;
    }
    const version = section === "workspace.package" ? VERSION_LINE.exec(line) : null;
    if (version?.[1] !== undefined) {
      return ok(version[1]);
    }
  }
  return fail(
    sectionFound
      ? `${FILE_NAMES.cargoToml}: no version in [workspace.package]`
      : `${FILE_NAMES.cargoToml}: no [workspace.package] section`,
  );
}

interface LockEntry {
  name?: string;
  version?: string;
}

function lockEntries(cargoLock: string): LockEntry[] {
  const entries: LockEntry[] = [];
  let current: LockEntry | undefined;
  for (const line of cargoLock.split(/\r?\n/)) {
    if (SECTION_HEADER.test(line)) {
      current = line.trim() === "[[package]]" ? {} : undefined;
      if (current) entries.push(current);
      continue;
    }
    if (!current) continue;
    current.name ??= NAME_LINE.exec(line)?.[1];
    current.version ??= VERSION_LINE.exec(line)?.[1];
  }
  return entries;
}

/** Reads the versions of the workspace crates recorded in Cargo.lock. */
export function readCargoLockVersions(cargoLock: string): Result<Record<WorkspaceCrate, string>> {
  const entries = lockEntries(cargoLock);
  const versions: Partial<Record<WorkspaceCrate, string>> = {};
  for (const crate of WORKSPACE_CRATES) {
    const matches = entries.filter((entry) => entry.name === crate);
    if (matches.length === 0) {
      return fail(`${FILE_NAMES.cargoLock}: no package ${crate}`);
    }
    const version = matches[0]?.version;
    if (matches.length > 1 || version === undefined) {
      return fail(`${FILE_NAMES.cargoLock}: package ${crate} must appear once with a version`);
    }
    versions[crate] = version;
  }
  return ok(versions as Record<WorkspaceCrate, string>);
}

/** Reads the top-level string `version` of a JSON file (package.json, tauri.conf.json). */
export function readJsonVersion(text: string, fileName: string): Result<string> {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    // The parser's message can quote file contents, so it is not passed on.
    return fail(`${fileName}: not valid JSON`);
  }
  const version =
    typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>).version
      : undefined;
  if (typeof version !== "string") {
    return fail(`${fileName}: no string "version" field`);
  }
  return ok(version);
}

interface FoundVersion {
  label: string;
  version: string;
}

function collectVersions(sources: VersionSources): Result<FoundVersion[]> {
  const cargo = readCargoWorkspaceVersion(sources.cargoToml);
  if (!cargo.ok) return cargo;
  const lock = readCargoLockVersions(sources.cargoLock);
  if (!lock.ok) return lock;
  const pkg = readJsonVersion(sources.packageJson, FILE_NAMES.packageJson);
  if (!pkg.ok) return pkg;
  const tauri = readJsonVersion(sources.tauriConf, FILE_NAMES.tauriConf);
  if (!tauri.ok) return tauri;
  return ok([
    { label: FILE_NAMES.cargoToml, version: cargo.value },
    ...WORKSPACE_CRATES.map((crate) => ({
      label: `${FILE_NAMES.cargoLock} (${crate})`,
      version: lock.value[crate],
    })),
    { label: FILE_NAMES.packageJson, version: pkg.value },
    { label: FILE_NAMES.tauriConf, version: tauri.value },
  ]);
}

/**
 * Checks that the four files carry one version and, when a tag is given, that
 * the tag is a valid `vX.Y.Z` naming that version. Returns the version.
 */
export function checkVersions(sources: VersionSources, tag?: string): Result<string> {
  const tagVersion = tag === undefined ? undefined : parseTag(tag);
  if (tagVersion && !tagVersion.ok) return tagVersion;

  const found = collectVersions(sources);
  if (!found.ok) return found;

  const [first, ...rest] = found.value;
  if (!first) return fail("No versions were read.");
  if (rest.some((entry) => entry.version !== first.version)) {
    const lines = found.value.map((entry) => `  ${entry.label}: ${entry.version}`);
    return fail(["Versions do not match:", ...lines].join("\n"));
  }
  if (tagVersion && tagVersion.value !== first.version) {
    return fail(`Tag ${tag} does not match version ${first.version} in the files.`);
  }
  return ok(first.version);
}
