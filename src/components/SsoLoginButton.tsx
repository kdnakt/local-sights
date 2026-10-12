import { useRef, type RefObject } from "react";
import Button, { type ButtonProps } from "@cloudscape-design/components/button";
import type { SessionView } from "../api";
import type { Translate } from "../i18n/messages";

export interface SsoLoginButtonProps {
  session: SessionView;
  t: Translate;
  /**
   * Starts the login. The button's ref comes along, so the focus can go back
   * to it when the dialog closes (its `current` is null once it is gone).
   */
  onStart: (returnFocus: RefObject<ButtonProps.Ref | null>) => void;
  /** `data-testid` of this placement (the list error or the fetch error). */
  testId: string;
}

/**
 * [Sign in with SSO] (261011-sso-login FR1.1, BR1.5, BR2.5): a Cloudscape
 * button shown inside an auth error only when the core says the selected
 * profile leads to an SSO profile (`ssoLogin.available`), and disabled while
 * a fetch, a log group listing or a login runs (`ssoLogin.canStart`). It is
 * a normal button, so the keyboard reaches it (NFR7).
 */
export function SsoLoginButton({ session, t, onStart, testId }: SsoLoginButtonProps) {
  const ref = useRef<ButtonProps.Ref>(null);
  if (!session.ssoLogin.available) {
    return null;
  }
  return (
    <Button
      ref={ref}
      disabled={!session.ssoLogin.canStart}
      onClick={() => onStart(ref)}
      data-testid={testId}
    >
      {t("ssoLogin.button")}
    </Button>
  );
}
