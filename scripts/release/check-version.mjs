#!/usr/bin/env node
// Checks that Cargo.toml, Cargo.lock, package.json and src-tauri/tauri.conf.json
// carry the same version, and with `--tag vX.Y.Z` that the tag names it.
//
//   npm run check:version
//   npm run check:version -- --tag v0.1.0
//
// Exit code 0: consistent (the version is printed). 1: a mismatch, a malformed
// tag or an unreadable file (the reason goes to standard error). 2: invalid
// arguments.
//
// Requires Node.js 22.18 or later, which imports versions.ts directly
// (built-in TypeScript type stripping).

import { readFile } from "node:fs/promises";
import { checkVersions, FILE_NAMES } from "./versions.ts";

const repoRoot = new URL("../../", import.meta.url);

function parseArgs(argv) {
  if (argv.length === 0) return { tag: undefined };
  if (argv.length === 2 && argv[0] === "--tag" && argv[1]) return { tag: argv[1] };
  return undefined;
}

async function readSource(key) {
  const name = FILE_NAMES[key];
  try {
    return { ok: true, value: await readFile(new URL(name, repoRoot), "utf8") };
  } catch (error) {
    // Only the file name and the error code, never a stack or file contents.
    const code = error && typeof error === "object" && "code" in error ? ` (${error.code})` : "";
    return { ok: false, error: `${name}: could not be read${code}` };
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args) {
    console.error("Usage: node scripts/release/check-version.mjs [--tag vX.Y.Z]");
    return 2;
  }

  const keys = Object.keys(FILE_NAMES);
  const read = await Promise.all(keys.map(readSource));
  const failed = read.filter((result) => !result.ok);
  if (failed.length > 0) {
    for (const result of failed) console.error(result.error);
    return 1;
  }
  const sources = Object.fromEntries(keys.map((key, index) => [key, read[index].value]));

  const result = checkVersions(sources, args.tag);
  if (!result.ok) {
    console.error(result.error);
    return 1;
  }
  const files = Object.values(FILE_NAMES).join(", ");
  const tagNote = args.tag ? ` and matches tag ${args.tag}` : "";
  console.log(`Version ${result.value} is the same in ${files}${tagNote}.`);
  return 0;
}

process.exitCode = await main();
