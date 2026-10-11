import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionView } from "../api";
import { createTranslator } from "../i18n/messages";
import { sessionView, t } from "../test/fixtures";
import appSheet from "../styles.css?raw";
import pickerSheet from "../../node_modules/@cloudscape-design/components/date-picker/styles.scoped.css?raw";
import { ConditionArea } from "./ConditionArea";

type Overrides = Parameters<typeof sessionView>[0];

function renderArea(overrides: Overrides = {}, translate = t) {
  const onChange = vi.fn();
  const onFetch = vi.fn();
  const view = (next: Overrides) => (
    <ConditionArea
      session={sessionView({ ...overrides, ...next })}
      t={translate}
      onChange={onChange}
      onFetch={onFetch}
      filterText=""
      onFilterTextChange={vi.fn()}
      onLogFilterChange={vi.fn()}
    >
      <div data-testid="children" />
    </ConditionArea>
  );
  const result = render(view({}));
  return {
    onChange,
    onFetch,
    ...result,
    rerender: (next: Overrides) => result.rerender(view(next)),
  };
}

const input = (testId: string) => within(screen.getByTestId(testId)).getByRole("textbox");
const dateInput = (side: "start" | "end") => input(`fetch-form-${side}-date`);
const timeInput = (side: "start" | "end") => input(`fetch-form-${side}-time`);
const fetchButton = () => screen.getByTestId("fetch-form-submit-button");
const filterBox = () => screen.getByRole("searchbox", { name: "Filter messages" });

/** A session whose inputs the core already holds as `start` / `end`. */
function withTexts(start: string, end: string, more: Overrides = {}): Overrides {
  return {
    startInput: { text: start, instant: null, error: null },
    endInput: { text: end, instant: null, error: null },
    ...more,
  };
}

describe("ConditionArea (mockups screens 1, 2, 5a; FR1, FR2, FR3.1)", () => {
  it("heads the container with the selected log group, or says none is selected", () => {
    const { rerender } = renderArea({ selectedLogGroupName: null });
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent("Log group: none");
    rerender({ selectedLogGroupName: "/aws/lambda/MyFunction" });
    expect(screen.getByTestId("fetch-form-selected-log-group")).toHaveTextContent(
      "/aws/lambda/MyFunction",
    );
    expect(screen.getByTestId("children")).toBeInTheDocument();
  });

  it("says Choose a log group once, in the reason line and not in the heading (gate fix 15c)", () => {
    renderArea({
      selectedLogGroupName: null,
      canFetch: false,
      validationErrors: ["selection.logGroupRequired"],
    });
    expect(screen.getAllByText("Choose a log group.")).toHaveLength(1);
    expect(screen.getByTestId("fetch-form-reason-log-group")).toHaveTextContent(
      "Choose a log group.",
    );
    expect(screen.getByRole("heading", { level: 2 })).not.toHaveTextContent(/choose|select/i);
    // The heading and the reason never share a text in either language.
    const area = screen.getByTestId("fetch-form").textContent ?? "";
    expect(area.split("Choose a log group.")).toHaveLength(2);
  });

  it("labels the filter field like the start and the end, so the fields line up (gate fix 15d)", () => {
    renderArea();
    const filterGroup = screen.getByTestId("fetch-form-filter");
    const label = within(filterGroup).getByText("Filter messages", { selector: "div" });
    expect(label).toBeInTheDocument();
    expect(filterGroup).toContainElement(filterBox());
    // The label precedes the field, as the "Start (…)" label precedes its fields.
    expect(
      label.compareDocumentPosition(filterBox()) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    const startLabel = within(screen.getByTestId("fetch-form-start")).getByText("Start (Local)", {
      selector: "div",
    });
    // Both labels are the same Cloudscape key-label variant.
    expect(label.className).toBe(startLabel.className);
  });

  it("splits the core's texts into the date and time fields and sends them joined (FR1.1)", async () => {
    const user = userEvent.setup();
    const { onChange } = renderArea(withTexts("2024/03/01 10:00:00", "2024/03/01 12:00:00"));
    expect(dateInput("start")).toHaveValue("2024/03/01");
    expect(timeInput("start")).toHaveValue("10:00:00");
    expect(dateInput("end")).toHaveValue("2024/03/01");
    expect(timeInput("end")).toHaveValue("12:00:00");
    await user.clear(dateInput("start"));
    await user.type(dateInput("start"), "2024/03/15");
    expect(dateInput("start")).toHaveValue("2024/03/15");
    expect(onChange).toHaveBeenLastCalledWith("startText", "2024/03/15 10:00:00");
  });

  it("picks a date from the calendar into the date field as yyyy/mm/dd (FR1.1, K2)", async () => {
    const user = userEvent.setup();
    const { onChange } = renderArea(withTexts("2024/03/01 10:00:00", ""));
    await user.click(within(screen.getByTestId("fetch-form-start-date")).getByRole("button"));
    const grid = await screen.findByRole("grid");
    expect(screen.getByRole("dialog")).toHaveTextContent("March 2024");
    // Each day cell holds the visible number in a span (plus a hidden label).
    const day = Array.from(grid.querySelectorAll("td")).find(
      (cell) =>
        cell.querySelector("span")?.textContent === "15" &&
        cell.getAttribute("aria-disabled") !== "true",
    );
    expect(day).toBeDefined();
    await user.click(day!);
    expect(dateInput("start")).toHaveValue("2024/03/15");
    expect(onChange).toHaveBeenLastCalledWith("startText", "2024/03/15 10:00:00");
    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
  });

  it("opens a complete calendar outside the field, from a field as wide as the DatePicker allows (gate fix 14a)", async () => {
    const user = userEvent.setup();
    renderArea(withTexts("2024/03/01 10:00:00", ""));
    await user.click(within(screen.getByTestId("fetch-form-start-date")).getByRole("button"));
    const grid = await screen.findByRole("grid");
    // The whole week: seven weekday columns and both month buttons.
    expect(within(grid).getAllByRole("columnheader")).toHaveLength(7);
    // The two month buttons (previous, next) carry no name without the
    // I18nProvider, so they are counted.
    expect(within(screen.getByRole("dialog")).getAllByRole("button")).toHaveLength(2);
    // expandToViewport: the dropdown is not inside the fixed-width field box.
    expect(screen.getByTestId("fetch-form-start-date").contains(grid)).toBe(false);
    // Cloudscape sizes that dropdown to the trigger, so the field must be as
    // wide as the DatePicker's own maximum, where the calendar fits.
    const fieldWidth = /\.condition-date\s*\{[^}]*?\bwidth:\s*(\d+)px/.exec(appSheet)?.[1];
    const pickerMax = /date-picker-container[^{]*\{[^}]*?max-inline-size:\s*(\d+)px/.exec(
      pickerSheet,
    )?.[1];
    expect(pickerMax).toBeDefined();
    expect(fieldWidth).toBe(pickerMax);
  });

  it("lays the children in a column that the list alone shrinks, inside a frame as high as the container (gate fix 18a)", () => {
    renderArea();
    const frame = screen.getByTestId("condition-area-frame");
    const column = screen.getByTestId("condition-area");
    expect(screen.getByTestId("fetch-form")).toContainElement(frame);
    expect(column.parentElement).toBe(frame);
    expect(column).toHaveClass("condition-area");
    // The children come last in the column, after the condition row.
    expect(column.lastElementChild).toBe(screen.getByTestId("children"));
    // jsdom computes no heights, so the rules are read from the stylesheet.
    const rule = (selector: string) => {
      const escaped = selector.replace(/[.-]/g, (c) => `\\${c}`);
      const body = new RegExp(`(?:^|\\n)${escaped}\\s*\\{([^}]*)\\}`).exec(appSheet)?.[1];
      expect(body, selector).toBeDefined();
      return body ?? "";
    };
    expect(rule(".condition-area-frame")).toMatch(/position:\s*relative/);
    expect(rule(".condition-area-frame")).toMatch(/(?:^|\s)height:\s*100%/);
    const columnRule = rule(".condition-area");
    expect(columnRule).toMatch(/position:\s*absolute/);
    expect(columnRule).toMatch(/inset:\s*0/);
    expect(columnRule).toMatch(/display:\s*flex/);
    expect(columnRule).toMatch(/flex-direction:\s*column/);
    // The list takes what is left and may shrink below its rows' height.
    expect(rule(".log-table")).toMatch(/flex:\s*1/);
    expect(rule(".log-table")).toMatch(/min-height:\s*0/);
    // The status line never shrinks.
    expect(rule(".status-line")).toMatch(/flex:\s*0 0 auto/);
  });

  it("shows the format reason under the date field for an incomplete or impossible date (FR2.4)", async () => {
    const user = userEvent.setup();
    const { unmount } = renderArea(withTexts("2024/03/01 10:00:00", "2024/03/01 12:00:00"));
    await user.clear(dateInput("start"));
    await user.type(dateInput("start"), "2024/03");
    expect(
      within(screen.getByTestId("fetch-form-start-date-field")).getByText(
        "Enter the date as yyyy/mm/dd.",
      ),
    ).toBeInTheDocument();
    unmount();
    // A hyphen-separated or impossible text the core still holds is a format error too.
    renderArea(
      withTexts("2024-03-01 10:00:00", "2024/02/30 12:00:00", {
        canFetch: false,
        validationErrors: ["validation.startFormat", "validation.endFormat"],
      }),
    );
    expect(screen.getByTestId("fetch-form-start-date-field")).toHaveTextContent(
      "Enter the date as yyyy/mm/dd.",
    );
    expect(screen.getByTestId("fetch-form-end-date-field")).toHaveTextContent(
      "Enter the date as yyyy/mm/dd.",
    );
    expect(screen.queryByText(/Enter the start as a valid/)).not.toBeInTheDocument();
    expect(fetchButton()).toBeDisabled();
  });

  it("completes a short time when the field loses focus: :00 for the start, :59 for the end (FR1.3)", async () => {
    // user-event's Tab carries no keyCode, which the time mask treats as a
    // key to swallow; moving the focus by hand blurs the field as Tab does.
    const user = userEvent.setup();
    const { onChange } = renderArea(withTexts("2024/03/01 ", "2024/03/01 "));
    await user.type(timeInput("start"), "10:00");
    act(() => timeInput("end").focus());
    expect(timeInput("start")).toHaveValue("10:00:00");
    expect(onChange).toHaveBeenLastCalledWith("startText", "2024/03/01 10:00:00");
    await user.type(timeInput("end"), "10:00");
    act(() => fetchButton().focus());
    expect(timeInput("end")).toHaveValue("10:00:59");
    expect(onChange).toHaveBeenLastCalledWith("endText", "2024/03/01 10:00:59");
    // An empty end time becomes the end of the day (Q4).
    await user.clear(timeInput("end"));
    act(() => fetchButton().focus());
    expect(timeInput("end")).toHaveValue("23:59:59");
    expect(onChange).toHaveBeenLastCalledWith("endText", "2024/03/01 23:59:59");
  });

  it("fills an empty time before fetching with Enter: 00:00:00 and 23:59:59 (Q4, R-01)", async () => {
    const user = userEvent.setup();
    const { onChange, onFetch, rerender } = renderArea(
      withTexts("2024/03/01 ", "2024/03/02 ", {
        canFetch: false,
        validationErrors: ["validation.startFormat", "validation.endFormat"],
      }),
    );
    // No reason is shown for the empty times, and Fetch may be pressed.
    expect(screen.queryByText(/Enter the time/)).not.toBeInTheDocument();
    expect(fetchButton()).toBeEnabled();
    dateInput("start").focus();
    await user.keyboard("{Enter}");
    expect(onChange).toHaveBeenCalledWith("startText", "2024/03/01 00:00:00");
    expect(onChange).toHaveBeenCalledWith("endText", "2024/03/02 23:59:59");
    expect(timeInput("start")).toHaveValue("00:00:00");
    expect(timeInput("end")).toHaveValue("23:59:59");
    expect(onFetch).not.toHaveBeenCalled();
    // The core takes the completed texts and allows the fetch: it starts.
    rerender(withTexts("2024/03/01 00:00:00", "2024/03/02 23:59:59", { canFetch: true }));
    expect(onFetch).toHaveBeenCalledTimes(1);
  });

  it("fetches at once with Enter or [Fetch] when the core already allows it, never when it refuses", async () => {
    const user = userEvent.setup();
    const { onFetch, rerender } = renderArea(
      withTexts("2024/03/01 10:00:00", "2024/03/01 12:00:00", { canFetch: true }),
    );
    timeInput("end").focus();
    await user.keyboard("{Enter}");
    expect(onFetch).toHaveBeenCalledTimes(1);
    await user.click(fetchButton());
    expect(onFetch).toHaveBeenCalledTimes(2);
    rerender(
      withTexts("2024/03/01 10:00:00", "2024/03/01 12:00:00", {
        canFetch: false,
        validationErrors: ["selection.logGroupRequired"],
      }),
    );
    expect(fetchButton()).toBeDisabled();
    dateInput("start").focus();
    await user.keyboard("{Enter}");
    expect(onFetch).toHaveBeenCalledTimes(2);
  });

  it("puts the order reason under the end date and the selection reason under the heading (FR3.1)", () => {
    renderArea(
      withTexts("2024/03/02 00:00:00", "2024/03/01 00:00:00", {
        canFetch: false,
        validationErrors: ["selection.logGroupRequired", "validation.rangeOrder"],
      }),
    );
    expect(screen.getByTestId("fetch-form-reason-log-group")).toHaveTextContent(
      "Choose a log group.",
    );
    expect(screen.getByTestId("fetch-form-end-date-field")).toHaveTextContent(
      "End must be after start.",
    );
    expect(screen.getByTestId("fetch-form-start-date-field")).not.toHaveTextContent("End must");
    expect(fetchButton()).toBeDisabled();
  });

  it("puts a time skipped by daylight saving under the time field, naming the zone in format reasons", () => {
    renderArea(
      withTexts("2024/03/10 02:30:00", "2024/03/10 04:00:00", {
        timeZone: "Local",
        canFetch: false,
        validationErrors: ["validation.startNonexistentLocalTime"],
      }),
    );
    expect(screen.getByTestId("fetch-form-start-time-field")).toHaveTextContent(
      "The start does not exist because of the daylight saving time change.",
    );
    expect(screen.getByRole("group", { name: "Start (Local)" })).toBeInTheDocument();
    expect(screen.getByRole("group", { name: "End (Local)" })).toBeInTheDocument();
  });

  it("asks for a date from the first moment and shows a bad time under the time field", () => {
    renderArea(
      withTexts("", " 25:00:00", {
        canFetch: false,
        validationErrors: ["validation.startFormat", "validation.endFormat"],
      }),
    );
    expect(screen.getByTestId("fetch-form-start-date-field")).toHaveTextContent("Enter a date.");
    expect(screen.getByTestId("fetch-form-end-date-field")).toHaveTextContent("Enter a date.");
    expect(screen.getByTestId("fetch-form-end-time-field")).toHaveTextContent(
      "Enter the time as hh:mm:ss.",
    );
  });

  it("disables the date, time and Fetch while fetching, keeps the filter usable, and moves the focus to it (FR1.8, R-06)", async () => {
    const { rerender } = renderArea(
      withTexts("2024/03/01 10:00:00", "2024/03/01 12:00:00", {
        canFetch: true,
        selectedLogGroupName: "/g",
      }),
    );
    fetchButton().focus();
    expect(fetchButton()).toHaveFocus();
    rerender(
      withTexts("2024/03/01 10:00:00", "2024/03/01 12:00:00", {
        phase: "Fetching",
        canFetch: false,
        selectedLogGroupName: "/g",
      }),
    );
    for (const side of ["start", "end"] as const) {
      expect(dateInput(side)).toBeDisabled();
      expect(timeInput(side)).toBeDisabled();
    }
    expect(fetchButton()).toBeDisabled();
    expect(filterBox()).toBeEnabled();
    expect(filterBox()).toHaveFocus();
  });

  it("disables everything, the filter included, while a dialog is open (U2:BR2.6, U6:BR5.1)", () => {
    const { rerender } = renderArea({
      canFetch: false,
      pendingChange: { proposedProfile: null, proposedRegion: "eu-west-1" },
    });
    expect(filterBox()).toBeDisabled();
    expect(dateInput("start")).toBeDisabled();
    expect(fetchButton()).toBeDisabled();
    rerender({ canFetch: true, settingsDialog: "Open" });
    expect(filterBox()).toBeDisabled();
    expect(fetchButton()).toBeDisabled();
  });

  it("regression (FR3.1): the list of reasons is gone; each reason sits under its field", () => {
    renderArea(
      withTexts("2024/03/01 10:00:00", "2024/03/01 12:00:00", {
        canFetch: false,
        validationErrors: ["validation.logGroupRequired", "validation.rangeOrder"],
      }),
    );
    expect(screen.queryByTestId("fetch-form-reasons")).not.toBeInTheDocument();
    expect(screen.getByTestId("fetch-form")).not.toHaveTextContent("Fetch is unavailable because");
    expect(screen.getByTestId("fetch-form").querySelectorAll("ul li")).toHaveLength(0);
    expect(screen.getByTestId("fetch-form-reason-log-group")).toHaveTextContent(
      "Enter a log group name.",
    );
    expect(screen.getByTestId("fetch-form-end-date-field")).toHaveTextContent(
      "End must be after start.",
    );
  });

  it("is in Japanese for the ja locale, with the zone in the labels", () => {
    renderArea(
      withTexts("2024/03/01 10:00:00", "2024/03/10 02:30:00", {
        canFetch: false,
        validationErrors: ["validation.endNonexistentLocalTime"],
      }),
      createTranslator("ja"),
    );
    expect(screen.getByRole("group", { name: "開始（ローカル）" })).toBeInTheDocument();
    expect(screen.getByTestId("fetch-form-submit-button")).toHaveTextContent("取得");
    expect(screen.getByTestId("fetch-form-end-time-field")).toHaveTextContent(
      "終了日時は夏時間の切り替えで存在しない日時です。",
    );
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent("ロググループ: 未選択");
    expect(screen.getByRole("searchbox", { name: "メッセージを絞り込み" })).toBeInTheDocument();
  });

  it("orders Tab as Fetch, filter, start date, its calendar, start time, end date, its calendar, end time (K7)", async () => {
    const user = userEvent.setup();
    renderArea(withTexts("2024/03/01 10:00:00", "2024/03/01 12:00:00", { canFetch: true }));
    await user.tab();
    expect(fetchButton()).toHaveFocus();
    await user.tab();
    expect(filterBox()).toHaveFocus();
    await user.tab();
    expect(dateInput("start")).toHaveFocus();
    // From here Tab follows the DOM order of the tabbable controls (the masked
    // inputs swallow user-event's keyCode-less Tab, so the order is checked
    // on the DOM).
    const tabbable = Array.from(
      screen
        .getByTestId("fetch-form")
        .querySelectorAll<HTMLElement>("input:not([disabled]), button:not([disabled])"),
    ).filter((element) => element.tabIndex >= 0);
    expect(tabbable).toEqual([
      fetchButton(),
      filterBox(),
      dateInput("start"),
      within(screen.getByTestId("fetch-form-start-date")).getByRole("button"),
      timeInput("start"),
      dateInput("end"),
      within(screen.getByTestId("fetch-form-end-date")).getByRole("button"),
      timeInput("end"),
    ]);
  });
});

describe("ConditionArea calendar limit (gate fix 17b)", () => {
  // The calendar and "Local" read the machine's zone; fix it, and fix only
  // `Date` so Cloudscape's timers and user-event's delays still run.
  beforeAll(() => {
    vi.stubEnv("TZ", "Asia/Tokyo");
  });
  afterAll(() => {
    vi.unstubAllEnvs();
  });
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["Date"] });
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  /** Opens the calendar of `side` and returns its day cells by visible number. */
  async function openCalendar(side: "start" | "end") {
    const user = userEvent.setup();
    await user.click(within(screen.getByTestId(`fetch-form-${side}-date`)).getByRole("button"));
    const grid = await screen.findByRole("grid");
    return (day: string) => {
      const cells = Array.from(grid.querySelectorAll("td")).filter(
        (cell) => cell.querySelector("span")?.textContent === day,
      );
      expect(cells).toHaveLength(1);
      return cells[0]!;
    };
  }

  it("disables tomorrow and later in the start calendar and keeps today enabled", async () => {
    // 2024-03-15 12:00 in Tokyo.
    vi.setSystemTime(new Date(Date.UTC(2024, 2, 15, 3, 0, 0)));
    const { onChange } = renderArea(withTexts("2024/03/14 00:00:00", "2024/03/15 23:59:59"));
    const cell = await openCalendar("start");
    expect(cell("15")).toHaveAttribute("aria-disabled", "false");
    expect(cell("15")).toHaveAttribute("aria-current", "date");
    expect(cell("14")).toHaveAttribute("aria-disabled", "false");
    expect(cell("16")).toHaveAttribute("aria-disabled", "true");
    expect(cell("31")).toHaveAttribute("aria-disabled", "true");
    // A disabled day picks nothing; today does.
    const user = userEvent.setup();
    await user.click(cell("16"));
    expect(dateInput("start")).toHaveValue("2024/03/14");
    expect(onChange).not.toHaveBeenCalled();
    await user.click(cell("15"));
    expect(dateInput("start")).toHaveValue("2024/03/15");
    expect(onChange).toHaveBeenLastCalledWith("startText", "2024/03/15 00:00:00");
  });

  it("limits the end calendar in the same way", async () => {
    vi.setSystemTime(new Date(Date.UTC(2024, 2, 15, 3, 0, 0)));
    renderArea(withTexts("2024/03/14 00:00:00", "2024/03/15 23:59:59"));
    const cell = await openCalendar("end");
    expect(cell("15")).toHaveAttribute("aria-disabled", "false");
    expect(cell("16")).toHaveAttribute("aria-disabled", "true");
    expect(cell("10")).toHaveAttribute("aria-disabled", "false");
  });

  it("takes today in UTC when the fields show UTC", async () => {
    // 2024-03-15 16:00 UTC is already 2024-03-16 01:00 in Tokyo.
    vi.setSystemTime(new Date(Date.UTC(2024, 2, 15, 16, 0, 0)));
    const { unmount } = renderArea(
      withTexts("2024/03/14 00:00:00", "2024/03/15 23:59:59", { timeZone: "Utc" }),
    );
    let cell = await openCalendar("start");
    expect(cell("15")).toHaveAttribute("aria-disabled", "false");
    expect(cell("16")).toHaveAttribute("aria-disabled", "true");
    unmount();
    // The same moment shown in local time allows the local today, the 16th.
    renderArea(withTexts("2024/03/14 00:00:00", "2024/03/15 23:59:59", { timeZone: "Local" }));
    cell = await openCalendar("start");
    expect(cell("16")).toHaveAttribute("aria-disabled", "false");
    expect(cell("17")).toHaveAttribute("aria-disabled", "true");
  });
});

/** Keeps the fixture type visible to the compiler for the helpers above. */
export type { SessionView };
