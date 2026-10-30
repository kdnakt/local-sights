import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import Alert from "@cloudscape-design/components/alert";
import Box from "@cloudscape-design/components/box";
import Button from "@cloudscape-design/components/button";
import ButtonDropdown from "@cloudscape-design/components/button-dropdown";
import Container from "@cloudscape-design/components/container";
import DatePicker from "@cloudscape-design/components/date-picker";
import FormField from "@cloudscape-design/components/form-field";
import Header from "@cloudscape-design/components/header";
import Modal from "@cloudscape-design/components/modal";
import SpaceBetween from "@cloudscape-design/components/space-between";
import StatusIndicator from "@cloudscape-design/components/status-indicator";
import TimeInput from "@cloudscape-design/components/time-input";
import { QUERY_EXAMPLES, problemText, type QueryExample } from "../analytics";
import {
  getCompletionCandidates,
  type AnalyticsField,
  type CompletionCandidate,
  type ExportFormat,
  type ExportTarget,
  type InsightsView,
  type ResultSummary,
  type SessionView,
} from "../api";
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
import { ExportMenu } from "./ExportMenu";
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
  /** A field changed: the log groups as a list (U3:BR4.1), the others as a text. */
  onChange: (field: AnalyticsField, value: string | readonly string[]) => void;
  onRun: () => void;
  onCancel: () => void;
  onError: (error: unknown) => void;
  /** One of the four exports was chosen in the results' [Export] menu (U2:BR4.1). */
  onExport: (format: ExportFormat, target: ExportTarget) => void;
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
 * changed and the query can only be read (U1:BR2.4). The page shows each
 * state in words (U1:BR2.4-BR2.7, BR6.1): no results yet, the fetch's and
 * the evaluation's progress, no matching events, the rows, a warning with
 * the failed streams, the fetch's error, the cancel. A reading
 * problem is written under the query field with its line and column, and
 * the field takes the focus with the cursor there (U1:BR3.9); the previous
 * result stays meanwhile.
 *
 * Since 261017-insights-query U2 the problem is also underlined and marked
 * in the field, and its words, underline and mark go as soon as the text is
 * changed (U2:BR6.1). The results heading counts the rows shown, and when a
 * `limit` or the default cut dropped rows that matched it says how many
 * matched and "Showing N" (U2:BR5.3, BR7.2). An evaluation that ended
 * abnormally is said in its own words above the results, apart from a
 * failed fetch (U2:BR10.2). The query field completes the commands, words,
 * fixed fields and the JSON field names of the last fetch, asked of the
 * core after every change of the run (U2:BR9.1), and [Examples] puts an
 * example in the field, asking first when the field holds another query
 * (U2:BR9.3); both are Should.
 *
 * Since 261017-insights-query U3 up to 50 log groups are chosen (U3:BR4.1).
 * While a run fetches, the progress says which log group of how many is
 * fetched, its streams and events, how many log groups failed, and when the
 * cache is read (U3:BR2.7). The previous result stays shown until the new
 * one replaces it (U3:BR3.4). A failed fetch names every log group that
 * failed, and the failed streams are named "log group / stream" (U3:BR2.6).
 *
 * Since 261020-carryover-fixes FR4.4 the run's progress sits in the query
 * container just below the run row, since it comes from [Run query], and the
 * results container holds only the heading, the result (the previous one
 * while a run goes) and the words for no result; while a run goes with no
 * previous result the results container is empty (A9).
 *
 * Since 261022-small-features U2 the results heading carries the [Export]
 * menu (U2:BR4.1), disabled while the core says the result cannot be
 * exported (`insights.canExport`, U2:BR3.2) and busy while an export runs
 * (`session.export.status`, U2:BR4.8); a chosen item goes to `onExport`.
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
  onExport,
}: LogAnalyticsPageProps) {
  const insights = session.insights;
  const [drafts, setDrafts] = useState<Record<Side, DateTimeParts>>(() => ({
    start: splitDateTime(insights.startInput.text),
    end: splitDateTime(insights.endInput.text),
  }));
  const [query, setQuery] = useState(insights.queryText);
  const [failuresOpen, setFailuresOpen] = useState(false);
  const [focusRequest, setFocusRequest] = useState(0);
  // U2:BR6.1: the text a reading problem was found in; `null` once it was changed.
  const [problemQuery, setProblemQuery] = useState<string | null>(() =>
    insights.phase === "QueryError" ? insights.queryText : null,
  );
  const [candidates, setCandidates] = useState<readonly CompletionCandidate[]>([]);
  const [pendingExample, setPendingExample] = useState<QueryExample | null>(null);
  const draftsRef = useRef(drafts);
  const queryRef = useRef(query);
  const pendingRunRef = useRef(false);
  const shownProblemRun = useRef<number | null>(null);

  // U2:BR9.1: the core gathers the JSON field names when a fetch ends, so
  // the candidates are asked again whenever the run changes.
  useEffect(() => {
    let current = true;
    Promise.resolve()
      .then(getCompletionCandidates)
      .then(
        (list) => {
          if (current && Array.isArray(list)) {
            setCandidates(list);
          }
        },
        (error: unknown) => {
          if (current) {
            onError(error);
          }
        },
      );
    return () => {
      current = false;
    };
  }, [insights.runId, insights.phase, onError]);

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
    // U2:BR6.1: changing the text takes the problem's words and marks away
    // until the next run.
    setProblemQuery(null);
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
      setProblemQuery(queryRef.current);
      setFocusRequest((count) => count + 1);
    }
  }, [insights.phase, insights.runId]);

  /** U2:BR9.3: an example replaces the query at once when nothing else is written. */
  const chooseExample = (id: string) => {
    const example = QUERY_EXAMPLES.find((candidate) => candidate.id === id);
    if (example === undefined || locked) {
      return;
    }
    const written = queryRef.current.trim();
    if (written === "" || written === example.text) {
      handleQuery(example.text);
    } else {
      setPendingExample(example);
    }
  };

  const handleFieldKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Enter" && event.target instanceof HTMLInputElement) {
      event.preventDefault();
      run();
    }
  };

  const problem =
    insights.phase === "QueryError" && problemQuery !== null ? insights.problem : null;
  const problemWords = problem === null ? "" : problemText(problem, t);

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
                selected={insights.selectedLogGroupNames}
                max={insights.maxLogGroups}
                status={insights.logGroupStatus}
                disabled={locked}
                t={t}
                onChange={(names) => onChange("logGroup", names)}
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
                problemMessage={problemWords}
                candidates={candidates}
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
                  {problemWords}
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
              onExample={chooseExample}
            />
            {/* 261020-carryover-fixes FR4.4.1, FR4.4.4: the run's progress belongs
                to the run, so it sits just below the run row, and only while
                the run fetches or evaluates. */}
            {running && <RunProgress insights={insights} t={t} />}
          </SpaceBetween>
        </Container>
        <ResultArea
          session={session}
          t={t}
          onError={onError}
          onShowFailures={() => setFailuresOpen(true)}
          onExport={onExport}
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
      {pendingExample !== null && (
        <ReplaceQueryDialog
          t={t}
          onReplace={() => {
            handleQuery(pendingExample.text);
            setPendingExample(null);
          }}
          onCancel={() => setPendingExample(null)}
        />
      )}
    </div>
  );
}

interface ReplaceQueryDialogProps {
  t: Translate;
  onReplace: () => void;
  onCancel: () => void;
}

/**
 * U2:BR9.3: asks before an example replaces a query the field holds
 * (refined-mockups screen 8: the written text is never dropped silently).
 * Escape or the close button keeps the query.
 */
function ReplaceQueryDialog({ t, onReplace, onCancel }: ReplaceQueryDialogProps) {
  return (
    <Modal
      visible
      size="small"
      header={t("insights.examples.confirm.title")}
      closeAriaLabel={t("insights.examples.confirm.cancel")}
      onDismiss={onCancel}
      footer={
        <Box float="right">
          <SpaceBetween direction="horizontal" size="xs">
            <Button onClick={onCancel} data-testid="replace-query-cancel-button">
              {t("insights.examples.confirm.cancel")}
            </Button>
            <Button variant="primary" onClick={onReplace} data-testid="replace-query-button">
              {t("insights.examples.confirm.replace")}
            </Button>
          </SpaceBetween>
        </Box>
      }
      data-testid="replace-query-dialog"
    >
      <p>{t("insights.examples.confirm.message")}</p>
    </Modal>
  );
}

interface RunBarProps {
  insights: InsightsView;
  locked: boolean;
  mac: boolean;
  t: Translate;
  onRun: () => void;
  onCancel: () => void;
  onExample: (id: string) => void;
}

/** The run row (interaction-spec RunBar, U1:BR1.5, BR2.4), with [Examples] (U2:BR9.3). */
function RunBar({ insights, locked, mac, t, onRun, onCancel, onExample }: RunBarProps) {
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
        <ButtonDropdown
          items={QUERY_EXAMPLES.map((example) => ({ id: example.id, text: t(example.labelKey) }))}
          disabled={locked}
          onItemClick={({ detail }) => onExample(detail.id)}
          data-testid="log-analytics-examples"
        >
          {t("insights.examples")}
        </ButtonDropdown>
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
  onExport: (format: ExportFormat, target: ExportTarget) => void;
}

/**
 * The count beside the results heading (U2:BR5.3, BR7.2): the rows shown,
 * and the matched rows too when a cut dropped some. An aggregates result
 * counts its rows (261025-insights-stats BR3.4; never cut in U1).
 */
function resultCount(result: ResultSummary, t: Translate): string {
  if (result.shape === "aggregates") {
    return t("insights.results.rows", { count: result.rowCount });
  }
  return result.truncatedAt === null
    ? t("insights.results.matched", { count: result.rowCount })
    : t("insights.results.count", { shown: result.rowCount, matched: result.matchedCount });
}

/**
 * The notices above the results and the results container (U1:BR2.4-BR2.7,
 * BR6.1). Since 261020-carryover-fixes FR4.4.2 the container holds only the
 * heading with its count, the result (the previous one while a run goes) and
 * the words for no result; the run's progress is in the query container.
 */
function ResultArea({ session, t, onError, onShowFailures, onExport }: ResultAreaProps) {
  const insights = session.insights;
  const result = insights.result;
  const failedCount = insights.failedStreams.length;
  // U3:BR3.4: the previous result stays shown while the next run fetches and
  // evaluates; a failed fetch or evaluation has dropped it already.
  const showsRows = result !== null;
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
      {insights.phase === "EvaluationFailed" && (
        <div role="alert" data-testid="log-analytics-evaluation-failed">
          <Alert type="error" statusIconAriaLabel={t("failure.kind.Other")}>
            <div>{t("insights.evaluationFailed")}</div>
            <div>{t("insights.evaluationFailed.next")}</div>
          </Alert>
        </div>
      )}
      <Container
        header={
          <Header
            variant="h2"
            counter={showsRows && result !== null ? `(${resultCount(result, t)})` : undefined}
            actions={
              <SpaceBetween direction="horizontal" size="s" alignItems="center">
                {showsRows && result !== null && result.truncatedAt !== null && (
                  <Box data-testid="log-analytics-truncated">
                    {t("insights.results.truncated", { shown: result.truncatedAt })}
                  </Box>
                )}
                {/* 261022-small-features U2:BR4.1: [Export] of the result. */}
                <ExportMenu
                  disabled={!insights.canExport}
                  loading={session.export.status === "Running"}
                  onSelect={onExport}
                  t={t}
                />
              </SpaceBetween>
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
  const running = insights.phase === "Fetching" || insights.phase === "Evaluating";
  const result = insights.result;
  const rows =
    showsRows && result !== null ? (
      result.rowCount === 0 ? (
        result.shape === "aggregates" ? (
          // 261025-insights-stats BR4.5, FR3.2: a `stats ... by` with nothing left.
          <Box data-testid="log-analytics-no-rows">{t("insights.results.noRows")}</Box>
        ) : (
          <Box data-testid="log-analytics-no-match">{t("insights.results.none")}</Box>
        )
      ) : (
        <QueryResultTable
          result={result}
          timeZone={session.timeZone}
          t={t}
          onError={onError}
          previous={running}
        />
      )
    ) : null;
  if (rows !== null) {
    return rows;
  }
  // 261020-carryover-fixes FR4.4.2 (A9): while a run goes with no previous
  // result the body is empty; "No results yet." would not fit a run, and the
  // progress is in the query container.
  if (
    running ||
    insights.phase === "Cancelled" ||
    insights.phase === "FetchFailed" ||
    insights.phase === "EvaluationFailed"
  ) {
    return null;
  }
  return <Box data-testid="log-analytics-initial">{t("insights.results.initial")}</Box>;
}

/**
 * The progress of a run (U1:BR2.4, U3:BR2.7): the cache being read, which log
 * group of how many is fetched with its streams and events, how many log groups
 * failed so far, then the evaluation's progress. Shown in the query container
 * just below the run row, never in the results container (261020-carryover-fixes
 * FR4.4.1, FR4.4.3: only its place changed, not its words, role or test id).
 */
function RunProgress({ insights, t }: { insights: InsightsView; t: Translate }) {
  const progress = insights.progress;
  let text: string;
  if (insights.phase === "Evaluating") {
    text = t("insights.status.running", {
      scanned: progress?.evaluatedCount ?? 0,
      total: progress?.totalCount ?? 0,
    });
  } else if (progress?.readingCache) {
    text = t("insights.status.readingCache");
  } else if (progress === null || progress.groupNumber === null) {
    text = t("insights.status.preparing");
  } else if (progress.plannedStreamCount === null) {
    text = t("insights.status.listing", {
      number: progress.groupNumber,
      groups: progress.groupCount,
      count: progress.selectedStreamCount,
    });
  } else {
    text = t("insights.status.fetching", {
      number: progress.groupNumber,
      groups: progress.groupCount,
      done: progress.finishedStreamCount,
      total: progress.plannedStreamCount,
      events: progress.eventCount,
    });
  }
  const failedGroups = insights.phase === "Fetching" ? (progress?.failedGroupCount ?? 0) : 0;
  return (
    <div role="status" data-testid="log-analytics-progress">
      <StatusIndicator type="loading">{text}</StatusIndicator>
      {failedGroups > 0 && (
        <Box variant="small" data-testid="log-analytics-failed-groups">
          {t("insights.status.failedGroups", { count: failedGroups })}
        </Box>
      )}
    </div>
  );
}

/**
 * U1:BR2.6: the fetch's error in the Log Management words, and that nothing was
 * run; since U3 with every log group that failed and why (U3:BR2.6).
 */
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
        {insights.failedGroups.length > 0 && (
          <div data-testid="log-analytics-failed-group-list">
            <div>{t("insights.fetchFailed.groups")}</div>
            <ul>
              {insights.failedGroups.map((group) => (
                <li key={group.logGroupName} data-testid="log-analytics-failed-group">
                  {group.logGroupName}:{" "}
                  {errorLines(group.failure.kind, "Fetch", group.failure.safeDetail, t).what}
                </li>
              ))}
            </ul>
          </div>
        )}
        <div>{t("insights.notRun")}</div>
      </Alert>
    </div>
  );
}
