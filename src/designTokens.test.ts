import { describe, expect, it } from "vitest";
import { APP_DESIGN_TOKENS, applyDesignTokens } from "./designTokens";

describe("designTokens (FR4.5, NFR2 fallback styling)", () => {
  it("sets every --ls-* custom property on the target element", () => {
    const body = document.createElement("body");
    applyDesignTokens(body);
    for (const [name, value] of Object.entries(APP_DESIGN_TOKENS)) {
      expect(name.startsWith("--ls-"), name).toBe(true);
      expect(body.style.getPropertyValue(name), name).toBe(value);
    }
  });

  it("uses Cloudscape variables only, never a color of its own", () => {
    for (const [name, value] of Object.entries(APP_DESIGN_TOKENS)) {
      expect(value, name).toMatch(/^var\(--/);
    }
  });
});
