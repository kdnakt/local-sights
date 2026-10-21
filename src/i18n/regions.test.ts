// 261020-carryover-fixes FR5: the region names of the region picker and its
// filter. The names are the table in regions.ts alone; this test keeps no
// copy of them, and checks the table against the Rust core's built-in list.
import { describe, expect, it } from "vitest";
import regionsRs from "../../crates/local-sights-core/src/catalog/regions.rs?raw";
import { REGION_NAMES, filterRegionOptions, regionLabel } from "./regions";

/** The codes of `STANDARD_REGIONS` in regions.rs, in their order; throws when the list cannot be read. */
function standardRegions(source: string): string[] {
  const opening = "pub const STANDARD_REGIONS: &[&str] = &[";
  const start = source.indexOf(opening);
  if (start < 0) {
    throw new Error("STANDARD_REGIONS is not in regions.rs");
  }
  const end = source.indexOf("];", start);
  if (end < 0) {
    throw new Error("STANDARD_REGIONS has no end in regions.rs");
  }
  const block = source.slice(start + opening.length, end);
  const codes = [...block.matchAll(/"([^"]+)"/g)].map((match) => match[1] ?? "");
  if (codes.length === 0) {
    throw new Error("STANDARD_REGIONS lists no region in regions.rs");
  }
  return codes;
}

const CODES = ["ap-northeast-1", "ap-northeast-3", "eu-west-1", "us-east-1", "xx-test-1"];

describe("region names (261020-carryover-fixes FR5.1, FR5.3, FR5.5)", () => {
  it("writes the code and the Japanese name on the Japanese screen", () => {
    expect(regionLabel("ap-northeast-1", "ja")).toBe("ap-northeast-1 アジアパシフィック (東京)");
    expect(regionLabel("us-east-1", "ja")).toBe("us-east-1 米国東部 (バージニア北部)");
  });

  it("writes the code and the English name on the English screen", () => {
    expect(regionLabel("ap-northeast-1", "en")).toBe("ap-northeast-1 Asia Pacific (Tokyo)");
    expect(regionLabel("us-east-1", "en")).toBe("us-east-1 US East (N. Virginia)");
  });

  it("writes only the code of a region the table does not name", () => {
    expect(regionLabel("xx-test-1", "ja")).toBe("xx-test-1");
    expect(regionLabel("xx-test-1", "en")).toBe("xx-test-1");
  });
});

describe("region filter (FR5.4, FR5.5, A7)", () => {
  it("finds a region by either language's name, by any case, on either screen", () => {
    for (const text of ["東京", "tokyo", "TOKYO", "Tokyo", "northeast-1"]) {
      expect(filterRegionOptions(CODES, text), text).toContain("ap-northeast-1");
    }
    expect(filterRegionOptions(CODES, "東京")).toEqual(["ap-northeast-1"]);
    expect(filterRegionOptions(CODES, "northeast")).toEqual(["ap-northeast-1", "ap-northeast-3"]);
  });

  it("does not tell full-width and half-width parentheses apart", () => {
    expect(filterRegionOptions(CODES, "（東京）")).toEqual(["ap-northeast-1"]);
    expect(filterRegionOptions(CODES, "(東京)")).toEqual(["ap-northeast-1"]);
    expect(filterRegionOptions(CODES, "(N. VIRGINIA)")).toEqual(["us-east-1"]);
  });

  it("leaves nothing for a text no region holds", () => {
    expect(filterRegionOptions(CODES, "zzz")).toEqual([]);
  });

  it("keeps every region in its order for an empty text", () => {
    expect(filterRegionOptions(CODES, "")).toEqual(CODES);
    expect(filterRegionOptions(CODES, "   ")).toEqual(CODES);
  });

  it("finds a region the table does not name by its code", () => {
    expect(filterRegionOptions(CODES, "XX-TEST")).toEqual(["xx-test-1"]);
    expect(filterRegionOptions(CODES, "test")).toEqual(["xx-test-1"]);
  });
});

describe("the name table and the core's region list (FR5.10, review R-08)", () => {
  it("reads the core's list, and fails when it cannot", () => {
    expect(() => standardRegions("no list here")).toThrow();
    expect(() => standardRegions("pub const STANDARD_REGIONS: &[&str] = &[\n];")).toThrow();
    expect(standardRegions(regionsRs).length).toBeGreaterThanOrEqual(30);
  });

  it("names exactly the regions of STANDARD_REGIONS, none missing and none more", () => {
    const listed = [...standardRegions(regionsRs)].sort();
    const named = Object.keys(REGION_NAMES).sort();
    expect(named).toEqual(listed);
    for (const [code, names] of Object.entries(REGION_NAMES)) {
      expect(names.en.length, code).toBeGreaterThan(0);
      expect(names.ja.length, code).toBeGreaterThan(0);
    }
  });
});
