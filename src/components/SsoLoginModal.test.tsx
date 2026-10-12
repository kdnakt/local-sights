import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { SsoLoginView } from "../api";
import { createTranslator } from "../i18n/messages";
import { ssoLoginView, t } from "../test/fixtures";
import { SsoLoginModal } from "./SsoLoginModal";

const DEVICE_URL = "https://device.sso.us-east-1.amazonaws.com/";

function renderModal(login: SsoLoginView, translate = t) {
  const handlers = { onCancel: vi.fn(), onClose: vi.fn(), onOpenUrl: vi.fn() };
  render(<SsoLoginModal login={login} t={translate} {...handlers} />);
  return handlers;
}

/** Cloudscape's modal reads the key code; user-event sends none, so Escape is fired by hand. */
const pressEscape = () =>
  fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape", keyCode: 27 });

const waiting = (overrides: Partial<SsoLoginView> = {}) =>
  ssoLoginView({
    available: true,
    state: "Waiting",
    attemptId: 1,
    verificationUrl: DEVICE_URL,
    userCode: "ABCD-EFGH",
    profileOption: "--profile dev",
    ...overrides,
  });

describe("SsoLoginModal (261011-sso-login FR2.2-FR2.5, FR3.3, FR4.2)", () => {
  it("shows the guidance with a spinner and text, the URL, the code and [Cancel] while waiting", async () => {
    renderModal(waiting());
    const status = screen.getByTestId("sso-login-modal-waiting");
    expect(status).toHaveAttribute("role", "status");
    expect(status).toHaveTextContent("Approve the sign-in in your browser.");
    expect(screen.getByTestId("sso-login-modal-url-link")).toHaveTextContent(DEVICE_URL);
    expect(screen.getByTestId("sso-login-modal-code")).toHaveTextContent("Verification code");
    expect(screen.getByTestId("sso-login-modal-code")).toHaveTextContent("ABCD-EFGH");
    await waitFor(() => expect(screen.getByTestId("sso-login-modal-cancel-button")).toHaveFocus());
    expect(screen.queryByTestId("sso-login-modal-close-button")).not.toBeInTheDocument();
  });

  it("shows only the URL when the CLI prints no code (PKCE), in Japanese too", () => {
    renderModal(waiting({ userCode: null }), createTranslator("ja"));
    expect(screen.getByTestId("sso-login-modal-waiting")).toHaveTextContent(
      "ブラウザでログインを承認してください。",
    );
    expect(screen.getByTestId("sso-login-modal-url-link")).toBeInTheDocument();
    expect(screen.queryByTestId("sso-login-modal-code")).not.toBeInTheDocument();
  });

  it("opens the URL through the app instead of navigating", async () => {
    const user = userEvent.setup();
    const { onOpenUrl } = renderModal(waiting());
    await user.click(screen.getByTestId("sso-login-modal-url-link"));
    expect(onOpenUrl).toHaveBeenCalledWith(DEVICE_URL);
    expect(window.location.href).not.toBe(DEVICE_URL);
  });

  it("cancels with [Cancel]", async () => {
    const user = userEvent.setup();
    const { onCancel, onClose } = renderModal(
      waiting({ state: "Starting", verificationUrl: null }),
    );
    expect(screen.queryByTestId("sso-login-modal-url")).not.toBeInTheDocument();
    await user.click(screen.getByTestId("sso-login-modal-cancel-button"));
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();
  });

  it("does not close with Escape or the close button while waiting", async () => {
    const user = userEvent.setup();
    const { onCancel, onClose } = renderModal(waiting());
    pressEscape();
    await user.click(screen.getByRole("button", { name: "Close" }));
    expect(onCancel).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByTestId("sso-login-modal")).toBeInTheDocument();
  });

  it.each([
    [ssoLoginView({ state: "TimedOut" }), "The sign-in timed out. Try again."],
    [ssoLoginView({ state: "Failed", exitCode: 255 }), "Could not sign in (exit code 255)."],
    [
      ssoLoginView({ state: "Failed", exitCode: null }),
      "Could not sign in: the AWS CLI could not be started.",
    ],
    [ssoLoginView({ state: "CliNotFound" }), "AWS CLI was not found. Install AWS CLI v2."],
  ])("tells a result in words with the terminal fallback and closes (%#)", async (login, text) => {
    const user = userEvent.setup();
    const { onClose, onCancel } = renderModal({ ...login, profileOption: "--profile dev" });
    expect(screen.getByTestId("sso-login-modal-result")).toHaveTextContent(text);
    expect(screen.getByTestId("sso-login-modal-fallback")).toHaveTextContent(
      "You can also run aws sso login --profile dev in a terminal and press [Reload].",
    );
    expect(screen.queryByTestId("sso-login-modal-waiting")).not.toBeInTheDocument();
    await user.click(screen.getByTestId("sso-login-modal-close-button"));
    expect(onClose).toHaveBeenCalledTimes(1);
    pressEscape();
    expect(onClose).toHaveBeenCalledTimes(2);
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("writes the fallback without --profile when none is needed", () => {
    renderModal(ssoLoginView({ state: "TimedOut", profileOption: "" }), createTranslator("ja"));
    expect(screen.getByTestId("sso-login-modal-fallback")).toHaveTextContent(
      "ターミナルで aws sso login を実行してから、[再読み込み] を押してもログインできます。",
    );
  });
});
