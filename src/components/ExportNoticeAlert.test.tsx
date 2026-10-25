import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ExportNotice } from "../api";
import { createTranslator } from "../i18n/messages";
import { t } from "../test/fixtures";
import { ExportNoticeAlert } from "./ExportNoticeAlert";

function renderNotice(notice: ExportNotice | null, translate = t) {
  const onDismiss = vi.fn();
  render(<ExportNoticeAlert notice={notice} t={translate} onDismiss={onDismiss} />);
  return { onDismiss };
}

describe("ExportNoticeAlert (261022-small-features U2:BR4.9, review R-02)", () => {
  it("tells a saved export with its path and row count in a status region", () => {
    renderNotice({ kind: "Saved", path: "/Users/me/orders_20261024-213045.csv", rowCount: 1234 });
    const notice = screen.getByTestId("export-notice");
    expect(notice).toHaveAttribute("role", "status");
    expect(notice).toHaveTextContent("Saved 1234 rows to /Users/me/orders_20261024-213045.csv.");
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("tells a copied export with its row count, in Japanese too", () => {
    const { unmount } = render(
      <ExportNoticeAlert notice={{ kind: "Copied", rowCount: 7 }} t={t} onDismiss={vi.fn()} />,
    );
    expect(screen.getByTestId("export-notice")).toHaveTextContent(
      "Copied 7 rows to the clipboard.",
    );
    unmount();
    renderNotice({ kind: "Copied", rowCount: 7 }, createTranslator("ja"));
    expect(screen.getByTestId("export-notice")).toHaveTextContent(
      "7 行をクリップボードにコピーしました。",
    );
  });

  it("tells a failure as an alert, built from its kind and the path, never the OS's words", () => {
    const { unmount } = render(
      <ExportNoticeAlert
        notice={{ kind: "SaveFailed", failure: "PermissionDenied", path: "/root/out.csv" }}
        t={t}
        onDismiss={vi.fn()}
      />,
    );
    const notice = screen.getByTestId("export-notice");
    expect(notice).toHaveAttribute("role", "alert");
    expect(notice).toHaveTextContent(
      "The file could not be saved. You do not have permission to write /root/out.csv.",
    );
    unmount();
    renderNotice({ kind: "CopyFailed", failure: "Clipboard" }, createTranslator("ja"));
    expect(screen.getByTestId("export-notice")).toHaveTextContent(
      "クリップボードにコピーできませんでした。クリップボードが受け付けませんでした。",
    );
  });

  it("calls onDismiss when closed and draws nothing without a notice", async () => {
    const user = userEvent.setup();
    const { onDismiss } = renderNotice({ kind: "Saved", path: "/tmp/a.md", rowCount: 1 });
    await user.click(
      within(screen.getByTestId("export-notice")).getByRole("button", { name: "Close" }),
    );
    expect(onDismiss).toHaveBeenCalledTimes(1);
    const { container } = render(<ExportNoticeAlert notice={null} t={t} onDismiss={vi.fn()} />);
    expect(container).toBeEmptyDOMElement();
  });
});
