import { describe, expect, it } from "vitest";
import { configuredPerfRowCount, formatDisplayTime, makePerfRow, perfRows } from "./perfRows";

/** An environment as Vite exposes it, with the given flags and variables. */
function env(values: Partial<ImportMetaEnv>): ImportMetaEnv {
  return { BASE_URL: "/", MODE: "test", DEV: true, PROD: false, SSR: false, ...values };
}

describe("perfRows measurement harness (plan Step 10, review R-03)", () => {
  it("is off in a production build even when the row count variable is set (14f)", () => {
    expect(
      configuredPerfRowCount(
        env({ DEV: false, PROD: true, VITE_LOCAL_SIGHTS_PERF_ROWS: "1000000" }),
      ),
    ).toBe(0);
    expect(configuredPerfRowCount(env({ DEV: false, PROD: true }))).toBe(0);
  });

  it("is off in a development build without the variable, and on with it", () => {
    expect(configuredPerfRowCount(env({}))).toBe(0);
    expect(configuredPerfRowCount(env({ VITE_LOCAL_SIGHTS_PERF_ROWS: "" }))).toBe(0);
    expect(configuredPerfRowCount(env({ VITE_LOCAL_SIGHTS_PERF_ROWS: "abc" }))).toBe(0);
    expect(configuredPerfRowCount(env({ VITE_LOCAL_SIGHTS_PERF_ROWS: "-5" }))).toBe(0);
    expect(configuredPerfRowCount(env({ VITE_LOCAL_SIGHTS_PERF_ROWS: "1000000" }))).toBe(1_000_000);
  });

  it("does nothing in the test run, where no variable is set", () => {
    expect(perfRows.enabled).toBe(false);
    expect(perfRows.total()).toBe(0);
    expect(perfRows.getRows(0, 10).rows).toEqual([]);
  });

  it("generates deterministic rows with the core's time format", () => {
    const row = makePerfRow(3);
    expect(row).toEqual(makePerfRow(3));
    expect(row.message.startsWith("{")).toBe(true);
    expect(row.displayTime).toMatch(/^\d{4}\/\d{2}\/\d{2} \d{2}:\d{2}:\d{2}\.\d{3}$/);
    expect(formatDisplayTime(Date.UTC(2024, 2, 1, 1, 2, 3, 4))).toBe("2024/03/01 01:02:03.004");
    expect(makePerfRow(0).message.length).toBeGreaterThanOrEqual(80);
  });
});
