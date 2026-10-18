import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import Alert from "@cloudscape-design/components/alert";
import Box from "@cloudscape-design/components/box";
import Button from "@cloudscape-design/components/button";
import Container from "@cloudscape-design/components/container";
import DatePicker from "@cloudscape-design/components/date-picker";
import FormField from "@cloudscape-design/components/form-field";
import Header from "@cloudscape-design/components/header";
import SpaceBetween from "@cloudscape-design/components/space-between";
import StatusIndicator from "@cloudscape-design/components/status-indicator";
import TimeInput from "@cloudscape-design/components/time-input";
import { problemText } from "../analytics";
import type { AnalyticsField, InsightsView, SessionView } from "../api";
import {
  completeTime,
  fieldReasons,
  isMaskCompletionOf,
  isValidDateText,
  isoToSlashed,
  joinDateTime,
  slashedToIso,
  splitDateTime,
  type DateTimeParts,
  type Side,
} from "../dateTime";
import { errorLines } from "../errorText";
import { timeZoneLabelKey, type Locale, type Translate } from "../i18n/messages";
import { ElapsedTime } from "./ElapsedTime";
import { FailureList } from "./FailureList";
import { LogGroupMultiselect } from "./LogGroupMultiselect";
import { QueryEditor } from "./QueryEditor";
import { QueryResultTable } from "./QueryResultTable";

export interface LogAnalyticsPageProps {
  session: SessionView;
  t: Translate;
  locale?: Locale;
  /** Whether the shortcut is Cmd (macOS) rather than Ctrl. */
  mac: boolean;
  onChange: (field: AnalyticsField, value: string) => void;
  onRun: () => void;
  onCancel: () => void;
  onError: (error: unknown) => void;
}

const SIDES: Record<Side, { field: AnalyticsField; label: string; date: string; time: string }> = {
  start: {
    field: "startText",
    label: "form.start.label",
    date: "form.start.date",
    time: "form.start.time",
  },
  end: { field: "endText", label: "form.end.label", date: "form.end.date", time: "form.end.time" },
};

const SIDE_ORDER: readonly Side[] = ["start", "end"];

/** Whether the core holds the texts the fields show. */
function synced(insights: InsightsView, drafts: Record<Side, DateTimeParts>, query: string) {
  return (
    insights.startInput.text === joinDateTime(drafts.start.date, drafts.start.time) &&
    insights.endInput.text === joinDateTime(drafts.end.date, drafts.end.time) &&
    insights.queryText === query
  );
}

/**
 * The Log Analytics screen (261017-insights-query U1, refined-mockups screens
 * 2-7): one container with, from the top, the log group (U1:BR1.3), the start
 * and the end (the Log Management fields' rules, U1:BR1.4), the query field
 * and the run row with [Run query], the shortcut and, while running,
 * [Cancel]; then the results container. The inputs are the core's, held
 * apart from Log Management's (U1:BR1.2); the date, time and query fields
 * keep drafts so typing stays quick, and the page is remounted when the
 * core's inputs must be taken again (a time zone switch).
 *
 * [Run query] can be pressed when the core says so (U1:BR1.5) and the first
 * reason why not is shown beside it, except while running. A short time is
 * completed before a run, as on Log Management, and the run starts once the
 * core holds what the fields show. While running, the fields cannot be
 * changed and the query can only be read (U1:BR2.4). The results container
 * shows each state in words (U1:BR2.4-BR2.7, BR6.1): no results yet, the
 * fetch's and the evaluation's progress, no matching events, the rows, a
 * warning with the failed streams, the fetch's error, the cancel. A reading
 * problem is written under the query field with its line and column, and
 * the field takes the focus with the cursor there (U1:BR3.9); the previous
 * result stays meanwhile.
 */
export function LogAnalyticsPage({
  session,
  t,
  locale = "en",
  mac,
  onChange,
  onRun,
  onCancel,
  onError,
}: LogAnalyticsPageProps) {
  const insights = session.insights;
  const [drafts, setDrafts] = useState<Record<Side, DateTimeParts>>(() => ({
    start: splitDateTime(insights.startInput.text),
    end: splitDateTime(insights.endInput.text),
  }));
  const [query, setQuery] = useState(insights.queryText);
  const [failuresOpen, setFailuresOpen] = useState(false);
  const [focusRequest, setFocusRequest] = useState(0);
  const draftsRef = useRef(drafts);
  const queryRef = useRef(query);
  const pendingRunRef = useRef(false);
  const shownProblemRun = useRef<number | null>(null);

  const running = insights.phase === "Fetching" || insights.phase === "Evaluating";
  const dialogOpen = session.pendingChange !== null || session.settingsDialog === "Open";
  const locked = running || dialogOpen;
  const zone = t(timeZoneLabelKey(session.timeZone));
  const reasons = fieldReasons({
    start: drafts.start,
    end: drafts.end,
    validationErrors: insights.rangeErrors,
  });

  const commit = (side: Side, parts: DateTimeParts) => {
    draftsRef.current = { ...draftsRef.current, [side]: parts };
    setDrafts(draftsRef.current);
    onChange(SIDES[side].field, joinDateTime(parts.date, parts.time));
  };

  const handleTime = (side: Side, value: string) => {
    const previous = draftsRef.current[side].time;
    const time = isMaskCompletionOf(previous, value) ? completeTime(previous, side) : value;
    commit(side, { ...draftsRef.current[side], time });
  };

  const completeSide = (side: Side): boolean => {
    const parts = draftsRef.current[side];
    const time = isValidDateText(parts.date) ? completeTime(parts.time, side) : parts.time;
    if (time === parts.time) {
      return false;
    }
    commit(side, { ...parts, time });
    return true;
  };

  const handleQuery = (value: string) => {
    queryRef.current = value;
    setQuery(value);
    onChange("queryText", value);
  };

  /** [Run query] or Cmd/Ctrl+Enter (U1:BR1.6): completes the times first. */
  const run = () => {
    if (locked) {
      return;
    }
    const changed = SIDE_ORDER.map(completeSide).some(Boolean);
    if (!changed && synced(insights, draftsRef.current, queryRef.current)) {
      if (insights.canRun) {
        onRun();
      }
      return;
    }
    pendingRunRef.current = true;
  };

  useEffect(() => {
    if (!pendingRunRef.current || !synced(insights, draftsRef.current, queryRef.current)) {
      return;
    }
    pendingRunRef.current = false;
    if (insights.canRun && !locked) {
      onRun();
    }
  }, [insights, locked, onRun]);

  // U1:BR3.9: a new reading problem brings the focus to the query field.
  useEffect(() => {
    if (insights.phase === "QueryError" && shownProblemRun.current !== insights.runId) {
      shownProblemRun.current = insights.runId;
      setFocusRequest((count) => count + 1);
    }
  }, [insights.phase, insights.runId]);

  const handleFieldKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Enter" && event.target instanceof HTMLInputElement) {
      event.preventDefault();
      run();
    }
  };

  const problem = insights.phase === "QueryError" ? insights.problem : null;

  return (
    <div className="log-analytics-page" data-testid="log-analytics-page">
      <SpaceBetween size="m">
        <Container
          header={<Header variant="h2">{t("insights.title")}</Header>}
          data-testid="log-analytics-form"
        >
          <SpaceBetween size="s">
            <FormField label={t("insights.logGroups.label")} data-testid="log-analytics-log-group">
              <LogGroupMultiselect
                options={insights.logGroupOptions}
                selected={insights.selectedLogGroupName}
                status={insights.logGroupStatus}
                disabled={locked}
                t={t}
                onChange={(name) => onChange("logGroup", name ?? "")}
              />
            </FormField>
            <div className="condition-row-times" onKeyDown={handleFieldKeyDown}>
              {SIDE_ORDER.map((side) => {
                const spec = SIDES[side];
                const label = t(spec.label, { zone });
                const dateReason = reasons[`${side}Date`];
                const timeReason = reasons[`${side}Time`];
                return (
                  <div
                    key={side}
                    role="group"
                    aria-label={label}
                    data-testid={`log-analytics-${side}`}
                  >
                    <Box variant="awsui-key-label">{label}</Box>
                    <div className="condition-date-time">
                      <div className="condition-date">
                        <FormField errorText={dateReason && t(dateReason, { zone })}>
                          <DatePicker
                            value={slashedToIso(drafts[side].date)}
                            onChange={({ detail }) =>
                              commit(side, {
                                ...draftsRef.current[side],
                                date: isoToSlashed(detail.value),
                              })
                            }
                            placeholder={t("form.date.placeholder")}
                            ariaLabel={t(spec.date)}
                            locale={locale}
                            disabled={locked}
                            expandToViewport
                            data-testid={`log-analytics-${side}-date`}
                          />
                        </FormField>
                      </div>
                      <div className="condition-time">
                        <FormField errorText={timeReason && t(timeReason, { zone })}>
                          <TimeInput
                            value={drafts[side].time}
                            format="hh:mm:ss"
                            use24Hour
                            placeholder={t("form.time.placeholder")}
                            ariaLabel={t(spec.time)}
                            disabled={locked}
                            onChange={({ detail }) => handleTime(side, detail.value)}
                            onBlur={() => completeSide(side)}
                            data-testid={`log-analytics-${side}-time`}
                          />
                        </FormField>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
            <FormField label={t("insights.query.label")} stretch>
              <QueryEditor
                value={query}
                readOnly={locked}
                problem={problem}
                focusRequest={focusRequest}
                mac={mac}
                t={t}
                onChange={handleQuery}
                onRun={run}
              />
            </FormField>
            {problem !== null && (
              <div role="alert" aria-live="assertive" data-testid="query-problem">
                <StatusIndicator type="error" iconAriaLabel={t("failure.kind.Other")}>
                  {problemText(problem, t)}
                </StatusIndicator>
              </div>
            )}
            <RunBar
              insights={insights}
              locked={locked}
              mac={mac}
              t={t}
              onRun={run}
              onCancel={onCancel}
            />
          </SpaceBetween>
        </Container>
        <ResultArea
          session={session}
          t={t}
          onError={onError}
          onShowFailures={() => setFailuresOpen(true)}
        />
        <Box variant="small" data-testid="log-analytics-status">
          <ElapsedTime elapsed={insights.elapsed} t={t} />
        </Box>
      </SpaceBetween>
      {failuresOpen && (
        <FailureList
          failedStreams={insights.failedStreams}
          listingFailure={insights.listingFailure}
          t={t}
          onClose={() => setFailuresOpen(false)}
        />
      )}
    </div>
  );
}

interface RunBarProps {
  insights: InsightsView;
  locked: boolean;
  mac: boolean;
  t: Translate;
  onRun: () => void;
  onCancel: () => void;
}

/** The run row (interaction-spec RunBar, U1:BR1.5, BR2.4). */
function RunBar({ insights, locked, mac, t, onRun, onCancel }: RunBarProps) {
  const running = insights.phase === "Fetching" || insights.phase === "Evaluating";
  return (
    <div className="log-analytics-run" data-testid="log-analytics-run">
      <SpaceBetween direction="horizontal" size="s" alignItems="center">
        <Button
          variant="primary"
          disabled={locked || !insights.canRun}
          onClick={onRun}
          data-testid="log-analytics-run-button"
        >
          {t("insights.run")}
        </Button>
        {running && (
          <Button
            disabled={!insights.cancelAvailable && !insights.cancelling}
            loading={insights.cancelling}
            loadingText={t("insights.cancelling")}
            onClick={() => {
              if (insights.cancelAvailable) {
                onCancel();
              }
            }}
            data-testid="log-analytics-cancel-button"
          >
            {insights.cancelling ? t("insights.cancelling") : t("insights.cancel")}
          </Button>
        )}
        <Box variant="small" color="text-body-secondary">
          {t(mac ? "insights.shortcut.mac" : "insights.shortcut.other")}
        </Box>
        {!running && insights.runBlocker !== null && (
          <span data-testid="log-analytics-run-reason">
            <StatusIndicator type="info">{t(insights.runBlocker)}</StatusIndicator>
          </span>
        )}
      </SpaceBetween>
    </div>
  );
}

interface ResultAreaProps {
  session: SessionView;
  t: Translate;
  onError: (error: unknown) => void;
  onShowFailures: () => void;
}

/** The notices above the results and the results container (U1:BR2.4-BR2.7, BR6.1). */
function ResultArea({ session, t, onError, onShowFailures }: ResultAreaProps) {
  const insights = session.insights;
  const result = insights.result;
  const failedCount = insights.failedStreams.length;
  const showsRows = result !== null && insights.phase !== "Fetching";
  return (
    <SpaceBetween size="s">
      {insights.phase === "FetchFailed" && insights.failure !== null && (
        <FetchFailedAlert insights={insights} t={t} />
      )}
      {insights.phase === "Done" && insights.partial && (
        <div role="status" data-testid="log-analytics-partial">
          <Alert
            type="warning"
            statusIconAriaLabel={t("failure.kind.Other")}
            action={
              <Button onClick={onShowFailures} data-testid="log-analytics-show-failures-button">
                {t("insights.partial.show")}
              </Button>
            }
          >
            {failedCount > 0
              ? t("insights.partial", { count: failedCount })
              : t("insights.partial.listing")}
          </Alert>
        </div>
      )}
      {insights.phase === "Cancelled" && (
        <div role="status" data-testid="log-analytics-cancelled">
          <Alert type="info" statusIconAriaLabel={t("insights.cancel")}>
            {t("insights.cancelled")}
          </Alert>
        </div>
      )}
      <Container
        header={
          <Header
            variant="h2"
            counter={
              showsRows && result !== null
                ? `(${t("insights.results.matched", { count: result.rowCount })})`
                : undefined
            }
          >
            {t("insights.results.title")}
          </Header>
        }
        data-testid="log-analytics-results"
      >
        <ResultBody session={session} t={t} onError={onError} showsRows={showsRows} />
      </Container>
    </SpaceBetween>
  );
}

function ResultBody({
  session,
  t,
  onError,
  showsRows,
}: {
  session: SessionView;
  t: Translate;
  onError: (error: unknown) => void;
  showsRows: boolean;
}) {
  const insights = session.insights;
  const progress = insights.progress;
  if (insights.phase === "Fetching") {
    const text =
      progress === null || progress.plannedStreamCount === null
        ? t("insights.status.listing", { count: progress?.selectedStreamCount ?? 0 })
        : t("insights.status.fetching", {
            done: progress.finishedStreamCount,
            total: progress.plannedStreamCount,
            events: progress.eventCount,
          });
    return (
      <div role="status" data-testid="log-analytics-progress">
        <StatusIndicator type="loading">{text}</StatusIndicator>
      </div>
    );
  }
  if (insights.phase === "Evaluating") {
    return (
      <div role="status" data-testid="log-analytics-progress">
        <StatusIndicator type="loading">
          {t("insights.status.running", {
            scanned: progress?.evaluatedCount ?? 0,
            total: progress?.totalCount ?? 0,
          })}
        </StatusIndicator>
      </div>
    );
  }
  const result = insights.result;
  if (showsRows && result !== null) {
    if (result.rowCount === 0) {
      return <Box data-testid="log-analytics-no-match">{t("insights.results.none")}</Box>;
    }
    return <QueryResultTable result={result} timeZone={session.timeZone} t={t} onError={onError} />;
  }
  if (insights.phase === "Cancelled" || insights.phase === "FetchFailed") {
    return null;
  }
  return <Box data-testid="log-analytics-initial">{t("insights.results.initial")}</Box>;
}

/** U1:BR2.6: the fetch's error in the Log Management words, and that nothing was run. */
function FetchFailedAlert({ insights, t }: { insights: InsightsView; t: Translate }) {
  const failure = insights.failure;
  if (failure === null) {
    return null;
  }
  const lines = errorLines(failure.kind, "Fetch", failure.safeDetail, t);
  return (
    <div role="alert" data-testid="log-analytics-fetch-error">
      <Alert
        type="error"
        statusIconAriaLabel={t("failure.kind.Other")}
        header={<span data-testid="log-analytics-fetch-error-what">{lines.what}</span>}
      >
        <div>{lines.next}</div>
        {lines.detail !== null && <div>{lines.detail}</div>}
        <div>{t("insights.notRun")}</div>
      </Alert>
    </div>
  );
}
