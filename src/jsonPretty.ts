/**
 * JsonPrettyPrinter (261022-small-features U2:BR5.1, BR5.2, ADR-003): finds
 * the first readable JSON object or array in a log message and pretty-prints
 * it while keeping every token as written. Pure; never uses `JSON.parse`, so
 * numbers such as `1.0`, `1e5`, `-0` or 20-digit integers, the key order and
 * every string escape survive untouched (FR5.4).
 *
 * How a candidate is chosen (BR5.1, functional-design review R-03): the
 * positions of `{` and `[` are tried from the start of the message, each as
 * the start of one JSON value read by a token scanner (RFC 8259). The first
 * one that reads to its end wins and what follows it is never scanned
 * (Q4 A). An array holding no object and no array (scalars only, such as
 * `[1234]` or `[]`) is not a candidate, so a leading `[200]` does not hide the
 * `{...}` after it. A `{` inside a string is read as string content within a
 * scan that started at an earlier candidate; when it is later tried as a
 * candidate of its own it cannot be read to its end, so the search moves on.
 * The scanner uses an explicit stack, never recursion, and gives up on the
 * whole message beyond `MAX_DEPTH` nested containers, `MAX_CANDIDATES` tried
 * positions or a message longer than `MAX_MESSAGE_LENGTH` (NFR2).
 */

/** Deepest nesting of objects and arrays read; deeper is "not found". */
export const MAX_DEPTH = 256;
/** Most `{` / `[` positions tried in one message; more is "not found". */
export const MAX_CANDIDATES = 64;
/** Longest message scanned, in UTF-16 units (1 MiB); longer is "not found". */
export const MAX_MESSAGE_LENGTH = 1_048_576;

/** The message split around the JSON found (entities.md JsonSegments). */
export interface JsonSegments {
  /** The text before the JSON, as it is (its trailing whitespace too). */
  prefix: string;
  /** The JSON exactly as written in the message. */
  json: string;
  /** The JSON pretty-printed (BR5.2). */
  pretty: string;
  /** The text after the JSON, as it is. */
  suffix: string;
  /** Where `json` starts in the message. */
  start: number;
  /** Where `json` ends in the message (exclusive). */
  end: number;
}

const WHITESPACE = new Set([" ", "\t", "\n", "\r"]);
const ESCAPABLE = new Set(['"', "\\", "/", "b", "f", "n", "r", "t"]);
const HEX = /^[0-9a-fA-F]{4}$/;
const DIGIT = /[0-9]/;
const SCALAR_END = new Set([",", "]", "}", " ", "\t", "\n", "\r"]);

function skipWhitespace(text: string, index: number): number {
  let at = index;
  while (at < text.length && WHITESPACE.has(text[at]!)) {
    at += 1;
  }
  return at;
}

/** The index after the string starting with the `"` at `index`, or -1. */
function scanString(text: string, index: number): number {
  let at = index + 1;
  while (at < text.length) {
    const ch = text[at]!;
    if (ch === '"') {
      return at + 1;
    }
    if (ch === "\\") {
      const next = text[at + 1];
      if (next === undefined) {
        return -1;
      }
      if (next === "u") {
        if (!HEX.test(text.slice(at + 2, at + 6))) {
          return -1;
        }
        at += 6;
        continue;
      }
      if (!ESCAPABLE.has(next)) {
        return -1;
      }
      at += 2;
      continue;
    }
    if (ch < " ") {
      return -1;
    }
    at += 1;
  }
  return -1;
}

function scanDigits(text: string, index: number): number {
  let at = index;
  while (at < text.length && DIGIT.test(text[at]!)) {
    at += 1;
  }
  return at;
}

/** The index after the RFC 8259 number starting at `index`, or -1. */
function scanNumber(text: string, index: number): number {
  let at = index;
  if (text[at] === "-") {
    at += 1;
  }
  if (text[at] === "0") {
    at += 1;
  } else if (text[at] !== undefined && /[1-9]/.test(text[at]!)) {
    at = scanDigits(text, at);
  } else {
    return -1;
  }
  if (text[at] === ".") {
    const fraction = scanDigits(text, at + 1);
    if (fraction === at + 1) {
      return -1;
    }
    at = fraction;
  }
  if (text[at] === "e" || text[at] === "E") {
    let exponent = at + 1;
    if (text[exponent] === "+" || text[exponent] === "-") {
      exponent += 1;
    }
    const digits = scanDigits(text, exponent);
    if (digits === exponent) {
      return -1;
    }
    at = digits;
  }
  return at;
}

/** The index after the scalar (string, number or literal) at `index`, or -1. */
function scanScalar(text: string, index: number): number {
  const ch = text[index];
  if (ch === '"') {
    return scanString(text, index);
  }
  for (const literal of ["true", "false", "null"]) {
    if (text.startsWith(literal, index)) {
      return index + literal.length;
    }
  }
  if (ch === "-" || (ch !== undefined && DIGIT.test(ch))) {
    return scanNumber(text, index);
  }
  return -1;
}

type Container = "object" | "array";
type Need = "value" | "key" | "commaOrEnd";

interface Scanned {
  end: number;
  /** The value is an object, or an array holding an object or an array. */
  candidate: boolean;
}

/** The scan nested deeper than `MAX_DEPTH`: the whole search stops. */
const TOO_DEEP = Symbol("tooDeep");

/**
 * Reads one JSON value starting at `start` with an explicit stack of the open
 * containers; `null` when it cannot be read to its end, `TOO_DEEP` when it
 * nests deeper than `MAX_DEPTH` (a later candidate inside the same nesting
 * would only read a cut of it, so the search gives up as a whole).
 */
function scanValue(text: string, start: number): Scanned | null | typeof TOO_DEEP {
  const stack: Container[] = [];
  let topArrayHasContainer = false;
  let topKind: Container | null = null;
  let need: Need = "value";
  let at = start;
  for (;;) {
    at = skipWhitespace(text, at);
    const ch = text[at];
    if (ch === undefined) {
      return null;
    }
    if (need === "value") {
      if (ch === "{" || ch === "[") {
        if (stack.length >= MAX_DEPTH) {
          return TOO_DEEP;
        }
        const kind: Container = ch === "{" ? "object" : "array";
        if (stack.length === 0) {
          topKind = kind;
        } else if (stack.length === 1 && stack[0] === "array") {
          topArrayHasContainer = true;
        }
        stack.push(kind);
        at += 1;
        const afterOpen = skipWhitespace(text, at);
        const closer = kind === "object" ? "}" : "]";
        if (text[afterOpen] === closer) {
          at = afterOpen + 1;
          stack.pop();
          if (stack.length === 0) {
            break;
          }
          need = "commaOrEnd";
        } else {
          need = kind === "object" ? "key" : "value";
        }
        continue;
      }
      if (stack.length === 0) {
        // A scalar alone is never a candidate (BR5.1).
        return null;
      }
      const end = scanScalar(text, at);
      if (end < 0) {
        return null;
      }
      at = end;
      need = "commaOrEnd";
      continue;
    }
    if (need === "key") {
      if (ch !== '"') {
        return null;
      }
      const end = scanString(text, at);
      if (end < 0) {
        return null;
      }
      at = skipWhitespace(text, end);
      if (text[at] !== ":") {
        return null;
      }
      at += 1;
      need = "value";
      continue;
    }
    const top = stack[stack.length - 1]!;
    if (ch === ",") {
      at += 1;
      need = top === "object" ? "key" : "value";
      continue;
    }
    if (ch === (top === "object" ? "}" : "]")) {
      at += 1;
      stack.pop();
      if (stack.length === 0) {
        break;
      }
      need = "commaOrEnd";
      continue;
    }
    return null;
  }
  return { end: at, candidate: topKind === "object" || topArrayHasContainer };
}

/**
 * BR5.1: the first readable JSON object or array of `message`, with the text
 * before and after it kept as they are; `null` when there is none (the
 * message is then shown as it is, FR5.3).
 */
export function findJson(message: string): JsonSegments | null {
  if (message.length === 0 || message.length > MAX_MESSAGE_LENGTH) {
    return null;
  }
  let tried = 0;
  let from = 0;
  while (tried < MAX_CANDIDATES) {
    const brace = message.indexOf("{", from);
    const bracket = message.indexOf("[", from);
    const start = brace < 0 ? bracket : bracket < 0 ? brace : Math.min(brace, bracket);
    if (start < 0) {
      return null;
    }
    tried += 1;
    const scanned = scanValue(message, start);
    if (scanned === TOO_DEEP) {
      return null;
    }
    if (scanned !== null && scanned.candidate) {
      const json = message.slice(start, scanned.end);
      return {
        prefix: message.slice(0, start),
        json,
        pretty: prettyPrint(json),
        suffix: message.slice(scanned.end),
        start,
        end: scanned.end,
      };
    }
    from = start + 1;
  }
  return null;
}

/** The index after the scalar token at `index` of a JSON text already read. */
function scalarEnd(text: string, index: number): number {
  if (text[index] === '"') {
    const end = scanString(text, index);
    return end < 0 ? text.length : end;
  }
  let at = index;
  while (at < text.length && !SCALAR_END.has(text[at]!)) {
    at += 1;
  }
  return at;
}

/**
 * BR5.2: `json` (a value `findJson` read) with two-space indentation, one
 * member or element per line, `"key": value`, empty `{}` and `[]` kept as
 * they are and the original whitespace dropped. Every token is copied as
 * written (FR5.4).
 */
export function prettyPrint(json: string): string {
  const parts: string[] = [];
  let depth = 0;
  let at = skipWhitespace(json, 0);
  while (at < json.length) {
    const ch = json[at]!;
    if (ch === "{" || ch === "[") {
      const closer = ch === "{" ? "}" : "]";
      const afterOpen = skipWhitespace(json, at + 1);
      if (json[afterOpen] === closer) {
        parts.push(ch + closer);
        at = afterOpen + 1;
      } else {
        depth += 1;
        parts.push(`${ch}\n${"  ".repeat(depth)}`);
        at += 1;
      }
    } else if (ch === "}" || ch === "]") {
      depth = Math.max(0, depth - 1);
      parts.push(`\n${"  ".repeat(depth)}${ch}`);
      at += 1;
    } else if (ch === ",") {
      parts.push(`,\n${"  ".repeat(depth)}`);
      at += 1;
    } else if (ch === ":") {
      parts.push(": ");
      at += 1;
    } else {
      const end = scalarEnd(json, at);
      parts.push(json.slice(at, end));
      at = end;
    }
    at = skipWhitespace(json, at);
  }
  return parts.join("");
}
