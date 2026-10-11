import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import tauriConf from "../src-tauri/tauri.conf.json";
import { App } from "./App";
import { sessionView } from "./test/fixtures";

vi.mock("./api", async (importOriginal) => {
  const original = await importOriginal<typeof import("./api")>();
  const unlisten = () => undefined;
  return {
    ...original,
    getSession: vi.fn(async () => sessionView()),
    getRows: vi.fn(async () => ({ rows: [], totalCount: 0 })),
    rowPositions: vi.fn(async () => ({
      positions: [],
      timelineVersion: 0,
      resultVersion: 0,
      discardGeneration: 0,
      totalCount: 0,
    })),
    onSessionChanged: vi.fn(async () => unlisten),
    onFetchProgress: vi.fn(async () => unlisten),
    onFilterProgress: vi.fn(async () => unlisten),
  };
});

describe("display name", () => {
  it.each(["en", "ja"] as const)("shows Local Sights as the heading (%s)", async (locale) => {
    render(<App locale={locale} />);
    await screen.findByTestId("fetch-form");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(/^Local Sights$/);
  });

  it("names the bundle and the window Local Sights and keeps the identifier", () => {
    expect(tauriConf.productName).toBe("Local Sights");
    expect(tauriConf.app.windows[0]?.title).toBe("Local Sights");
    expect(tauriConf.identifier).toBe("dev.local-sights.app");
  });
});
