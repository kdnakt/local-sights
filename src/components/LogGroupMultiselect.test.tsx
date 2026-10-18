import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { newestSelection } from "../analytics";
import type { ListingStatus } from "../api";
import { t } from "../test/fixtures";
import { LogGroupMultiselect } from "./LogGroupMultiselect";

const GROUPS = ["/aws/ecs/web", "/aws/lambda/MyFunction", "/aws/lambda/orders"];

function renderSelect({
  selected = null as string | null,
  status = "Complete" as ListingStatus | null,
  disabled = false,
} = {}) {
  const onChange = vi.fn();
  render(
    <LogGroupMultiselect
      options={GROUPS}
      selected={selected}
      status={status}
      disabled={disabled}
      t={t}
      onChange={onChange}
    />,
  );
  return onChange;
}

const trigger = () =>
  within(screen.getByTestId("log-group-multiselect")).getAllByRole("button")[0]!;

describe("LogGroupMultiselect (261017-insights-query U1:BR1.3, refined-mockups Q2)", () => {
  it("offers every log group of the list and shows the chosen one as a token", async () => {
    const user = userEvent.setup();
    renderSelect({ selected: "/aws/ecs/web" });
    expect(screen.getByTestId("log-group-multiselect")).toHaveTextContent("/aws/ecs/web");
    await user.click(trigger());
    const options = screen.getAllByRole("option").map((option) => option.textContent);
    expect(options).toEqual(GROUPS);
  });

  it("replaces the chosen log group with the newest choice (one in U1)", async () => {
    const user = userEvent.setup();
    const onChange = renderSelect({ selected: "/aws/ecs/web" });
    await user.click(trigger());
    await user.click(screen.getByRole("option", { name: "/aws/lambda/orders" }));
    expect(onChange).toHaveBeenCalledWith("/aws/lambda/orders");
  });

  it("deselects when the token is removed", async () => {
    const user = userEvent.setup();
    const onChange = renderSelect({ selected: "/aws/ecs/web" });
    await user.click(screen.getByRole("button", { name: "Remove /aws/ecs/web" }));
    expect(onChange).toHaveBeenCalledWith(null);
  });

  it("says when the list is loading or could not be read, and asks when nothing is chosen", async () => {
    const user = userEvent.setup();
    renderSelect({ status: "Loading" });
    expect(trigger()).toHaveTextContent("Select a log group");
    await user.click(trigger());
    // The dropdown is drawn outside the field (expandToViewport).
    expect(screen.getByText("Loading log groups")).toBeInTheDocument();
  });

  it("says the list could not be read when it is partial (refined-mockups screen 6e)", async () => {
    const user = userEvent.setup();
    renderSelect({ status: "Partial" });
    await user.click(trigger());
    expect(screen.getByText("Couldn't load log groups")).toBeInTheDocument();
    expect(screen.getAllByRole("option")).toHaveLength(GROUPS.length);
  });

  it("cannot be changed while the query runs", () => {
    renderSelect({ disabled: true, status: "Partial" });
    expect(trigger()).toBeDisabled();
  });

  it("keeps the newest of the chosen values (pure)", () => {
    expect(newestSelection([], "/a")).toBeNull();
    expect(newestSelection(["/a", "/b"], "/a")).toBe("/b");
    expect(newestSelection(["/b"], "/a")).toBe("/b");
    expect(newestSelection(["/a"], "/a")).toBe("/a");
  });
});
