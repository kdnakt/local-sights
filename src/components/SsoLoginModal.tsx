import Box from "@cloudscape-design/components/box";
import Button from "@cloudscape-design/components/button";
import Link from "@cloudscape-design/components/link";
import Modal from "@cloudscape-design/components/modal";
import SpaceBetween from "@cloudscape-design/components/space-between";
import Spinner from "@cloudscape-design/components/spinner";
import StatusIndicator from "@cloudscape-design/components/status-indicator";
import { isSsoLoginRunning, type SsoLoginView } from "../api";
import { useInitialFocus } from "../hooks/useInitialFocus";
import type { Translate } from "../i18n/messages";

export interface SsoLoginModalProps {
  login: SsoLoginView;
  t: Translate;
  /** [Cancel] while the CLI waits (FR2.4). */
  onCancel: () => void;
  /** [Close] on a result (time-out, failure, no CLI). */
  onClose: () => void;
  /** [Sign in with SSO] on a result: starts a new attempt from the dialog. */
  onRetry: () => void;
  /** Opens the sign-in URL in the browser (the app checks it again). */
  onOpenUrl: (url: string) => void;
}

/**
 * The SSO login dialog in the middle of the screen (261011-sso-login FR2.2,
 * FR2.3, FR2.4, FR3.3, FR4.2; frontend-components.md). While the CLI starts
 * or waits it shows what to do, a spinner with the same words as text (so
 * the waiting is not shown by the spinner alone, NFR7), the URL the CLI
 * printed as a link the app opens, the verification code when there is
 * one (the newer CLI's PKCE flow prints none) and [Cancel]; the close button
 * and Escape do nothing then, since the CLI runs. A time-out, a failure (with
 * the exit code) or a missing CLI is told with the terminal fallback,
 * [Sign in with SSO] to try again right there (shown while the core offers
 * the login, disabled while it cannot start) and [Close], which the close
 * button and Escape also mean. Focus starts on [Cancel] or [Close].
 */
export function SsoLoginModal({
  login,
  t,
  onCancel,
  onClose,
  onRetry,
  onOpenUrl,
}: SsoLoginModalProps) {
  const footerRef = useInitialFocus();
  const running = isSsoLoginRunning(login);
  return (
    <Modal
      visible
      size="medium"
      header={t("ssoLogin.title")}
      closeAriaLabel={t("ssoLogin.close")}
      onDismiss={() => {
        if (!running) {
          onClose();
        }
      }}
      footer={
        <Box float="right">
          {running ? (
            <Button ref={footerRef} onClick={onCancel} data-testid="sso-login-modal-cancel-button">
              {t("ssoLogin.cancel")}
            </Button>
          ) : (
            <SpaceBetween direction="horizontal" size="xs">
              <Button ref={footerRef} onClick={onClose} data-testid="sso-login-modal-close-button">
                {t("ssoLogin.close")}
              </Button>
              {login.available && (
                <Button
                  variant="primary"
                  disabled={!login.canStart}
                  onClick={onRetry}
                  data-testid="sso-login-modal-retry-button"
                >
                  {t("ssoLogin.button")}
                </Button>
              )}
            </SpaceBetween>
          )}
        </Box>
      }
      data-testid="sso-login-modal"
    >
      {running ? (
        <WaitingContent login={login} t={t} onOpenUrl={onOpenUrl} />
      ) : (
        <ResultContent login={login} t={t} />
      )}
    </Modal>
  );
}

function WaitingContent({
  login,
  t,
  onOpenUrl,
}: Pick<SsoLoginModalProps, "login" | "t" | "onOpenUrl">) {
  const url = login.verificationUrl;
  return (
    <SpaceBetween size="m">
      <div role="status" className="sso-login-waiting" data-testid="sso-login-modal-waiting">
        <Spinner size="normal" />
        <span>{t("ssoLogin.waiting")}</span>
      </div>
      {url !== null && (
        <Box data-testid="sso-login-modal-url">
          <Link
            href={url}
            external
            onFollow={(event) => {
              // The webview never navigates; the app opens the browser.
              event.preventDefault();
              onOpenUrl(url);
            }}
            data-testid="sso-login-modal-url-link"
          >
            {url}
          </Link>
        </Box>
      )}
      {login.userCode !== null && (
        <div data-testid="sso-login-modal-code">
          <Box variant="awsui-key-label">{t("ssoLogin.code")}</Box>
          <Box fontSize="display-l" fontWeight="bold">
            {login.userCode}
          </Box>
        </div>
      )}
    </SpaceBetween>
  );
}

function ResultContent({ login, t }: Pick<SsoLoginModalProps, "login" | "t">) {
  return (
    <SpaceBetween size="s">
      <div role="alert" data-testid="sso-login-modal-result">
        <StatusIndicator type="error" iconAriaLabel={t("failure.kind.Other")}>
          {resultText(login, t)}
        </StatusIndicator>
      </div>
      <Box data-testid="sso-login-modal-fallback">{fallbackText(login, t)}</Box>
    </SpaceBetween>
  );
}

/** What a result says in words (FR2.5, FR3.3, FR4.2). */
function resultText(login: SsoLoginView, t: Translate): string {
  switch (login.state) {
    case "TimedOut":
      return t("ssoLogin.timedOut");
    case "CliNotFound":
      return t("ssoLogin.cliNotFound");
    default:
      return login.exitCode === null
        ? t("ssoLogin.failedToStart")
        : t("ssoLogin.failed", { code: login.exitCode });
  }
}

/** The terminal fallback, with `--profile <name>` only when it is needed. */
function fallbackText(login: SsoLoginView, t: Translate): string {
  return t("ssoLogin.fallback", { profileOption: login.profileOption }).replace(/ {2,}/g, " ");
}
