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
import { I18nProvider, type I18nProviderProps } from "@cloudscape-design/components/i18n";
import enMessages from "@cloudscape-design/components/i18n/messages/all.en";
import jaMessages from "@cloudscape-design/components/i18n/messages/all.ja";
import {
  cancelClose,
  cancelConnectionChange,
  cancelFetch,
  cancelQuery,
  cancelSettings,
  cancelSsoLogin,
  clearCache,
  confirmClose,
  confirmConnectionChange,
  dismissExportNotice,
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
  runQuery,
  saveSettings,
  selectLogGroup,
  selectProfile,
  selectRegion,
  selectTimeZone,
  setFailureListOpen,
  setLogFilter,
  startExport,
  startFetch,
  startSsoLogin,
  updateAnalyticsInput,
  updateInput,
  updateLogGroupFilter,
  withFilter,
  withProgress,
  type AnalyticsField,
  type ExportFormat,
  type ExportScreen,
  type ExportTarget,
  type InputField,
  type ProfileSelector,
  type SessionView,
  type TimeZoneChoice,
} from "./api";
import { APP_PAGES, PAGE_NAME_KEY, type AppPage } from "./analytics";
import { AppNavigation } from "./components/AppNavigation";
import { CancelNotice } from "./components/CancelNotice";
import { CloseConfirmDialog } from "./components/CloseConfirmDialog";
import { ConditionArea } from "./components/ConditionArea";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { ExportNoticeAlert } from "./components/ExportNoticeAlert";
import { FailureList } from "./components/FailureList";
import { FetchErrorBanner } from "./components/FetchErrorBanner";
import { LogAnalyticsPage } from "./components/LogAnalyticsPage";
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
import {
  createTranslator,
  resolveLocale,
  type LanguagePreference,
  type Locale,
} from "./i18n/messages";
import { currentPlatform, isMacPlatform, isShortcutModifier } from "./platform";

export interface AppProps {
  /**
   * The OS/browser languages in order of preference, read when the saved
   * language preference is "system" (261022-small-features U1:BR1.2); this
   * browser's when absent.
   */
  languages?: readonly string[];
  /** The platform string the shortcuts follow; this browser's when absent. */
  platform?: string;
}

/** The OS/browser languages, `navigator.language` alone when the list is empty. */
function browserLanguages(): readonly string[] {
  return navigator.languages.length > 0 ? navigator.languages : [navigator.language];
}

// FR4.6: Cloudscape's own texts (calendar month names, "Close modal", ...)
// follow the same locale as the app's catalog.
const cloudscapeMessages: Record<Locale, I18nProviderProps.Messages> = {
  en: enMessages,
  ja: jaMessages,
};

/** The language preference to show before the session view arrived (U1 W1). */
const PREFERENCE_BEFORE_SESSION: LanguagePreference = "system";

/**
 * A refused command, kept as its message key and placeholders so the text
 * follows a language switch and the error handler never changes with the
 * locale (261022-small-features U1:BR1.2; a changing handler would make the
 * subscription effect run again).
 */
interface CommandErrorText {
  key: string;
  params?: Record<string, string>;
}

/** Where the focus goes when a page is shown (interaction-spec SideNavigation). */
const FIRST_FIELD: Record<AppPage, readonly string[]> = {
  logManagement: [
    '[data-testid="log-group-pane-filter"] input',
    '[data-testid="log-group-pane-expand-button"]',
  ],
  logAnalytics: ['[data-testid="log-group-multiselect"] button'],
};

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
 * Since 261012-parallel-fetch U5 the [Fetch] button becomes [Cancel] while
 * fetching, which calls `cancelFetch`, and the cancel notice of the last
 * fetch is shown where the fetch error would be (BR5.11).
 * Since 261017-insights-query U1 a navigation at the left edge switches
 * between this screen, Log Management, and Log Analytics (U1:BR1.1): the app
 * opens on Log Management, whose contents and behaviour stay as they were;
 * Ctrl+1 / Ctrl+2 (Cmd on macOS) switch too. The Log Analytics page is made
 * the first time it is shown and then kept, hidden when the other page is
 * shown, so switching discards nothing on either (U1:BR1.2); fetching and
 * evaluating go on meanwhile. The focus moves to the first field of the page
 * shown and its name is announced. Whether the navigation is open is kept
 * until the app closes. The top bar and the dialogs are the same for both.
 * Since 261022-small-features U1 the locale is decided here, not by
 * `main.tsx`: from the session view's saved language preference and the OS
 * languages (`resolveLocale`, U1:BR1.2), the OS language alone until the
 * first view arrives (W1). The translator, Cloudscape's `I18nProvider`, the
 * region names and `document.documentElement.lang` all follow that one
 * value, so a saved preference switches every text without a restart and
 * without remounting the pages: fetched logs, opened rows, field texts, the
 * connection and the filters stay (U1:BR1.3, FR1.5). The settings dialog
 * hands both settings to `saveSettings`; a failed save leaves the view, and
 * so the language, as it was (U1:BR1.6).
 * Since 261022-small-features U2 both screens have an [Export] menu whose
 * choice goes to `startExport` (U2:BR4.1); a refused request is shown like
 * any other refused command. The notice of the last export is shown in the
 * alert area of the page shown, built from its kind and arguments so a
 * language switch rewrites it, and closing it calls `dismissExportNotice`
 * (U2:BR4.9).
 */
export function App({ languages, platform }: AppProps) {
  const [session, setSession] = useState<SessionView | null>(null);
  const osLanguages = useMemo(() => languages ?? browserLanguages(), [languages]);
  const preference = session?.language ?? PREFERENCE_BEFORE_SESSION;
  const locale = useMemo(() => resolveLocale(preference, osLanguages), [preference, osLanguages]);
  const t = useMemo(() => createTranslator(locale), [locale]);
  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);
  const mac = useMemo(() => isMacPlatform(platform ?? currentPlatform()), [platform]);
  const [page, setPage] = useState<AppPage>("logManagement");
  const [analyticsShown, setAnalyticsShown] = useState(false);
  const [navigationOpen, setNavigationOpen] = useState(true);
  const [announcement, setAnnouncement] = useState("");
  const [focusRequest, setFocusRequest] = useState(0);
  const [commandError, setCommandError] = useState<CommandErrorText | null>(null);
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

  const reportError = useCallback((error: unknown) => {
    setCommandError(
      isCommandError(error)
        ? { key: error.key }
        : { key: "error.command", params: { detail: String(error) } },
    );
  }, []);

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
  // 261022-small-features U1:BR1.1: both settings in one save.
  const handleSaveSettings = useCallback(
    (enabled: boolean, language: LanguagePreference) => run(() => saveSettings(enabled, language)),
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
  // Trying again from the result dialog keeps the first button as the
  // place the focus goes back to.
  const handleRetrySsoLogin = useCallback(() => run(startSsoLogin), [run]);
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
  // 261012-parallel-fetch U5:BR5.1: [Cancel]; the state arrives as an event.
  const handleCancelFetch = useCallback(() => run(cancelFetch), [run]);

  // 261017-insights-query U1: the Log Analytics page's operations.
  const handleAnalyticsChange = useCallback(
    (field: AnalyticsField, value: string | readonly string[]) =>
      run(() => updateAnalyticsInput(field, value, generation)),
    [run, generation],
  );
  const handleRunQuery = useCallback(() => {
    setCommandError(null);
    runQuery().catch(reportError);
  }, [reportError]);
  const handleCancelQuery = useCallback(() => run(cancelQuery), [run]);

  // 261022-small-features U2:BR4.1: [Export] of either screen; the Running
  // job, the end and the notice arrive as `session-changed` (BR4.8, BR4.9).
  const handleExport = useCallback(
    (screen: ExportScreen, format: ExportFormat, target: ExportTarget) =>
      run(() => startExport(screen, format, target)),
    [run],
  );
  const handleExportLogs = useCallback(
    (format: ExportFormat, target: ExportTarget) => handleExport("logManagement", format, target),
    [handleExport],
  );
  const handleExportResult = useCallback(
    (format: ExportFormat, target: ExportTarget) => handleExport("logAnalytics", format, target),
    [handleExport],
  );
  const handleDismissExportNotice = useCallback(() => run(dismissExportNotice), [run]);

  // U1:BR1.1: switching pages; the page shown takes the focus.
  const navigate = useCallback(
    (target: AppPage) => {
      if (target === "logAnalytics") {
        setAnalyticsShown(true);
      }
      setPage(target);
      setAnnouncement(t(PAGE_NAME_KEY[target]));
      setFocusRequest((count) => count + 1);
    },
    [t],
  );
  useEffect(() => {
    if (focusRequest === 0) {
      return;
    }
    const container = document.querySelector(`[data-testid="app-page-${page}"]`);
    for (const selector of FIRST_FIELD[page]) {
      const field = container?.querySelector<HTMLElement>(selector);
      if (field) {
        field.focus();
        return;
      }
    }
  }, [focusRequest, page]);
  useEffect(() => {
    const handleKey = (event: KeyboardEvent) => {
      const index = ["1", "2"].indexOf(event.key);
      const target = APP_PAGES[index];
      if (target !== undefined && isShortcutModifier(event, mac)) {
        event.preventDefault();
        navigate(target);
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [mac, navigate]);

  const commandErrorAlert = commandError && (
    <div role="alert" data-testid="command-error">
      <Alert
        type="error"
        dismissible
        dismissAriaLabel={t("error.dismiss")}
        statusIconAriaLabel={t("failure.kind.Other")}
        onDismiss={dismissError}
      >
        {t(commandError.key, commandError.params)}
      </Alert>
    </div>
  );

  // U2:BR4.9: the same notice on whichever page is shown.
  const exportNoticeAlert = session && (
    <ExportNoticeAlert
      notice={session.exportNotice}
      t={t}
      onDismiss={handleDismissExportNotice}
      // 261025-insights-stats U3:BR3.5: the query screen's export of a cancelled result.
      partialResult={
        session.export.request?.screen === "logAnalytics" &&
        session.insights.result?.stage === "cancelledPartial"
      }
    />
  );

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
    <I18nProvider locale={locale} messages={[cloudscapeMessages[locale]]}>
      <main className="app" data-testid="app">
        {session && (
          <TopBar
            session={session}
            t={t}
            locale={locale}
            onSelectProfile={handleSelectProfile}
            onSelectRegion={handleSelectRegion}
            onSelectTimeZone={handleSelectTimeZone}
            onOpenSettings={handleOpenSettings}
            settingsButtonRef={settingsButtonRef}
          />
        )}
        <div className="app-shell">
          {session && (
            <AppNavigation
              page={page}
              open={navigationOpen}
              t={t}
              onNavigate={navigate}
              onOpenChange={setNavigationOpen}
            />
          )}
          <div
            className="app-page"
            hidden={page !== "logManagement"}
            data-testid="app-page-logManagement"
          >
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
                    onCancel={handleCancelFetch}
                    filterText={filterDraft ?? session.logFilter}
                    onFilterTextChange={setFilterDraft}
                    onLogFilterChange={handleLogFilter}
                    filterFailureCount={filterFailures}
                    onExport={handleExportLogs}
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
                    {page === "logManagement" && exportNoticeAlert}
                    {page === "logManagement" && commandErrorAlert}
                    <FetchErrorBanner session={session} t={t} onSsoLogin={handleSsoLogin} />
                    {/* U5:BR5.11: in the error area's place; a cancelled fetch is
                  not a failure, so the two never show together. Its status
                  region is always there, empty without a notice
                  (261020-carryover-fixes FR4.2.1). */}
                    <CancelNotice notice={session.cancelNotice} t={t} />
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
          </div>
          {session && analyticsShown && (
            <div
              className="app-page app-page-analytics"
              hidden={page !== "logAnalytics"}
              data-testid="app-page-logAnalytics"
            >
              {page === "logAnalytics" && exportNoticeAlert}
              {page === "logAnalytics" && commandErrorAlert}
              <LogAnalyticsPage
                key={`${session.sessionId}:${session.timeZone}`}
                session={session}
                t={t}
                locale={locale}
                mac={mac}
                onChange={handleAnalyticsChange}
                onRun={handleRunQuery}
                onCancel={handleCancelQuery}
                onError={reportError}
                onExport={handleExportResult}
              />
            </div>
          )}
        </div>
        <div className="visually-hidden" aria-live="polite" data-testid="app-page-announcement">
          {announcement}
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
            onRetry={handleRetrySsoLogin}
            onOpenUrl={handleOpenSsoUrl}
          />
        )}
      </main>
    </I18nProvider>
  );
}
