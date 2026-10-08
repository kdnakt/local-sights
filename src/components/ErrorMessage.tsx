import type { ApiFailure } from "../api";
import { errorLines, type ErrorScene } from "../errorText";
import type { Translate } from "../i18n/messages";

export interface ErrorMessageProps {
  failure: ApiFailure;
  /** Where the error is shown; decides the [Fetch] / [Reload] wording (BR2.1). */
  scene: ErrorScene;
  t: Translate;
  /** Only the "what happened" sentence, for the status line (BR2.4). */
  compact?: boolean;
  /** Prefix of the `data-testid` of each line. */
  testId?: string;
}

/**
 * An error in words (U7:BR2.2-BR2.5): what happened, what to do next and,
 * when there is one, the safe detail, each on its own line. Never a color
 * or an icon alone. `compact` keeps the first sentence only.
 */
export function ErrorMessage({
  failure,
  scene,
  t,
  compact = false,
  testId = "error-message",
}: ErrorMessageProps) {
  const lines = errorLines(failure.kind, scene, failure.safeDetail, t);
  if (compact) {
    return (
      <span className="error-message-what" data-testid={`${testId}-what`}>
        {lines.what}
      </span>
    );
  }
  return (
    <div className="error-message" data-testid={testId}>
      <p className="error-message-what" data-testid={`${testId}-what`}>
        {lines.what}
      </p>
      <p className="error-message-next" data-testid={`${testId}-next`}>
        {lines.next}
      </p>
      {lines.detail !== null && (
        <p className="error-message-detail" data-testid={`${testId}-detail`}>
          {lines.detail}
        </p>
      )}
    </div>
  );
}
