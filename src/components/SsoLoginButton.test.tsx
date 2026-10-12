import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { createTranslator } from "../i18n/messages";
import { sessionView, ssoLoginView, t } from "../test/fixtures";
import { SsoLoginButton } from "./SsoLoginButton";

const TEST_ID = "test-sso-login-button";

function renderButton(login = ssoLoginView({ available: true, canStart: true }), translate = t) {
  const onStart = vi.fn();
  render(
    <SsoLoginButton
      session={sessionView({ ssoLogin: login })}
      t={translate}
      onStart={onStart}
      testId={TEST_ID}
    />,
  );
  return onStart;
}

describe("SsoLoginButton (261011-sso-login FR1.1, BR1.5, BR2.5)", () => {
  it("is shown when the core says the login is available", () => {
    renderButton();
    expect(screen.getByTestId(TEST_ID)).toHaveTextContent("Sign in with SSO");
    expect(screen.getByTestId(TEST_ID)).toBeEnabled();
  });

  it("is not shown when the profile cannot sign in with SSO", () => {
    renderButton(ssoLoginView({ available: false, canStart: false }));
    expect(screen.queryByTestId(TEST_ID)).not.toBeInTheDocument();
  });

  it("is disabled while other work or a login runs, in Japanese too", () => {
    renderButton(
      ssoLoginView({ available: true, canStart: false, state: "Waiting" }),
      createTranslator("ja"),
    );
    expect(screen.getByTestId(TEST_ID)).toHaveTextContent("SSO にログイン");
    expect(screen.getByTestId(TEST_ID)).toBeDisabled();
  });

  it("starts the login by click or keyboard and hands over its own ref", async () => {
    const user = userEvent.setup();
    const onStart = renderButton();
    await user.click(screen.getByTestId(TEST_ID));
    expect(onStart).toHaveBeenCalledTimes(1);
    const ref = onStart.mock.calls[0]?.[0] as { current: { focus(): void } | null };
    expect(ref.current).not.toBeNull();
    screen.getByTestId(TEST_ID).blur();
    ref.current?.focus();
    expect(screen.getByTestId(TEST_ID)).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(onStart).toHaveBeenCalledTimes(2);
  });
});
