import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { limitSelection } from "../analytics";
import type { ListingStatus } from "../api";
import { t } from "../test/fixtures";
import { LogGroupMultiselect } from "./LogGroupMultiselect";

const GROUPS = ["/aws/ecs/web", "/aws/lambda/MyFunction", "/aws/lambda/orders"];

function renderSelect({
  selected = [] as readonly string[],
  status = "Complete" as ListingStatus | null,
  disabled = false,
  max = 50,
  options = GROUPS as readonly string[],
} = {}) {
  const onChange = vi.fn();
  render(
    <LogGroupMultiselect
      options={options}
      selected={selected}
      max={max}
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

describe("LogGroupMultiselect (261017-insights-query U1:BR1.3, U3:BR4.1)", () => {
  it("offers every log group of the list and shows each chosen one as a token", async () => {
    const user = userEvent.setup();
    renderSelect({ selected: ["/aws/ecs/web", "/aws/lambda/orders"] });
    const field = screen.getByTestId("log-group-multiselect");
    expect(field).toHaveTextContent("/aws/ecs/web");
    expect(field).toHaveTextContent("/aws/lambda/orders");
    await user.click(trigger());
    const options = screen.getAllByRole("option").map((option) => option.textContent);
    expect(options).toEqual(GROUPS);
  });

  it("adds a newly chosen log group to the ones chosen before (U3:BR4.1)", async () => {
    const user = userEvent.setup();
    const onChange = renderSelect({ selected: ["/aws/ecs/web"] });
    await user.click(trigger());
    await user.click(screen.getByRole("option", { name: "/aws/lambda/orders" }));
    expect(onChange).toHaveBeenCalledWith(["/aws/ecs/web", "/aws/lambda/orders"]);
  });

  it("removes one log group when its token's × is pressed", async () => {
    const user = userEvent.setup();
    const onChange = renderSelect({ selected: ["/aws/ecs/web", "/aws/lambda/orders"] });
    await user.click(screen.getByRole("button", { name: "Remove /aws/ecs/web" }));
    expect(onChange).toHaveBeenCalledWith(["/aws/lambda/orders"]);
  });

  it("says the limit and offers no other log group once fifty are chosen (U3:BR4.1)", async () => {
    const user = userEvent.setup();
    const many = Array.from(
      { length: 51 },
      (_, index) => `/g/${String(index + 1).padStart(2, "0")}`,
    );
    const onChange = renderSelect({ selected: many.slice(0, 50), options: many });
    expect(screen.getByTestId("log-group-limit")).toHaveTextContent(
      "You can choose up to 50 log groups.",
    );
    await user.click(trigger());
    const last = screen.getByRole("option", { name: "/g/51" });
    expect(last).toHaveAttribute("aria-disabled", "true");
    await user.click(last);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("says nothing of the limit below it", () => {
    renderSelect({ selected: ["/aws/ecs/web"] });
    expect(screen.queryByTestId("log-group-limit")).toBeNull();
  });

  it("says when the list is loading or could not be read, and asks when nothing is chosen", async () => {
    const user = userEvent.setup();
    renderSelect({ status: "Loading" });
    expect(trigger()).toHaveTextContent("Select log groups");
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

  it("keeps each chosen value once, in order, up to the limit (pure)", () => {
    expect(limitSelection([], 50)).toEqual([]);
    expect(limitSelection(["/b", "/a", "/b", ""], 50)).toEqual(["/b", "/a"]);
    expect(limitSelection(["/a", "/b", "/c"], 2)).toEqual(["/a", "/b"]);
  });
});
