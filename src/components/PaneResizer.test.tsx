import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  PANE_WIDTH_DEFAULT_PERCENT,
  PANE_WIDTH_MAX_PERCENT,
  PANE_WIDTH_MIN_PERCENT,
  PANE_WIDTH_STORAGE_KEY,
  usePaneWidth,
} from "../hooks/usePaneWidth";
import { t } from "../test/fixtures";
import { PaneResizer } from "./PaneResizer";

/** The width jsdom reports for the body in these tests, so 10px is 1%. */
const BODY_WIDTH_PX = 1000;

/** The handle, driven by the hook like `App` does, inside a body of a known width. */
function Harness({ bodyWidth = BODY_WIDTH_PX }: { bodyWidth?: number }) {
  const pane = usePaneWidth();
  return (
    <div
      data-testid="body"
      ref={(element) => {
        if (element && bodyWidth > 0) {
          // jsdom lays nothing out; the handle reads its parent's clientWidth.
          Object.defineProperty(element, "clientWidth", { value: bodyWidth, configurable: true });
        }
      }}
      style={{ "--ls-log-group-pane-width": `${pane.width}%` } as never}
    >
      <PaneResizer value={pane.width} onChange={pane.setWidth} t={t} />
    </div>
  );
}

const handle = () => screen.getByRole("separator", { name: "Log group pane width" });
const widthOf = (element: HTMLElement) => Number(element.getAttribute("aria-valuenow"));
const bodyWidthVar = () =>
  screen.getByTestId("body").style.getPropertyValue("--ls-log-group-pane-width");

describe("PaneResizer (gate fixes 14d, 15b)", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    window.localStorage.clear();
  });

  it("starts at the default share with the slider values of a vertical separator, in percent", () => {
    render(<Harness />);
    expect(handle()).toHaveAttribute("aria-orientation", "vertical");
    expect(widthOf(handle())).toBe(PANE_WIDTH_DEFAULT_PERCENT);
    expect(handle()).toHaveAttribute("aria-valuemin", String(PANE_WIDTH_MIN_PERCENT));
    expect(handle()).toHaveAttribute("aria-valuemax", String(PANE_WIDTH_MAX_PERCENT));
    expect(handle()).toHaveAttribute("aria-valuetext", `${PANE_WIDTH_DEFAULT_PERCENT}%`);
    expect(bodyWidthVar()).toBe(`${PANE_WIDTH_DEFAULT_PERCENT}%`);
  });

  it("changes the share while dragging, by the moved pixels over the body's width", () => {
    render(<Harness />);
    fireEvent.pointerDown(handle(), { button: 0, pointerId: 1, clientX: 200 });
    fireEvent.pointerMove(handle(), { pointerId: 1, clientX: 300 });
    expect(widthOf(handle())).toBe(PANE_WIDTH_DEFAULT_PERCENT + 10);
    fireEvent.pointerMove(handle(), { pointerId: 1, clientX: 180 });
    expect(widthOf(handle())).toBe(PANE_WIDTH_DEFAULT_PERCENT - 2);
    fireEvent.pointerUp(handle(), { pointerId: 1, clientX: 180 });
    // After the pointer is released, moving it changes nothing.
    fireEvent.pointerMove(handle(), { pointerId: 1, clientX: 400 });
    expect(widthOf(handle())).toBe(PANE_WIDTH_DEFAULT_PERCENT - 2);
    expect(bodyWidthVar()).toBe(`${PANE_WIDTH_DEFAULT_PERCENT - 2}%`);
  });

  it("falls back to the window's width when the body has none", () => {
    render(<Harness bodyWidth={0} />);
    const quarter = window.innerWidth / 4; // 25% of the window
    fireEvent.pointerDown(handle(), { button: 0, pointerId: 1, clientX: 0 });
    fireEvent.pointerMove(handle(), { pointerId: 1, clientX: quarter });
    expect(widthOf(handle())).toBe(PANE_WIDTH_DEFAULT_PERCENT + 25);
  });

  it("moves by one percent with the Left and Right arrow keys", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    handle().focus();
    await user.keyboard("{ArrowRight}{ArrowRight}");
    expect(widthOf(handle())).toBe(PANE_WIDTH_DEFAULT_PERCENT + 2);
    expect(handle()).toHaveAttribute("aria-valuetext", `${PANE_WIDTH_DEFAULT_PERCENT + 2}%`);
    await user.keyboard("{ArrowLeft}");
    expect(widthOf(handle())).toBe(PANE_WIDTH_DEFAULT_PERCENT + 1);
  });

  it("stops at 20% and 50%, by keys and by dragging", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    handle().focus();
    await user.keyboard("{Home}{ArrowLeft}");
    expect(widthOf(handle())).toBe(PANE_WIDTH_MIN_PERCENT);
    await user.keyboard("{End}{ArrowRight}");
    expect(widthOf(handle())).toBe(PANE_WIDTH_MAX_PERCENT);
    fireEvent.pointerDown(handle(), { button: 0, pointerId: 2, clientX: 0 });
    fireEvent.pointerMove(handle(), { pointerId: 2, clientX: -10_000 });
    expect(widthOf(handle())).toBe(PANE_WIDTH_MIN_PERCENT);
    fireEvent.pointerMove(handle(), { pointerId: 2, clientX: 10_000 });
    expect(widthOf(handle())).toBe(PANE_WIDTH_MAX_PERCENT);
  });

  it("remembers the share in localStorage and restores it", () => {
    const { unmount } = render(<Harness />);
    fireEvent.pointerDown(handle(), { button: 0, pointerId: 3, clientX: 0 });
    fireEvent.pointerMove(handle(), { pointerId: 3, clientX: 40 });
    expect(window.localStorage.getItem(PANE_WIDTH_STORAGE_KEY)).toBe(
      String(PANE_WIDTH_DEFAULT_PERCENT + 4),
    );
    unmount();
    render(<Harness />);
    expect(widthOf(handle())).toBe(PANE_WIDTH_DEFAULT_PERCENT + 4);
  });
});
