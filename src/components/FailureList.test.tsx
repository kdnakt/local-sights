import { render, screen, within } from "@testing-library/react";
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

describe("FailureList", () => {
  it("lists each failed stream with what happened, what to do and the safe detail", () => {
    renderList();
    const items = screen.getAllByTestId("failure-list-item");
    expect(items).toHaveLength(2);
    expect(within(items[0]!).getByText("2024/01/02/[$LATEST]aaa")).toBeInTheDocument();
    expect(items[0]).toHaveTextContent("AWS throttled the requests.");
    expect(items[0]).toHaveTextContent("Wait a while, then press [Fetch] to fetch again.");
    expect(items[1]).toHaveTextContent("Access was denied.");
    expect(items[1]).toHaveTextContent("Details: kind=AccessDenied; api=GetLogEvents");
    expect(items[1]).not.toHaveTextContent("Access denied");
    expect(screen.queryByTestId("failure-list-listing")).not.toBeInTheDocument();
  });

  it("shows the stream listing failure first", () => {
    renderList(listingFailure);
    const list = screen.getAllByRole("listitem");
    expect(list[0]).toHaveAttribute("data-testid", "failure-list-listing");
    expect(list[0]).toHaveTextContent("Stream listing");
    expect(list[0]).toHaveTextContent("Could not connect to AWS.");
    expect(list[0]).toHaveTextContent("Check your connection, then press [Fetch] to fetch again.");
    expect(list[0]).toHaveTextContent("Details: kind=Network; api=DescribeLogStreams");
    expect(list).toHaveLength(3);
  });

  it("starts with the focus on Close and closes with it", async () => {
    const onClose = renderList();
    const close = screen.getByTestId("failure-list-close-button");
    expect(close).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes with Escape", async () => {
    const onClose = renderList();
    await userEvent.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("dialog", { name: "Failures" })).toBeInTheDocument();
  });
});
