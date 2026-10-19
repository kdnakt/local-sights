import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { AppPage } from "../analytics";
import { createTranslator } from "../i18n/messages";
import { t } from "../test/fixtures";
import { AppNavigation } from "./AppNavigation";

function renderNavigation(page: AppPage = "logManagement", open = true) {
  const handlers = { onNavigate: vi.fn(), onOpenChange: vi.fn() };
  const view = render(<AppNavigation page={page} open={open} t={t} {...handlers} />);
  return { ...handlers, ...view };
}

describe("AppNavigation (261017-insights-query U1:BR1.1, refined-mockups screens 1, 1b)", () => {
  it("lists the two screens and marks the current one", () => {
    renderNavigation();
    const navigation = screen.getByRole("navigation", { name: "Screens" });
    expect(navigation).toHaveTextContent("Log Management");
    expect(navigation).toHaveTextContent("Log Analytics");
    expect(screen.getByRole("link", { name: "Log Management" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getByRole("link", { name: "Log Analytics" })).not.toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("marks Log Analytics when it is the current screen", () => {
    renderNavigation("logAnalytics");
    expect(screen.getByRole("link", { name: "Log Analytics" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("switches the screen without following the link", async () => {
    const user = userEvent.setup();
    const { onNavigate } = renderNavigation();
    await user.click(screen.getByRole("link", { name: "Log Analytics" }));
    expect(onNavigate).toHaveBeenCalledWith("logAnalytics");
    expect(window.location.hash).toBe("");
    await user.click(screen.getByRole("link", { name: "Log Management" }));
    expect(onNavigate).toHaveBeenLastCalledWith("logManagement");
  });

  it("closes to a strip with only the open button, which opens it again", async () => {
    const user = userEvent.setup();
    const open = renderNavigation();
    await user.click(screen.getByTestId("app-navigation-close-button"));
    expect(open.onOpenChange).toHaveBeenCalledWith(false);
    open.unmount();
    const closed = renderNavigation("logManagement", false);
    expect(screen.queryByRole("navigation")).not.toBeInTheDocument();
    expect(screen.getByTestId("app-navigation-collapsed")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Open navigation" }));
    expect(closed.onOpenChange).toHaveBeenCalledWith(true);
  });

  it("moves the focus to the button that undoes a close or an open", () => {
    const handlers = { onNavigate: vi.fn(), onOpenChange: vi.fn() };
    const { rerender } = render(<AppNavigation page="logManagement" open t={t} {...handlers} />);
    expect(document.body).toHaveFocus();
    rerender(<AppNavigation page="logManagement" open={false} t={t} {...handlers} />);
    expect(screen.getByTestId("app-navigation-open-button")).toHaveFocus();
    rerender(<AppNavigation page="logManagement" open t={t} {...handlers} />);
    expect(screen.getByTestId("app-navigation-close-button")).toHaveFocus();
  });

  it("is shown in Japanese for the ja locale", () => {
    render(
      <AppNavigation
        page="logManagement"
        open
        t={createTranslator("ja")}
        onNavigate={vi.fn()}
        onOpenChange={vi.fn()}
      />,
    );
    expect(screen.getByRole("link", { name: "ログ管理" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "ナビゲーションを閉じる" })).toBeInTheDocument();
  });
});
