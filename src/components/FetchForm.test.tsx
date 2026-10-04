import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { FetchForm } from "./FetchForm";
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
    await user.type(screen.getByTestId("fetch-form-log-stream-input"), "{Enter}");
    expect(onFetch).not.toHaveBeenCalled();
  });

  it("forwards each change with its field name", async () => {
    const user = userEvent.setup();
    const { onChange } = renderForm();
    await user.type(screen.getByTestId("fetch-form-log-stream-input"), "ab");
    expect(onChange).toHaveBeenLastCalledWith("logStreamName", "ab");
    expect(screen.getByTestId("fetch-form-log-stream-input")).toHaveValue("ab");
  });

  it("has no profile or log group inputs and always shows the selected log group", () => {
    renderForm({ selectedLogGroupName: "/aws/lambda/MyFunction" });
    expect(screen.queryByTestId("fetch-form-profile-input")).not.toBeInTheDocument();
    expect(screen.queryByTestId("fetch-form-log-group-input")).not.toBeInTheDocument();
    expect(screen.getByTestId("fetch-form-selected-log-group")).toHaveTextContent(
      "/aws/lambda/MyFunction",
    );
  });

  it("says when no log group is selected", () => {
    renderForm({ selectedLogGroupName: null });
    expect(screen.getByTestId("fetch-form-selected-log-group")).toHaveTextContent("Not selected");
  });

  it("moves through the three inputs and Fetch with Tab", async () => {
    const user = userEvent.setup();
    renderForm({ canFetch: true });
    const order = [
      "fetch-form-log-stream-input",
      "fetch-form-start-input",
      "fetch-form-end-input",
      "fetch-form-submit-button",
    ];
    for (const testId of order) {
      await user.tab();
      expect(screen.getByTestId(testId)).toHaveFocus();
    }
  });
});
