import { useCallback, useEffect, useMemo, useState } from "react";
import {
  cancelConnectionChange,
  confirmConnectionChange,
  getSession,
  isCommandError,
  onLogBatch,
  onSessionChanged,
  reloadLogGroups,
  selectLogGroup,
  selectProfile,
  selectRegion,
  startFetch,
  updateInput,
  updateLogGroupFilter,
  type InputField,
  type LogBatch,
  type LogEvent,
  type ProfileSelector,
  type SessionView,
} from "./api";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { ConnectionBar } from "./components/ConnectionBar";
import { FetchForm } from "./components/FetchForm";
import { LogGroupPane } from "./components/LogGroupPane";
import { LogTable } from "./components/LogTable";
import { StatusLine } from "./components/StatusLine";
import { useEscapeKey } from "./hooks/useEscapeKey";
import { createTranslator, type Locale } from "./i18n/messages";

interface Rows {
  jobId: string | null;
  generation: number;
  events: LogEvent[];
}

const NO_ROWS: Rows = { jobId: null, generation: 0, events: [] };

/**
 * Rows belong to one job. A connection change that discarded the shown logs
 * (a new timeline generation, U2:BR2.4) empties them. Otherwise only a known
 * job ID that differs from the rows' job starts an empty list; a view without
 * a job ID (e.g. "Fetching" before the job has started) never discards rows,
 * whatever order it arrives in.
 */
function rowsForSession(rows: Rows, view: SessionView): Rows {
  if (view.timelineGeneration !== rows.generation) {
    return { jobId: view.currentJobId, generation: view.timelineGeneration, events: [] };
  }
  if (view.currentJobId === null || view.currentJobId === rows.jobId) {
    return rows;
  }
  return { jobId: view.currentJobId, generation: rows.generation, events: [] };
}

function rowsWithBatch(rows: Rows, batch: LogBatch): Rows {
  if (batch.jobId !== rows.jobId) {
    return { jobId: batch.jobId, generation: rows.generation, events: [...batch.events] };
  }
  return { ...rows, events: [...rows.events, ...batch.events] };
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

  // Connection and list commands return nothing; their result arrives as a
  // `session-changed` event, so a late response never overwrites newer state.
  const run = useCallback(
    (command: () => Promise<void>) => {
      command().catch(reportError);
    },
    [reportError],
  );
  const handleSelectProfile = useCallback(
    (profile: ProfileSelector) => run(() => selectProfile(profile)),
    [run],
  );
  const handleSelectRegion = useCallback(
    (region: string) => run(() => selectRegion(region)),
    [run],
  );
  const handleFilter = useCallback((text: string) => run(() => updateLogGroupFilter(text)), [run]);
  const handleReload = useCallback(() => run(reloadLogGroups), [run]);
  const handleSelectLogGroup = useCallback(
    (name: string) => run(() => selectLogGroup(name)),
    [run],
  );
  const handleConfirm = useCallback(() => run(confirmConnectionChange), [run]);
  const handleCancel = useCallback(() => run(cancelConnectionChange), [run]);

  const handleFetch = useCallback(() => {
    setCommandError(null);
    // The new state arrives through events only (see `startFetch`).
    startFetch().catch(reportError);
  }, [reportError]);

  return (
    <main className="app" data-testid="app">
      <h1 className="app-title">{t("app.title")}</h1>
      {session && (
        <ConnectionBar
          session={session}
          t={t}
          onSelectProfile={handleSelectProfile}
          onSelectRegion={handleSelectRegion}
        />
      )}
      <div className="app-body">
        {session && (
          <LogGroupPane
            key={session.sessionId}
            session={session}
            t={t}
            onFilterChange={handleFilter}
            onReload={handleReload}
            onSelect={handleSelectLogGroup}
          />
        )}
        <section className="app-main">
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
              <button
                type="button"
                data-testid="command-error-dismiss-button"
                onClick={dismissError}
              >
                {t("error.dismiss")}
              </button>
            </div>
          )}
          <LogTable events={rows.events} t={t} />
        </section>
      </div>
      {session?.pendingChange && (
        <ConfirmDialog t={t} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
    </main>
  );
}
