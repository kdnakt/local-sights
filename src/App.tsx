import { useCallback, useEffect, useMemo, useState } from "react";
import {
  cancelConnectionChange,
  confirmConnectionChange,
  getSession,
  isCommandError,
  onSessionChanged,
  reloadLogGroups,
  selectLogGroup,
  selectProfile,
  selectRegion,
  setFailureListOpen,
  startFetch,
  updateInput,
  updateLogGroupFilter,
  type InputField,
  type ProfileSelector,
  type SessionView,
} from "./api";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { ConnectionBar } from "./components/ConnectionBar";
import { FailureList } from "./components/FailureList";
import { FetchForm } from "./components/FetchForm";
import { LogGroupPane } from "./components/LogGroupPane";
import { LogTable } from "./components/LogTable";
import { StatusLine } from "./components/StatusLine";
import { useEscapeKey } from "./hooks/useEscapeKey";
import { createTranslator, type Locale } from "./i18n/messages";

export interface AppProps {
  locale: Locale;
}

/**
 * The single window. It holds no state of its own beyond mirroring the
 * session view the core pushes (ADR-001); the log rows are read from the
 * core by the table itself (U3:BR4.3). Operations that depend on the
 * connection carry the connection generation of the last view (U3:BR6.7).
 */
export function App({ locale }: AppProps) {
  const t = useMemo(() => createTranslator(locale), [locale]);
  const [session, setSession] = useState<SessionView | null>(null);
  const [commandError, setCommandError] = useState<string | null>(null);

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
    let unlisten: (() => void) | null = null;
    onSessionChanged(setSession).then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    }, reportError);
    getSession().then((view) => {
      if (!disposed) {
        setSession(view);
      }
    }, reportError);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [reportError]);

  const dismissError = useCallback(() => setCommandError(null), []);
  useEscapeKey(dismissError, commandError !== null);

  const handleChange = useCallback(
    (field: InputField, value: string) => {
      updateInput(field, value).then(setSession, reportError);
    },
    [reportError],
  );

  // These commands return nothing; their result arrives as a
  // `session-changed` event, so a late response never overwrites newer state.
  const run = useCallback(
    (command: () => Promise<void>) => {
      command().catch(reportError);
    },
    [reportError],
  );
  const generation = session?.connectionGeneration ?? 0;
  const handleSelectProfile = useCallback(
    (profile: ProfileSelector) => run(() => selectProfile(profile)),
    [run],
  );
  const handleSelectRegion = useCallback(
    (region: string) => run(() => selectRegion(region)),
    [run],
  );
  const handleFilter = useCallback((text: string) => run(() => updateLogGroupFilter(text)), [run]);
  const handleReload = useCallback(() => run(() => reloadLogGroups(generation)), [run, generation]);
  const handleSelectLogGroup = useCallback(
    (name: string) => run(() => selectLogGroup(name, generation)),
    [run, generation],
  );
  const handleConfirm = useCallback(() => run(confirmConnectionChange), [run]);
  const handleCancel = useCallback(() => run(cancelConnectionChange), [run]);
  const handleOpenFailures = useCallback(() => run(() => setFailureListOpen(true)), [run]);
  const handleCloseFailures = useCallback(() => run(() => setFailureListOpen(false)), [run]);

  const handleFetch = useCallback(() => {
    setCommandError(null);
    // The new state arrives through events only (see `startFetch`).
    startFetch(generation).catch(reportError);
  }, [reportError, generation]);

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
          <StatusLine session={session} t={t} onOpenFailures={handleOpenFailures} />
          {session?.failureListOpen && (
            <FailureList
              failedStreams={session.failedStreams}
              listingFailure={session.listingFailure}
              t={t}
              onClose={handleCloseFailures}
            />
          )}
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
          <LogTable
            totalCount={session?.eventCount ?? 0}
            timelineVersion={session?.timelineVersion ?? 0}
            t={t}
            onError={reportError}
          />
        </section>
      </div>
      {session?.pendingChange && (
        <ConfirmDialog t={t} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
    </main>
  );
}
