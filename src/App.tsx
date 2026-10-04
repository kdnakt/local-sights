import { useCallback, useEffect, useMemo, useState } from "react";
import {
  getSession,
  isCommandError,
  onLogBatch,
  onSessionChanged,
  startFetch,
  updateInput,
  type InputField,
  type LogBatch,
  type LogEvent,
  type SessionView,
} from "./api";
import { FetchForm } from "./components/FetchForm";
import { LogTable } from "./components/LogTable";
import { StatusLine } from "./components/StatusLine";
import { useEscapeKey } from "./hooks/useEscapeKey";
import { createTranslator, type Locale } from "./i18n/messages";

interface Rows {
  jobId: string | null;
  events: LogEvent[];
}

const NO_ROWS: Rows = { jobId: null, events: [] };

/** Rows belong to one job; a different job starts from an empty list. */
function rowsForSession(rows: Rows, view: SessionView): Rows {
  return view.currentJobId === rows.jobId ? rows : { jobId: view.currentJobId, events: [] };
}

function rowsWithBatch(rows: Rows, batch: LogBatch): Rows {
  if (batch.jobId !== rows.jobId) {
    return { jobId: batch.jobId, events: [...batch.events] };
  }
  return { jobId: rows.jobId, events: [...rows.events, ...batch.events] };
}

export interface AppProps {
  locale: Locale;
}

/**
 * The single window. It holds no state of its own beyond mirroring what the
 * core pushes: the session view and the rows of the current job (ADR-001).
 */
export function App({ locale }: AppProps) {
  const t = useMemo(() => createTranslator(locale), [locale]);
  const [session, setSession] = useState<SessionView | null>(null);
  const [rows, setRows] = useState<Rows>(NO_ROWS);
  const [commandError, setCommandError] = useState<string | null>(null);

  const applySession = useCallback((view: SessionView) => {
    setSession(view);
    setRows((previous) => rowsForSession(previous, view));
  }, []);

  const reportError = useCallback(
    (error: unknown) => {
      setCommandError(
        isCommandError(error) ? t(error.key) : t("error.command", { detail: String(error) }),
      );
    },
    [t],
  );

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const keep = (unlisten: () => void) => {
      if (disposed) {
        unlisten();
      } else {
        unlisteners.push(unlisten);
      }
    };
    onSessionChanged(applySession).then(keep, reportError);
    onLogBatch((batch) => setRows((previous) => rowsWithBatch(previous, batch))).then(
      keep,
      reportError,
    );
    getSession().then((view) => {
      if (!disposed) {
        applySession(view);
      }
    }, reportError);
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [applySession, reportError]);

  const dismissError = useCallback(() => setCommandError(null), []);
  useEscapeKey(dismissError, commandError !== null);

  const handleChange = useCallback(
    (field: InputField, value: string) => {
      updateInput(field, value).then(applySession, reportError);
    },
    [applySession, reportError],
  );

  const handleFetch = useCallback(() => {
    setCommandError(null);
    startFetch().then(applySession, reportError);
  }, [applySession, reportError]);

  return (
    <main className="app" data-testid="app">
      <h1 className="app-title">{t("app.title")}</h1>
      {session && (
        <FetchForm
          key={session.sessionId}
          session={session}
          t={t}
          onChange={handleChange}
          onFetch={handleFetch}
        />
      )}
      <StatusLine session={session} liveCount={rows.events.length} t={t} />
      {commandError && (
        <div className="command-error" role="alert" data-testid="command-error">
          <span>{commandError}</span>{" "}
          <button type="button" data-testid="command-error-dismiss-button" onClick={dismissError}>
            {t("error.dismiss")}
          </button>
        </div>
      )}
      <LogTable events={rows.events} t={t} />
    </main>
  );
}
