// Ad-hoc signing checks (pure logic, no Node APIs).
//
// The release .app is ad-hoc signed in CI: the build step of
// .github/workflows/release.yml passes APPLE_SIGNING_IDENTITY=- to Tauri, a
// later step verifies the signature with codesign, and the documentation
// describes the first-launch warning of an ad-hoc signed, not notarized app.
// Every function here takes file contents as a string, so signing.test.ts can
// check both synthetic texts and the real files; nothing is read from disk.
//
// The workflow reader is line-based and knows only what the checks need: the
// `steps:` list of each job, each step's top-level keys (`name`, `run`, `env`)
// and the entries of its `env` block. No YAML library is involved.
//
// This file uses erasable TypeScript syntax only, like versions.ts.

import type { Result } from "./versions";

function ok<T>(value: T): Result<T> {
  return { ok: true, value };
}

function fail<T>(error: string): Result<T> {
  return { ok: false, error };
}

/** One item of a job's `steps:` list, as the lines that belong to it. */
interface WorkflowStep {
  /** 1-based line number of the `- ` line. */
  line: number;
  /** Column of the item's keys (`name`, `run`, ...), after the `- `. */
  keyIndent: number;
  lines: string[];
}

function indentOf(line: string): number {
  return line.length - line.trimStart().length;
}

/** Splits every `steps:` list of the workflow into its items. */
function workflowSteps(workflowText: string): WorkflowStep[] {
  const steps: WorkflowStep[] = [];
  let stepsIndent: number | undefined;
  let itemIndent: number | undefined;
  let current: WorkflowStep | undefined;
  workflowText.split(/\r?\n/).forEach((line, index) => {
    const stepsKey = /^(\s*)steps:\s*$/.exec(line);
    if (stepsKey?.[1] !== undefined) {
      stepsIndent = stepsKey[1].length;
      itemIndent = undefined;
      current = undefined;
      return;
    }
    if (stepsIndent === undefined) return;
    if (line.trim() === "") {
      current?.lines.push(line);
      return;
    }
    const indent = indentOf(line);
    if (indent <= stepsIndent) {
      // Dedented out of the list: the job's next key, or the next job.
      stepsIndent = undefined;
      current = undefined;
      return;
    }
    if (/^\s*-\s/.test(line) && (itemIndent === undefined || indent === itemIndent)) {
      itemIndent = indent;
      current = { line: index + 1, keyIndent: indent + 2, lines: [line] };
      steps.push(current);
      return;
    }
    current?.lines.push(line);
  });
  return steps;
}

/** A top-level key of a step: its inline value and the indented lines below it. */
interface StepKey {
  value: string;
  block: string[];
}

function stepKey(step: WorkflowStep, key: string): StepKey | undefined {
  const keyLine = new RegExp(`^(\\s*-\\s+|\\s*)${key}:\\s*(.*?)\\s*$`);
  for (let i = 0; i < step.lines.length; i++) {
    const line = step.lines[i] ?? "";
    const match = keyLine.exec(line);
    if (!match || (match[1]?.length ?? 0) !== step.keyIndent) continue;
    const block: string[] = [];
    for (const below of step.lines.slice(i + 1)) {
      if (below.trim() !== "" && indentOf(below) <= step.keyIndent) break;
      block.push(below);
    }
    return { value: match[2] ?? "", block };
  }
  return undefined;
}

function unquote(value: string): string {
  const quoted = /^(["'])(.*)\1$/.exec(value);
  return quoted?.[2] ?? value;
}

function stepName(step: WorkflowStep): string | undefined {
  const name = stepKey(step, "name")?.value;
  return name === undefined ? undefined : unquote(name);
}

/** The `run:` script of a step (inline or block), or "" when it has none. */
function stepRun(step: WorkflowStep): string {
  const run = stepKey(step, "run");
  return run === undefined ? "" : [run.value, ...run.block].join("\n");
}

/** The entries of a step's `env:` block. */
function stepEnv(step: WorkflowStep): Map<string, string> {
  const env = new Map<string, string>();
  for (const line of stepKey(step, "env")?.block ?? []) {
    const entry = /^\s*([A-Za-z_][A-Za-z0-9_]*):\s*(.*?)\s*$/.exec(line);
    if (entry?.[1] !== undefined) {
      env.set(entry[1], unquote(entry[2] ?? ""));
    }
  }
  return env;
}

const TAURI_BUILD = "tauri build";
const CODESIGN_VERIFY = "codesign --verify";
const DMG_DIR = "target/release/bundle/dmg";
const PUBLISH = "gh release";
const ADHOC_SIGNATURE = "Signature=adhoc";

/**
 * Reads `APPLE_SIGNING_IDENTITY` from the `env:` of the step whose `run:`
 * contains `tauri build`. Returns undefined when no step runs `tauri build`
 * or that step's `env:` has no such entry (FR1.1, FR1.3).
 */
export function readTauriBuildSigningIdentity(workflowText: string): string | undefined {
  const build = workflowSteps(workflowText).find((step) => stepRun(step).includes(TAURI_BUILD));
  return build === undefined ? undefined : stepEnv(build).get("APPLE_SIGNING_IDENTITY");
}

/** What the signature check step of release.yml looks like (FR2.1, FR2.2). */
export interface SignatureCheckStep {
  name: string | undefined;
  /** Whether the step looks for `Signature=adhoc` in the codesign output. */
  checksAdhoc: boolean;
}

/**
 * Finds the step whose `run:` contains `codesign --verify` and checks that it
 * comes after the step that picks the built .dmg from
 * `target/release/bundle/dmg` (itself after `tauri build`) and before the
 * step that publishes with `gh release`. Tauri deletes the intermediate .app
 * once the .dmg is built, so the check has to open the .dmg; and a signature
 * failure must stop the job before anything is published.
 */
export function findSignatureCheckStep(workflowText: string): Result<SignatureCheckStep> {
  const steps = workflowSteps(workflowText);
  const index = (needle: string) => steps.findIndex((step) => stepRun(step).includes(needle));
  const check = index(CODESIGN_VERIFY);
  if (check < 0) return fail(`no step runs ${CODESIGN_VERIFY}`);
  const build = index(TAURI_BUILD);
  if (build < 0) return fail(`no step runs ${TAURI_BUILD}`);
  const dmg = index(DMG_DIR);
  if (dmg < 0) return fail(`no step checks ${DMG_DIR}`);
  const publish = index(PUBLISH);
  if (publish < 0) return fail(`no step publishes with ${PUBLISH}`);

  const step = steps[check];
  if (step === undefined) return fail(`no step runs ${CODESIGN_VERIFY}`);
  const name = stepName(step);
  const label = name === undefined ? `the step on line ${step.line}` : `step "${name}"`;
  if (check < build) return fail(`${label} runs ${CODESIGN_VERIFY} before the tauri build step`);
  if (check < dmg) return fail(`${label} runs ${CODESIGN_VERIFY} before the .dmg is picked`);
  if (check > publish)
    return fail(`${label} runs ${CODESIGN_VERIFY} after the Release is published`);
  return ok({ name, checksAdhoc: stepRun(step).includes(ADHOC_SIGNATURE) });
}

/** Consecutive non-blank lines, with the 1-based number of the first one. */
interface Paragraph {
  startLine: number;
  text: string;
}

function paragraphs(text: string): Paragraph[] {
  const result: Paragraph[] = [];
  let open = false;
  text.split(/\r?\n/).forEach((line, index) => {
    if (line.trim() === "") {
      open = false;
      return;
    }
    const last = result[result.length - 1];
    if (open && last !== undefined) {
      last.text += `\n${line}`;
    } else {
      result.push({ startLine: index + 1, text: line });
      open = true;
    }
  });
  return result;
}

/** Splits a paragraph into sentences; each carries its offset in the text. */
function sentences(text: string): { offset: number; text: string }[] {
  const result: { offset: number; text: string }[] = [];
  let start = 0;
  for (const end of text.matchAll(/[.!?](?=\s|$)/g)) {
    const stop = end.index + 1;
    result.push({ offset: start, text: text.slice(start, stop) });
    start = stop;
  }
  if (text.slice(start).trim() !== "") {
    result.push({ offset: start, text: text.slice(start) });
  }
  return result;
}

function lineAt(text: string, offset: number): number {
  return text.slice(0, offset).split("\n").length;
}

/** Wording that says there is no signature at all. */
const UNSIGNED_CLAIM = /\bunsigned\b|\bnot\s+signed\b/i;
/** Wording that only says which signature is missing, or names the ad-hoc one. */
const HONEST_WORDING = /\bnot\s+signed\s+with\b|\bnot\s+notarized\b|\bad-?hoc\b/i;

/**
 * Returns the 1-based line numbers of sentences that read as "the app has no
 * signature" (`unsigned`, `not signed`), leaving out sentences that say what
 * kind of signature is missing (`not signed with ...`, `not notarized`) or
 * mention the ad-hoc signature (`ad-hoc`, `Signature=adhoc`). Lines are
 * joined into paragraphs first, so a
 * sentence wrapped across lines is read whole (FR1.4, FR3.1, FR3.4).
 */
export function findUnsignedClaims(markdownText: string): number[] {
  const lines: number[] = [];
  for (const paragraph of paragraphs(markdownText)) {
    for (const sentence of sentences(paragraph.text)) {
      const claim = UNSIGNED_CLAIM.exec(sentence.text);
      if (claim === null || HONEST_WORDING.test(sentence.text)) continue;
      const offset = sentence.offset + claim.index;
      lines.push(paragraph.startLine + lineAt(paragraph.text, offset) - 1);
    }
  }
  return lines;
}

/** Where the Terminal fallback of install.md sits (FR3.2, FR3.3). */
export interface TerminalFallback {
  /** 1-based line of the paragraph that introduces the "is damaged" case. */
  fallbackLine: number;
  /** 1-based lines that mention `xattr`; all of them after `fallbackLine`. */
  xattrLines: number[];
}

const DAMAGED = "is damaged";
const XATTR = "xattr";

/**
 * Checks that `xattr` (the Terminal workaround) appears only after the
 * paragraph that mentions "is damaged", so the main first-launch steps need
 * no Terminal and the workaround is kept as a fallback.
 */
export function terminalFallbackOnly(markdownText: string): Result<TerminalFallback> {
  const lines = markdownText.split(/\r?\n/);
  const fallbackLine = lines.findIndex((line) => line.includes(DAMAGED)) + 1;
  if (fallbackLine === 0) {
    return fail(`no paragraph mentions "${DAMAGED}" (the Terminal fallback)`);
  }
  const xattrLines = lines.flatMap((line, index) => (line.includes(XATTR) ? [index + 1] : []));
  const early = xattrLines.find((line) => line < fallbackLine);
  if (early !== undefined) {
    return fail(
      `${XATTR} appears on line ${early}, before the "${DAMAGED}" paragraph on line ${fallbackLine}`,
    );
  }
  if (xattrLines.length === 0) {
    return fail(`${XATTR} does not appear after the "${DAMAGED}" paragraph (no Terminal fallback)`);
  }
  return ok({ fallbackLine, xattrLines });
}
