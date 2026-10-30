// Every command the front end invokes must be registered on the Rust side,
// or the real app denies the call while the mocked tests stay green
// (261011-sso-login review R-01). Three lists must agree with api.ts:
// the invoke handler (lib.rs), the app manifest (build.rs) and the window's
// capability (`allow-<kebab-name>` in capabilities/default.json).
import { describe, expect, it } from "vitest";
import apiSource from "./api.ts?raw";
import buildRs from "../src-tauri/build.rs?raw";
import libRs from "../src-tauri/src/lib.rs?raw";
import capabilityJson from "../src-tauri/capabilities/default.json?raw";

function invokedCommands(source: string): string[] {
  return [...source.matchAll(/invoke<[^>]*>\(\s*"([a-z_]+)"/g)].map((m) => m[1] ?? "");
}

function quotedNames(block: string): string[] {
  return [...block.matchAll(/"([a-z_]+)"/g)].map((m) => m[1] ?? "");
}

function manifestCommands(source: string): string[] {
  const block = /\.commands\(&\[([\s\S]*?)\]\)/.exec(source)?.[1] ?? "";
  return quotedNames(block);
}

function handlerCommands(source: string): string[] {
  const block = /generate_handler!\[([\s\S]*?)\]/.exec(source)?.[1] ?? "";
  return block
    .split(",")
    .map((name) => name.trim())
    .filter((name) => name.length > 0);
}

function allowedCommands(json: string): string[] {
  const capability = JSON.parse(json) as { permissions: string[] };
  return capability.permissions
    .filter((permission) => permission.startsWith("allow-"))
    .map((permission) => permission.slice("allow-".length).replace(/-/g, "_"));
}

const invoked = invokedCommands(apiSource);

describe("Tauri command registration", () => {
  it("finds the commands api.ts invokes, including the SSO login ones", () => {
    expect(invoked).toEqual(
      expect.arrayContaining([
        "get_session",
        "start_sso_login",
        "cancel_sso_login",
        "dismiss_sso_login",
        "open_sso_url",
        // 261025-insights-stats U2:BR4.2 (FR review R-08).
        "get_query_catalog",
      ]),
    );
  });

  it("registers every invoked command in the invoke handler (lib.rs)", () => {
    expect(handlerCommands(libRs)).toEqual(expect.arrayContaining(invoked));
  });

  it("lists every invoked command in the app manifest (build.rs)", () => {
    expect(manifestCommands(buildRs)).toEqual(expect.arrayContaining(invoked));
  });

  it("allows every invoked command in the window capability", () => {
    expect(allowedCommands(capabilityJson)).toEqual(expect.arrayContaining(invoked));
  });

  it("grants no permission for a command the app does not have", () => {
    const manifest = manifestCommands(buildRs);
    for (const allowed of allowedCommands(capabilityJson)) {
      expect(manifest).toContain(allowed);
    }
  });
});
