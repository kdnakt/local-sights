import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { FetchForm } from "./FetchForm";
import { createTranslator } from "../i18n/messages";
import { sessionView, t } from "../test/fixtures";

function renderForm(overrides: Parameters<typeof sessionView>[0] = {}) {
  const onChange = vi.fn();
  const onFetch = vi.fn();
  render(
    <FetchForm session={sessionView(overrides)} t={t} onChange={onChange} onFetch={onFetch} />,
  );
  return { onChange, onFetch };
}

describe("FetchForm", () => {
  it("shows why Fetch is unavailable and disables it", () => {
    renderForm({
      canFetch: false,
      validationErrors: ["validation.logGroupRequired", "validation.rangeOrder"],
    });
    expect(screen.getByTestId("fetch-form-submit-button")).toBeDisabled();
    const reasons = screen.getByTestId("fetch-form-reasons");
    expect(reasons).toHaveTextContent("Enter a log group name.");
    expect(reasons).toHaveTextContent("The start must be before the end.");
  });

  it("enables Fetch when the core says the input is valid", async () => {
    const { onFetch } = renderForm({ canFetch: true });
    expect(screen.queryByTestId("fetch-form-reasons")).not.toBeInTheDocument();
    await userEvent.click(screen.getByTestId("fetch-form-submit-button"));
    expect(onFetch).toHaveBeenCalledTimes(1);
  });

  it("disables every control while fetching", () => {
    renderForm({ phase: "Fetching", canFetch: false, validationErrors: [] });
    for (const input of screen.getAllByRole("textbox")) {
      expect(input).toBeDisabled();
    }
    expect(screen.getByTestId("fetch-form-submit-button")).toBeDisabled();
  });

  it("submits with Enter only when Fetch is available", async () => {
    const user = userEvent.setup();
    const { onFetch } = renderForm({ canFetch: true });
    await user.type(screen.getByTestId("fetch-form-end-input"), "{Enter}");
    expect(onFetch).toHaveBeenCalledTimes(1);
  });

  it("does not submit with Enter when Fetch is unavailable", async () => {
    const user = userEvent.setup();
    const { onFetch } = renderForm({
      canFetch: false,
      validationErrors: ["validation.endFormat"],
    });
    await user.type(screen.getByTestId("fetch-form-start-input"), "{Enter}");
    expect(onFetch).not.toHaveBeenCalled();
  });

  it("forwards each change with its field name", async () => {
    const user = userEvent.setup();
    const { onChange } = renderForm();
    await user.type(screen.getByTestId("fetch-form-start-input"), "ab");
    expect(onChange).toHaveBeenLastCalledWith("startText", "ab");
    expect(screen.getByTestId("fetch-form-start-input")).toHaveValue("ab");
  });

  it("has no profile, log group or stream inputs and always shows the selected log group", () => {
    renderForm({ selectedLogGroupName: "/aws/lambda/MyFunction" });
    expect(screen.queryByTestId("fetch-form-profile-input")).not.toBeInTheDocument();
    expect(screen.queryByTestId("fetch-form-log-group-input")).not.toBeInTheDocument();
    expect(screen.queryByTestId("fetch-form-log-stream-input")).not.toBeInTheDocument();
    expect(screen.getAllByRole("textbox")).toHaveLength(2);
    expect(screen.getByTestId("fetch-form-selected-log-group")).toHaveTextContent(
      "/aws/lambda/MyFunction",
    );
  });

  it("says when no log group is selected", () => {
    renderForm({ selectedLogGroupName: null });
    expect(screen.getByTestId("fetch-form-selected-log-group")).toHaveTextContent("Not selected");
  });

  it("moves through the two inputs and Fetch with Tab", async () => {
    const user = userEvent.setup();
    renderForm({ canFetch: true });
    const order = ["fetch-form-start-input", "fetch-form-end-input", "fetch-form-submit-button"];
    for (const testId of order) {
      await user.tab();
      expect(screen.getByTestId(testId)).toHaveFocus();
    }
  });
  it("disables every control while a connection change awaits confirmation", () => {
    renderForm({
      canFetch: false,
      pendingChange: { proposedProfile: null, proposedRegion: "eu-west-1" },
    });
    for (const input of screen.getAllByRole("textbox")) {
      expect(input).toBeDisabled();
    }
    expect(screen.getByTestId("fetch-form-submit-button")).toBeDisabled();
  });

  it("names the chosen time zone in the labels and the format reasons", () => {
    renderForm({
      timeZone: "Utc",
      canFetch: false,
      validationErrors: ["validation.startFormat"],
    });
    expect(screen.getByText("Start (UTC)")).toBeInTheDocument();
    expect(screen.getByText("End (UTC)")).toBeInTheDocument();
    expect(screen.getByTestId("fetch-form-reasons")).toHaveTextContent(
      "Enter the start as a valid yyyy-mm-dd hh:mm:ss (UTC).",
    );
  });

  it("shows a time skipped by daylight saving as its own reason", () => {
    renderForm({
      timeZone: "Local",
      canFetch: false,
      validationErrors: ["validation.startNonexistentLocalTime", "validation.endFormat"],
    });
    expect(screen.getByText("Start (Local)")).toBeInTheDocument();
    const reasons = screen.getByTestId("fetch-form-reasons");
    expect(reasons).toHaveTextContent(
      "The start does not exist because of the daylight saving time change.",
    );
    expect(reasons).toHaveTextContent("Enter the end as a valid yyyy-mm-dd hh:mm:ss (Local).");
  });

  it("shows the texts the core holds in the chosen zone, in Japanese too", () => {
    render(
      <FetchForm
        session={sessionView({
          timeZone: "Local",
          startInput: { text: "2024-03-01 10:00:00", instant: 1_709_254_800_000, error: null },
          endInput: { text: "2024-03-10 02:30:00", instant: null, error: "NonexistentLocalTime" },
          canFetch: false,
          validationErrors: ["validation.endNonexistentLocalTime"],
        })}
        t={createTranslator("ja")}
        onChange={vi.fn()}
        onFetch={vi.fn()}
      />,
    );
    expect(screen.getByTestId("fetch-form-start-input")).toHaveValue("2024-03-01 10:00:00");
    expect(screen.getByTestId("fetch-form-end-input")).toHaveValue("2024-03-10 02:30:00");
    expect(screen.getByText("開始日時（ローカル）")).toBeInTheDocument();
    expect(screen.getByTestId("fetch-form-reasons")).toHaveTextContent(
      "終了日時は夏時間の切り替えで存在しない日時です。",
    );
  });
});
