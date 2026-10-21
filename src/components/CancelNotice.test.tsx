import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { createTranslator } from "../i18n/messages";
import { t } from "../test/fixtures";
import { CancelNotice } from "./CancelNotice";

describe("CancelNotice (261012-parallel-fetch U5:BR5.11, NFR9.4)", () => {
  it("says how many streams were not read to their end, as a warning in a status region", () => {
    render(<CancelNotice notice={{ unfinishedStreams: 13 }} t={t} />);
    const notice = screen.getByTestId("cancel-notice");
    expect(notice).toHaveAttribute("role", "status");
    expect(screen.getByTestId("cancel-notice-text")).toHaveTextContent(
      "Fetch cancelled. 13 streams were not finished, so some logs are missing.",
    );
    // Not by colour alone: the warning icon carries a name.
    expect(within(notice).getByRole("img", { name: "Cancelled" })).toBeInTheDocument();
  });

  it("says the listing had not ended when there is no count", () => {
    render(<CancelNotice notice={{ unfinishedStreams: null }} t={t} />);
    expect(screen.getByTestId("cancel-notice-text")).toHaveTextContent(
      "Fetch cancelled before the stream listing finished, so some logs are missing.",
    );
  });

  it("has no close button", () => {
    render(<CancelNotice notice={{ unfinishedStreams: 2 }} t={t} />);
    expect(within(screen.getByTestId("cancel-notice")).queryByRole("button")).toBeNull();
  });

  it("keeps an empty status region on the screen while there is no notice (261020-carryover-fixes FR4.2)", () => {
    render(<CancelNotice notice={null} t={t} />);
    const region = screen.getByTestId("cancel-notice");
    expect(region).toHaveAttribute("role", "status");
    expect(region).toBeEmptyDOMElement();
    expect(screen.queryByTestId("cancel-notice-text")).toBeNull();
    expect(region).toHaveClass("cancel-notice");
  });

  it("puts the notice into the region that was already there, and empties it again (FR4.2.1)", () => {
    const { rerender } = render(<CancelNotice notice={null} t={t} />);
    const before = screen.getByTestId("cancel-notice");
    rerender(<CancelNotice notice={{ unfinishedStreams: 1 }} t={t} />);
    const after = screen.getByTestId("cancel-notice");
    expect(after).toBe(before);
    expect(within(after).getByTestId("cancel-notice-text")).toHaveTextContent(
      "Fetch cancelled. 1 stream was not finished, so some logs are missing.",
    );
    rerender(<CancelNotice notice={null} t={t} />);
    expect(screen.getByTestId("cancel-notice")).toBe(before);
    expect(before).toBeEmptyDOMElement();
  });

  it("is worded as the refined mockups in Japanese", () => {
    const ja = createTranslator("ja");
    const { rerender } = render(<CancelNotice notice={{ unfinishedStreams: 0 }} t={ja} />);
    expect(screen.getByTestId("cancel-notice-text")).toHaveTextContent(
      "取得を中断しました。0 本のストリームは読み終えていないため、一部のログが含まれていません。",
    );
    rerender(<CancelNotice notice={{ unfinishedStreams: null }} t={ja} />);
    expect(screen.getByTestId("cancel-notice-text")).toHaveTextContent(
      "ストリームの一覧を読み終える前に中断したため、一部のログが含まれていません。",
    );
  });
});
