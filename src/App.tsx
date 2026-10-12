import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type RefObject,
} from "react";
import Alert from "@cloudscape-design/components/alert";
import Button, { type ButtonProps } from "@cloudscape-design/components/button";
import {
  cancelClose,
  cancelConnectionChange,
  cancelSettings,
  cancelSsoLogin,
  clearCache,
  confirmClose,
  confirmConnectionChange,
  dismissSsoLogin,
  getSession,
  isCommandError,
  isSsoLoginModalShown,
  isSsoLoginRunning,
  mergeSessionView,
  onFetchProgress,
  onFilterProgress,
  onSessionChanged,
  openSettings,
  openSsoUrl,
  reloadLogGroups,
  saveSettings,
  selectLogGroup,
  selectProfile,
  selectRegion,
  selectTimeZone,
  setFailureListOpen,
  setLogFilter,
  startFetch,
  startSsoLogin,
  updateInput,
  updateLogGroupFilter,
  withFilter,
  withProgress,
  type InputField,
  type ProfileSelector,
  type SessionView,
  type TimeZoneChoice,
} from "./api";
import { CloseConfirmDialog } from "./components/CloseConfirmDialog";
import { ConditionArea } from "./components/ConditionArea";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { FailureList } from "./components/FailureList";
import { FetchErrorBanner } from "./components/FetchErrorBanner";
import { LogGroupPane } from "./components/LogGroupPane";
import { LogTable } from "./components/LogTable";
import { LogTableCloudscape } from "./components/LogTableCloudscape";
import { PaneResizer } from "./components/PaneResizer";
import { SettingsDialog } from "./components/SettingsDialog";
import { SsoLoginModal } from "./components/SsoLoginModal";
import { StatusLine } from "./components/StatusLine";
import { TopBar } from "./components/TopBar";
import { LOG_TABLE_VARIANT, usePerfRows } from "./dev/perfRows";
import { useColorScheme } from "./hooks/useColorScheme";
import { useEscapeKey } from "./hooks/useEscapeKey";
import { usePaneWidth } from "./hooks/usePaneWidth";
import { createTranslator, type Locale } from "./i18n/messages";

export interface AppProps {
  locale: Locale;
}

/**
 * The single window, laid out like the console's log events page (mockups
 * screen 1, FR4.2): the black top navigation, the log group pane on the
 * left and, on the right, one container with the conditions, the error
 * area, the log list and the status line. It holds no state of its own
 * beyond mirroring the session view the core pushes (ADR-001); the log rows
 * are read from the core by the table itself (U3:BR4.3). Operations that
 * depend on the connection carry the connection generation of the last view
 * (U3:BR6.7). Since U5 it keeps the log filter field's text (so a remount of
 * the form loses nothing), forwards the text after the typing pause, and
 * shows the filter result: the table counts the matching rows while a
 * filter is in force. Filter messages older than the shown result are
 * ignored (U5:BR3.6). Since U6 it opens the settings dialog from the gear
 * button and returns the focus to it when the dialog closes (U6:BR5.1,
 * BR5.2), and counts refused log filter hand-overs so the field can send its
 * text again (U5 review R-02). Since U7 it shows the failure of a whole
 * fetch in words above the list (U7:BR2.4), hands the list the discard
 * generation (U7:BR1.6) and shows the close confirmation while the session
 * says it is pending, forwarding the answer (U7:BR3.1-BR3.3). The Tab order
 * is the order of the screen: profile, region, time zone, settings, log
 * group search, log group list, [Reload], [Fetch], log filter, start date,
 * start time, end date, end time, then the log list (mockups screen 1).
 * The OS light or dark mode is followed with Cloudscape's theme (FR4.5).
 * The width of the log group pane is the user's, dragged or keyed on the
 * handle between the panes and remembered in the browser (gate fix 14d), as
 * a share of the app body (gate fix 15b); the pane can also be closed from
 * its heading, leaving only a slim strip with the button that opens it again
 * (the closed state is remembered too). Closing moves the focus to that
 * button, opening moves it back to the close button, so the keyboard never
 * loses its place.
 * Since 261011-sso-login [Sign in with SSO] in the list error and the fetch
 * error starts a login; the login dialog is shown while the core's
 * `ssoLogin` state asks for it, a success shows a dismissible notice at the
 * top of the right pane, and when the dialog closes the focus goes back to
 * the button that was pressed, or to [Reload] when that button is gone
 * (NFR7).
 */
export function App({ locale }: AppProps) {
  const t = useMemo(() => createTranslator(locale), [locale]);
  const [session, setSession] = useState<SessionView | null>(null);
  const [commandError, setCommandError] = useState<string | null>(null);
  const [filterDraft, setFilterDraft] = useState<string | null>(null);
  const [filterFailures, setFilterFailures] = useState(0);
  const settingsButtonRef = useRef<ButtonProps.Ref>(null);
  const perf = usePerfRows();
  const pane = usePaneWidth();
  const collapseButtonRef = useRef<ButtonProps.Ref>(null);
  const expandButtonRef = useRef<ButtonProps.Ref>(null);
  const reloadButtonRef = useRef<ButtonProps.Ref>(null);
  // The [Sign in with SSO] button last pressed, for the focus afterwards.
  const ssoTriggerRef = useRef<RefObject<ButtonProps.Ref | null> | null>(null);
  const ssoModalWasShown = useRef(false);
  // Which pane button takes the focus after the next collapse or expand.
  const paneFocusRef = useRef<"collapse" | "expand" | null>(null);
  useColorScheme();

  const { setCollapsed: setPaneCollapsed } = pane;
  const collapsePane = useCallback(() => {
    paneFocusRef.current = "expand";
    setPaneCollapsed(true);
  }, [setPaneCollapsed]);
  const expandPane = useCallback(() => {
    paneFocusRef.current = "collapse";
    setPaneCollapsed(false);
  }, [setPaneCollapsed]);
  useEffect(() => {
    const target = paneFocusRef.current;
    paneFocusRef.current = null;
    if (target === "expand") {
      expandButtonRef.current?.focus();
    } else if (target === "collapse") {
      collapseButtonRef.current?.focus();
    }
  }, [pane.collapsed]);

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
  const handleKeepFetching = useCallback(() => run(cancelClose), [run]);
  const handleCloseApp = useCallback(() => run(confirmClose), [run]);
  const handleOpenFailures = useCallback(() => run(() => setFailureListOpen(true)), [run]);
  const handleCloseFailures = useCallback(() => run(() => setFailureListOpen(false)), [run]);
  const handleSsoLogin = useCallback(
    (returnFocus: RefObject<ButtonProps.Ref | null>) => {
      ssoTriggerRef.current = returnFocus;
      run(startSsoLogin);
    },
    [run],
  );
  const handleCancelSsoLogin = useCallback(() => run(cancelSsoLogin), [run]);
  const handleDismissSsoLogin = useCallback(() => run(dismissSsoLogin), [run]);
  const handleOpenSsoUrl = useCallback((url: string) => run(() => openSsoUrl(url)), [run]);

  // NFR7: when the login dialog closes, the focus goes back to the pressed
  // button, or to [Reload] when that button is gone.
  const ssoModalShown = session !== null && isSsoLoginModalShown(session.ssoLogin);
  useEffect(() => {
    if (ssoModalWasShown.current && !ssoModalShown) {
      (ssoTriggerRef.current?.current ?? reloadButtonRef.current)?.focus();
    }
    ssoModalWasShown.current = ssoModalShown;
  }, [ssoModalShown]);

  const handleFetch = useCallback(() => {
    setCommandError(null);
    // The new state arrives through events only (see `startFetch`).
    startFetch(generation).catch(reportError);
  }, [reportError, generation]);

  // The list: the core's rows, or the measurement harness's (dev only).
  const tableProps = {
    totalCount: perf.enabled
      ? perf.total
      : (session?.filterSummary?.matchedCount ?? session?.eventCount ?? 0),
    timelineVersion: perf.enabled ? perf.version : (session?.timelineVersion ?? 0),
    timeZone: session?.timeZone ?? "Local",
    filterId: session?.filterSummary?.filterId ?? null,
    resultVersion: perf.enabled ? perf.version : (session?.filterResultVersion ?? 0),
    discardGeneration: session?.discardGeneration ?? 0,
    t,
    onError: reportError,
    emptyText: session?.filterSummary ? t("table.noMatches") : t("table.empty"),
  };

  return (
    <main className="app" data-testid="app">
      {session && (
        <TopBar
          session={session}
          t={t}
          onSelectProfile={handleSelectProfile}
          onSelectRegion={handleSelectRegion}
          onSelectTimeZone={handleSelectTimeZone}
          onOpenSettings={handleOpenSettings}
          settingsButtonRef={settingsButtonRef}
        />
      )}
      <div
        className="app-body"
        style={{ "--ls-log-group-pane-width": `${pane.width}%` } as CSSProperties}
      >
        {session && pane.collapsed && (
          <div className="log-group-pane-collapsed" data-testid="log-group-pane-collapsed">
            <Button
              ref={expandButtonRef}
              variant="icon"
              iconName="angle-right"
              ariaLabel={t("pane.expand")}
              onClick={expandPane}
              data-testid="log-group-pane-expand-button"
            />
          </div>
        )}
        {session && !pane.collapsed && (
          <LogGroupPane
            key={session.sessionId}
            session={session}
            t={t}
            onFilterChange={handleFilter}
            onReload={handleReload}
            onSelect={handleSelectLogGroup}
            onCollapse={collapsePane}
            collapseButtonRef={collapseButtonRef}
            reloadButtonRef={reloadButtonRef}
            onSsoLogin={handleSsoLogin}
          />
        )}
        {session && !pane.collapsed && (
          <PaneResizer value={pane.width} onChange={pane.setWidth} t={t} />
        )}
        <section className="app-main">
          {session && (
            <ConditionArea
              key={`${session.sessionId}:${session.timeZone}`}
              session={session}
              t={t}
              locale={locale}
              onChange={handleChange}
              onFetch={handleFetch}
              filterText={filterDraft ?? session.logFilter}
              onFilterTextChange={setFilterDraft}
              onLogFilterChange={handleLogFilter}
              filterFailureCount={filterFailures}
            >
              {session.ssoLogin.state === "Succeeded" && (
                <div role="status" data-testid="sso-login-success">
                  <Alert
                    type="success"
                    dismissible
                    dismissAriaLabel={t("ssoLogin.close")}
                    statusIconAriaLabel={t("ssoLogin.succeeded")}
                    onDismiss={handleDismissSsoLogin}
                  >
                    {t("ssoLogin.succeeded")}
                  </Alert>
                </div>
              )}
              {commandError && (
                <div role="alert" data-testid="command-error">
                  <Alert
                    type="error"
                    dismissible
                    dismissAriaLabel={t("error.dismiss")}
                    statusIconAriaLabel={t("failure.kind.Other")}
                    onDismiss={dismissError}
                  >
                    {commandError}
                  </Alert>
                </div>
              )}
              <FetchErrorBanner session={session} t={t} onSsoLogin={handleSsoLogin} />
              {LOG_TABLE_VARIANT === "cloudscape" ? (
                <LogTableCloudscape {...tableProps} />
              ) : (
                <LogTable {...tableProps} />
              )}
              <StatusLine session={session} t={t} onOpenFailures={handleOpenFailures} />
            </ConditionArea>
          )}
          {session?.failureListOpen && (
            <FailureList
              failedStreams={session.failedStreams}
              listingFailure={session.listingFailure}
              t={t}
              onClose={handleCloseFailures}
            />
          )}
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
      {session?.closeConfirmation === "Pending" && (
        <CloseConfirmDialog t={t} onKeep={handleKeepFetching} onClose={handleCloseApp} />
      )}
      {session && ssoModalShown && (
        <SsoLoginModal
          // A new attempt, or waiting turning into a result, starts the
          // dialog afresh so its footer button takes the focus.
          key={`${session.ssoLogin.attemptId}:${isSsoLoginRunning(session.ssoLogin)}`}
          login={session.ssoLogin}
          t={t}
          onCancel={handleCancelSsoLogin}
          onClose={handleDismissSsoLogin}
          onOpenUrl={handleOpenSsoUrl}
        />
      )}
    </main>
  );
}
