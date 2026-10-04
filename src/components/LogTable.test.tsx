import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LogTable } from "./LogTable";
import { logEvent, t } from "../test/fixtures";
import { formatUtcMillis } from "../format";

describe("LogTable", () => {
  it("shows the message on one line without changing the event", () => {
    const event = logEvent(0, "first line\nsecond line\r\nthird");
    render(<LogTable events={[event]} t={t} />);
    const row = screen.getByTestId("log-table-row");
    expect(within(row).getByText("first line second line third")).toHaveClass("log-table-message");
    expect(event.message).toBe("first line\nsecond line\r\nthird");
  });

  it("shows the time in UTC with milliseconds", () => {
    render(<LogTable events={[logEvent(7)]} t={t} />);
    expect(screen.getByText("2024-01-02 03:04:05.007")).toBeInTheDocument();
  });

  it("formats epoch zero and out-of-range timestamps safely", () => {
    expect(formatUtcMillis(0)).toBe("1970-01-01 00:00:00.000");
    expect(formatUtcMillis(Number.MAX_SAFE_INTEGER)).toBe(String(Number.MAX_SAFE_INTEGER));
  });

  it("renders every event in the given order", () => {
    const events = Array.from({ length: 1500 }, (_, index) => logEvent(index));
    render(<LogTable events={events} t={t} />);
    const rows = screen.getAllByTestId("log-table-row");
    expect(rows).toHaveLength(1500);
    expect(rows[0]).toHaveTextContent("message 0");
    expect(rows[1499]).toHaveTextContent("message 1499");
  });

  it("labels the columns from the message catalog", () => {
    render(<LogTable events={[]} t={t} />);
    expect(screen.getByRole("columnheader", { name: "Time (UTC)" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "Message" })).toBeInTheDocument();
  });
});
