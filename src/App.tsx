import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  cancelConnectionChange,
  cancelSettings,
  clearCache,
  confirmConnectionChange,
  getSession,
  isCommandError,
  mergeSessionView,
  onFetchProgress,
  onFilterProgress,
  onSessionChanged,
  openSettings,
  reloadLogGroups,
  saveSettings,
  selectLogGroup,
  selectProfile,
  selectRegion,
  selectTimeZone,
  setFailureListOpen,
  setLogFilter,
  startFetch,
  updateInput,
  updateLogGroupFilter,
  withFilter,
  withProgress,
  type InputField,
  type ProfileSelector,
  type SessionView,
  type TimeZoneChoice,
} from "./api";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { ConnectionBar } from "./components/ConnectionBar";
import { FailureList } from "./components/FailureList";
import { FetchForm } from "./components/FetchForm";
import { LogGroupPane } from "./components/LogGroupPane";
import { LogTable } from "./components/LogTable";
import { SettingsDialog } from "./components/SettingsDialog";
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
 * Since U5 it keeps the log filter field's text (so a remount of the form
 * loses nothing), forwards the text after the typing pause, and shows the
 * filter result: the table counts the matching rows while a filter is in
 * force. Filter messages older than the shown result are ignored (U5:BR3.6).
 * Since U6 it opens the settings dialog from the [*] button and returns the
 * focus to it when the dialog closes (U6:BR5.1, BR5.2), and counts refused
 * log filter hand-overs so the field can send its text again (U5 review R-02).
 */
export function App({ locale }: AppProps) {
  const t = useMemo(() => createTranslator(locale), [locale]);
  const [session, setSession] = useState<SessionView | null>(null);
  const [commandError, setCommandError] = useState<string | null>(null);
  const [filterDraft, setFilterDraft] = useState<string | null>(null);
  const [filterFailures, setFilterFailures] = useState(0);
  const settingsButtonRef = useRef<HTMLButtonElement>(null);

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
    const keep = (stop: () => void) => {
      if (disposed) {
        stop();
      } else {
        unlisteners.push(stop);
      }
    };
    onSessionChanged((view) => setSession((previous) => mergeSessionView(previous, view))).then(
      keep,
      reportError,
    );
    onFetchProgress((update) => setSession((view) => withProgress(view, update))).then(
      keep,
      reportError,
    );
    onFilterProgress((update) => setSession((view) => withFilter(view, update))).then(
      keep,
      reportError,
    );
    getSession().then((view) => {
      if (!disposed) {
        setSession(view);
      }
    }, reportError);
    return () => {
      disposed = true;
      unlisteners.forEach((stop) => stop());
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
  // U4:BR1.4: accepted at any time; the new view comes as `session-changed`.
  const handleSelectTimeZone = useCallback(
    (timeZone: TimeZoneChoice) => run(() => selectTimeZone(timeZone)),
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
  // U5:BR1.3: called once typing paused; the core trims and compares.
  const handleLogFilter = useCallback(
    (text: string) => {
      setLogFilter(text).catch((error: unknown) => {
        setFilterFailures((count) => count + 1);
        reportError(error);
      });
    },
    [reportError],
  );
  const handleOpenSettings = useCallback(() => run(openSettings), [run]);
  const handleCancelSettings = useCallback(() => run(cancelSettings), [run]);
  const handleSaveSettings = useCallback(
    (enabled: boolean) => run(() => saveSettings(enabled)),
    [run],
  );
  const handleClearCache = useCallback(() => run(clearCache), [run]);
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
          onSelectTimeZone={handleSelectTimeZone}
          onOpenSettings={handleOpenSettings}
          settingsButtonRef={settingsButtonRef}
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
              key={`${session.sessionId}:${session.timeZone}`}
              session={session}
              t={t}
              onChange={handleChange}
              onFetch={handleFetch}
              filterText={filterDraft ?? session.logFilter}
              onFilterTextChange={setFilterDraft}
              onLogFilterChange={handleLogFilter}
              filterFailureCount={filterFailures}
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
            totalCount={session?.filterSummary?.matchedCount ?? session?.eventCount ?? 0}
            timelineVersion={session?.timelineVersion ?? 0}
            timeZone={session?.timeZone ?? "Local"}
            filterId={session?.filterSummary?.filterId ?? null}
            resultVersion={session?.filterResultVersion ?? 0}
            t={t}
            onError={reportError}
          />
        </section>
      </div>
      {session?.pendingChange && (
        <ConfirmDialog t={t} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
      {session?.settingsDialog === "Open" && (
        <SettingsDialog
          session={session}
          t={t}
          returnFocusRef={settingsButtonRef}
          onSave={handleSaveSettings}
          onCancel={handleCancelSettings}
          onClear={handleClearCache}
        />
      )}
    </main>
  );
}
