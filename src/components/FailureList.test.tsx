import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ApiFailure, FailedStream } from "../api";
import { FailureList } from "./FailureList";
import { t } from "../test/fixtures";

const failed: FailedStream[] = [
  {
    logStreamName: "2024/01/02/[$LATEST]aaa",
    failure: {
      kind: "Throttled",
      safeDetail: "kind=Throttled; api=GetLogEvents; logStream=2024/01/02/[$LATEST]aaa",
      retryable: true,
    },
  },
  {
    logStreamName: "2024/01/02/[$LATEST]bbb",
    failure: {
      kind: "AccessDenied",
      safeDetail: "kind=AccessDenied; api=GetLogEvents",
      retryable: false,
    },
  },
];

const listingFailure: ApiFailure = {
  kind: "Network",
  safeDetail: "kind=Network; api=DescribeLogStreams",
  retryable: true,
};

function renderList(listing: ApiFailure | null = null) {
  const onClose = vi.fn();
  render(<FailureList failedStreams={failed} listingFailure={listing} t={t} onClose={onClose} />);
  return onClose;
}

/** The table row that holds the cell with `testId`. */
const rowOf = (element: HTMLElement) => element.closest("tr")!;

/** Cloudscape's modal reads the key code; user-event sends none, so Escape is fired by hand. */
const pressEscape = () =>
  fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape", keyCode: 27 });

describe("FailureList (FR3.3, mockups screen 5d)", () => {
  it("is a modal dialog with a table: stream, then what happened, what to do and the safe detail", () => {
    renderList();
    expect(screen.getByRole("dialog", { name: "Failures" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "Stream" })).toBeInTheDocument();
    expect(
      screen.getByRole("columnheader", { name: "What happened / what to do" }),
    ).toBeInTheDocument();
    const items = screen.getAllByTestId("failure-list-item");
    expect(items).toHaveLength(2);
    expect(items[0]).toHaveTextContent("2024/01/02/[$LATEST]aaa");
    expect(rowOf(items[0]!)).toHaveTextContent("AWS throttled the requests.");
    expect(rowOf(items[0]!)).toHaveTextContent("Wait a while, then press [Fetch] to fetch again.");
    expect(rowOf(items[1]!)).toHaveTextContent("Access was denied.");
    expect(rowOf(items[1]!)).toHaveTextContent("Details: kind=AccessDenied; api=GetLogEvents");
    expect(rowOf(items[1]!)).not.toHaveTextContent("Access denied");
    expect(screen.queryByTestId("failure-list-listing")).not.toBeInTheDocument();
  });

  it("shows the stream listing failure first", () => {
    renderList(listingFailure);
    const listing = screen.getByTestId("failure-list-listing");
    expect(listing).toHaveTextContent("Stream listing");
    expect(rowOf(listing)).toHaveTextContent("Could not connect to AWS.");
    expect(rowOf(listing)).toHaveTextContent(
      "Check your connection, then press [Fetch] to fetch again.",
    );
    expect(rowOf(listing)).toHaveTextContent("Details: kind=Network; api=DescribeLogStreams");
    const rows = screen.getAllByRole("row").slice(1);
    expect(rows).toHaveLength(3);
    expect(rows[0]).toContainElement(listing);
  });

  it("starts with the focus on Close and closes with it", async () => {
    const onClose = renderList();
    const close = screen.getByTestId("failure-list-close-button");
    expect(close).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes with Escape and with the dialog's close button", async () => {
    const onClose = renderList();
    pressEscape();
    expect(onClose).toHaveBeenCalledTimes(1);
    // The dialog's own close button (top right) comes before the footer button.
    const [dismiss] = screen.getAllByRole("button", { name: "Close (Esc)" });
    await userEvent.click(dismiss!);
    expect(onClose).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("dialog", { name: "Failures" })).toBeInTheDocument();
  });

  it("keeps Tab inside the dialog", async () => {
    const user = userEvent.setup();
    render(
      <>
        <button type="button" data-testid="outside-button">
          outside
        </button>
        <FailureList failedStreams={failed} listingFailure={null} t={t} onClose={vi.fn()} />
      </>,
    );
    const dialog = screen.getByRole("dialog", { name: "Failures" });
    for (let step = 0; step < 6; step += 1) {
      await user.tab();
      expect(dialog).toContainElement(document.activeElement as HTMLElement);
    }
    expect(screen.getByTestId("outside-button")).not.toHaveFocus();
  });
});
