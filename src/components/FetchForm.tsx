import { useState, type FormEvent } from "react";
import type { FetchInput, InputField, SessionView } from "../api";
import type { MessageKey, Translate } from "../i18n/messages";

interface FieldSpec {
  field: InputField;
  label: MessageKey;
  placeholder: MessageKey;
  testId: string;
}

const FIELDS: readonly FieldSpec[] = [
  {
    field: "logStreamName",
    label: "form.logStream.label",
    placeholder: "form.logStream.placeholder",
    testId: "fetch-form-log-stream-input",
  },
  {
    field: "startText",
    label: "form.start.label",
    placeholder: "form.dateTime.placeholder",
    testId: "fetch-form-start-input",
  },
  {
    field: "endText",
    label: "form.end.label",
    placeholder: "form.dateTime.placeholder",
    testId: "fetch-form-end-input",
  },
];

export interface FetchFormProps {
  session: SessionView;
  t: Translate;
  onChange: (field: InputField, value: string) => void;
  onFetch: () => void;
}

/**
 * The selected log group (always shown, U2:BR3.8), the stream and time
 * inputs, and the Fetch button. Fetch works only when the core says the
 * selection and input are valid; otherwise the reasons are listed. While
 * fetching (U1:BR1.4) or while a connection change awaits confirmation
 * (U2:BR2.6), every control is disabled. Enter in any input submits
 * the form (U1:BR6.2).
 */
export function FetchForm({ session, t, onChange, onFetch }: FetchFormProps) {
  // Local drafts keep typing responsive; every change is forwarded to the
  // core, which owns the state. Mount with a `key` to reset from a session.
  const [drafts, setDrafts] = useState<FetchInput>(session.input);
  const fetching = session.phase === "Fetching";
  const locked = fetching || session.pendingChange !== null;
  const canSubmit = session.canFetch && !locked;

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (canSubmit) {
      onFetch();
    }
  };

  const handleChange = (field: InputField, value: string) => {
    setDrafts((previous) => ({ ...previous, [field]: value }));
    onChange(field, value);
  };

  return (
    <form
      className="fetch-form"
      aria-label={t("form.label")}
      data-testid="fetch-form"
      onSubmit={handleSubmit}
    >
      <div className="fetch-form-fields">
        <div className="fetch-form-field">
          <span>{t("form.selectedLogGroup.label")}</span>
          <output
            className="fetch-form-selected-log-group"
            data-testid="fetch-form-selected-log-group"
          >
            {session.selectedLogGroupName ?? t("form.selectedLogGroup.none")}
          </output>
        </div>
        {FIELDS.map((spec) => (
          <label key={spec.field} className="fetch-form-field">
            <span>{t(spec.label)}</span>
            <input
              type="text"
              name={spec.field}
              value={drafts[spec.field]}
              placeholder={t(spec.placeholder)}
              disabled={locked}
              autoComplete="off"
              spellCheck={false}
              data-testid={spec.testId}
              onChange={(event) => handleChange(spec.field, event.target.value)}
            />
          </label>
        ))}
        <button type="submit" disabled={!canSubmit} data-testid="fetch-form-submit-button">
          {t("form.fetch")}
        </button>
      </div>
      {!fetching && session.validationErrors.length > 0 && (
        <div className="fetch-form-reasons" data-testid="fetch-form-reasons">
          <span>{t("form.reasons.title")}</span>
          <ul>
            {session.validationErrors.map((key) => (
              <li key={key}>{t(key)}</li>
            ))}
          </ul>
        </div>
      )}
    </form>
  );
}
