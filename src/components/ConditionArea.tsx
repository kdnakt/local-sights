import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import Box from "@cloudscape-design/components/box";
import Button from "@cloudscape-design/components/button";
import Container from "@cloudscape-design/components/container";
import DatePicker from "@cloudscape-design/components/date-picker";
import FormField from "@cloudscape-design/components/form-field";
import Header from "@cloudscape-design/components/header";
import StatusIndicator from "@cloudscape-design/components/status-indicator";
import type { TextFilterProps } from "@cloudscape-design/components/text-filter";
import TimeInput from "@cloudscape-design/components/time-input";
import type { InputField, SessionView } from "../api";
import {
  completeTime,
  completionWouldEnableFetch,
  fieldReasons,
  isFutureDate,
  isMaskCompletionOf,
  isoToSlashed,
  isValidDateText,
  joinDateTime,
  slashedToIso,
  splitDateTime,
  todayInZone,
  type DateTimeParts,
  type Side,
} from "../dateTime";
import { timeZoneLabelKey, type Locale, type MessageKey, type Translate } from "../i18n/messages";
import { LogFilterInput } from "./LogFilterInput";

interface SideSpec {
  field: InputField;
  label: MessageKey;
  dateLabel: MessageKey;
  timeLabel: MessageKey;
  testId: string;
}

const SIDES: Record<Side, SideSpec> = {
  start: {
    field: "startText",
    label: "form.start.label",
    dateLabel: "form.start.date",
    timeLabel: "form.start.time",
    testId: "fetch-form-start",
  },
  end: {
    field: "endText",
    label: "form.end.label",
    dateLabel: "form.end.date",
    timeLabel: "form.end.time",
    testId: "fetch-form-end",
  },
};

const SIDE_ORDER: readonly Side[] = ["start", "end"];

export interface ConditionAreaProps {
  session: SessionView;
  t: Translate;
  /** The language of the calendar (month names, weekdays; FR4.6). */
  locale?: Locale;
  onChange: (field: InputField, value: string) => void;
  onFetch: () => void;
  /** [Cancel] of the running fetch (261012-parallel-fetch U5:BR5.1). */
  onCancel: () => void;
  /** The log filter text in the field, kept by the parent (U5:BR3.4). */
  filterText: string;
  onFilterTextChange: (value: string) => void;
  /** The log filter text after the typing pause (U5:BR1.3). */
  onLogFilterChange: (text: string) => void;
  /** Failed log filter hand-overs so far (U5 review R-02). */
  filterFailureCount?: number;
  /** The error area, the log list and the status line, inside the same container. */
  children?: ReactNode;
}

/** Whether the core already holds the texts the fields show. */
function synced(session: SessionView, drafts: Record<Side, DateTimeParts>): boolean {
  return SIDE_ORDER.every(
    (side) =>
      session[side === "start" ? "startInput" : "endInput"].text ===
      joinDateTime(drafts[side].date, drafts[side].time),
  );
}

/**
 * The right pane's container (mockups screens 1, 2, 5a; FR4.2): the heading
 * is the selected log group (U2:BR3.8), or "Log group: none" (gate fix 15c:
 * the prompt "Choose a log group." is the reason line's alone, so it is not
 * said twice), with the primary [Fetch] at its right; below it the filter
 * field (U5:BR3.4) on the left, under the same kind of label as the dates
 * (gate fix 15d), and, on the right, the start and the end, each a date field with a
 * calendar (`yyyy/mm/dd`, FR1.1, FR1.2) and a time field (`hh:mm:ss`,
 * FR1.3). The core keeps one text per input: every change joins the two
 * fields and goes to the core, which owns the state and the time zone
 * (interaction-spec §8); the parent remounts the area (`key`) when the zone
 * changes, so the fields take the texts the core rewrote (FR1.5).
 *
 * A short or empty time is completed when the field loses focus (start
 * `00`, end `59`; mockups Q4/Q5) and before Enter or [Fetch] starts the fetch
 * (R-01); the completed value is shown in the field (accessibility E3). The
 * reasons why Fetch is unavailable are shown under the field they belong to
 * (FR3.1, R-02): the shape of each field first, then the core's reasons; the
 * list "Fetch is unavailable because:" of the MVP is gone. While fetching the
 * date and time are disabled, [Fetch] becomes [Cancel] (see below), and the
 * filter field stays usable
 * (FR1.8, U5:BR3.4); while a dialog is open everything is disabled
 * (U2:BR2.6, U6:BR5.1). When [Fetch] becomes disabled while it has the
 * focus, the focus moves to the filter field (R-06). Enter in a date or time
 * field starts the fetch when it is available (U1:BR6.2).
 *
 * The calendar opens with `expandToViewport`, outside the container (whose
 * fit-height content clips overflow), and Cloudscape sizes that dropdown to
 * the trigger; the date field is therefore as wide as the DatePicker's own
 * maximum (`styles.css` `.condition-date`), so the whole month and the
 * month buttons are visible (gate fix 14a).
 *
 * The calendar offers no day after today, today being read in the zone the
 * fields show (`session.timeZone`) when the calendar draws its days (gate
 * fix 17b). A future date typed by hand is left alone: the request was about
 * the calendar, and such a fetch only finds no events.
 *
 * The children (the error alerts, the log list and the status line) share
 * one flex column with the conditions, inside a frame as high as the
 * container (gate fix 18a). `fitHeight` makes the container as high as the
 * right pane, but its inner content box keeps the height of what it holds,
 * so a long list used to push the status line below the window; the frame
 * holds the column with absolute positioning, which takes the column out of
 * that height, and the list alone shrinks and scrolls.
 *
 * Since 261012-parallel-fetch U5 the same button is [Cancel] while fetching
 * (BR5.10, nfr-design review R-09): [Fetch] outside a fetch; while fetching a
 * primary [Cancel] when the core says it can be pressed
 * (`fetchCancelAvailable`), a `loading` [Cancelling…] once it was pressed
 * (`cancelling`; Cloudscape keeps a loading button focusable, so the focus
 * stays), and a disabled [Cancel] otherwise (while the cache is written or
 * closing awaits an answer). R-06 now moves the focus only when the button
 * itself is disabled: [Fetch] that cannot be pressed, or that disabled
 * [Cancel] (NFR10.1). Enter in a date or time field never cancels.
 */
export function ConditionArea({
  session,
  t,
  locale = "en",
  onChange,
  onFetch,
  onCancel,
  filterText,
  onFilterTextChange,
  onLogFilterChange,
  filterFailureCount = 0,
  children,
}: ConditionAreaProps) {
  // Local drafts keep typing responsive; every change is forwarded to the
  // core, which owns the state. Mount with a `key` to reset from a session.
  const [drafts, setDrafts] = useState<Record<Side, DateTimeParts>>(() => ({
    start: splitDateTime(session.startInput.text),
    end: splitDateTime(session.endInput.text),
  }));
  const draftsRef = useRef(drafts);
  const pendingFetchRef = useRef(false);
  const filterRef = useRef<TextFilterProps.Ref>(null);
  const fetchWrapRef = useRef<HTMLSpanElement>(null);

  const zone = t(timeZoneLabelKey(session.timeZone));
  const fetching = session.phase === "Fetching";
  const dialogOpen = session.pendingChange !== null || session.settingsDialog === "Open";
  const locked = fetching || dialogOpen;
  const reasonInput = {
    start: drafts.start,
    end: drafts.end,
    validationErrors: session.validationErrors,
  };
  const reasons = fieldReasons(reasonInput);
  const canSubmit = !locked && (session.canFetch || completionWouldEnableFetch(reasonInput));
  // U5 R-09: what the one button is while fetching.
  const cancelling = fetching && session.cancelling;
  const canCancel = fetching && !session.cancelling && session.fetchCancelAvailable;
  const buttonDisabled = fetching ? !(cancelling || canCancel) : !canSubmit;

  const commit = (side: Side, parts: DateTimeParts) => {
    draftsRef.current = { ...draftsRef.current, [side]: parts };
    setDrafts(draftsRef.current);
    onChange(SIDES[side].field, joinDateTime(parts.date, parts.time));
  };

  const handleDate = (side: Side, date: string) => {
    commit(side, { ...draftsRef.current[side], date });
  };

  // Q5: the mask fills a short time with zeros when the field loses focus
  // or Enter is pressed; the end field takes `:59` instead.
  const handleTime = (side: Side, value: string) => {
    const previous = draftsRef.current[side].time;
    const time = isMaskCompletionOf(previous, value) ? completeTime(previous, side) : value;
    commit(side, { ...draftsRef.current[side], time });
  };

  /**
   * Q4, Q5: completes the time of `side`; returns whether it changed. An
   * empty or malformed date gets no completion: the date is required and
   * its own reason is shown (mockups screen 2).
   */
  const completeSide = (side: Side): boolean => {
    const parts = draftsRef.current[side];
    const time = isValidDateText(parts.date) ? completeTime(parts.time, side) : parts.time;
    if (time === parts.time) {
      return false;
    }
    commit(side, { ...parts, time });
    return true;
  };

  /**
   * R-01: Enter or [Fetch]. The times are completed first; when the core
   * already holds what the fields show, the fetch starts at once, otherwise
   * it starts once the core has taken the fields and still allows it.
   */
  const submit = () => {
    if (locked) {
      return;
    }
    const changed = SIDE_ORDER.map(completeSide).some(Boolean);
    if (!changed && synced(session, draftsRef.current)) {
      if (session.canFetch) {
        onFetch();
      }
      return;
    }
    pendingFetchRef.current = true;
  };

  useEffect(() => {
    if (!pendingFetchRef.current || !synced(session, draftsRef.current)) {
      return;
    }
    pendingFetchRef.current = false;
    if (session.canFetch && !locked) {
      onFetch();
    }
  }, [session, locked, onFetch]);

  // R-06: a disabled button cannot keep the focus; the filter field takes it.
  // [Cancel] and [Cancelling…] keep it (U5 NFR10.1).
  useEffect(() => {
    if (buttonDisabled && fetchWrapRef.current?.contains(document.activeElement)) {
      filterRef.current?.focus();
    }
  }, [buttonDisabled]);

  const handleButton = () => {
    if (!fetching) {
      submit();
    } else if (canCancel) {
      onCancel();
    }
  };

  const buttonLabel = !fetching
    ? t("form.fetch")
    : cancelling
      ? t("fetch.cancelling")
      : t("fetch.cancel");

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    // U1:BR6.2: Enter in a date or time field; the calendar's own Enter is
    // not on an input and starts nothing.
    if (event.key === "Enter" && event.target instanceof HTMLInputElement) {
      event.preventDefault();
      submit();
    }
  };

  const reasonText = (key: string | undefined) =>
    key === undefined ? undefined : t(key, { zone });

  // Gate fix 17b: Cloudscape hands each day as local midnight, so its local
  // year, month and day are the calendar date; today is read in the shown zone.
  const isDateEnabled = (date: Date) =>
    !isFutureDate(todayInZone(date.getTime(), "Local"), todayInZone(Date.now(), session.timeZone));

  return (
    <Container
      fitHeight
      header={
        <Header
          variant="h2"
          actions={
            <span ref={fetchWrapRef}>
              <Button
                variant="primary"
                disabled={buttonDisabled}
                loading={cancelling}
                loadingText={cancelling ? t("fetch.cancelling") : undefined}
                onClick={handleButton}
                data-testid="fetch-form-submit-button"
              >
                {buttonLabel}
              </Button>
            </span>
          }
        >
          <span data-testid="fetch-form-selected-log-group">
            {session.selectedLogGroupName ?? t("form.selectedLogGroup.none")}
          </span>
        </Header>
      }
      data-testid="fetch-form"
    >
      {/* Gate fix 18a: the frame takes the container's height; the column
          inside is laid over it, so the rows of the list never make the
          container taller than the window (see `styles.css`). */}
      <div className="condition-area-frame" data-testid="condition-area-frame">
        <div className="condition-area" data-testid="condition-area">
          {reasons.logGroup !== undefined && (
            <div className="condition-reason" data-testid="fetch-form-reason-log-group">
              <StatusIndicator type="error" iconAriaLabel={t("failure.kind.Other")}>
                {reasonText(reasons.logGroup)}
              </StatusIndicator>
            </div>
          )}
          <div className="condition-row">
            <div className="condition-row-filter" data-testid="fetch-form-filter">
              {/* Gate fix 15d: the same label row as the start and the end, so
                the three fields share one top edge. */}
              <Box variant="awsui-key-label">{t("logFilter.label")}</Box>
              <LogFilterInput
                inputRef={filterRef}
                value={filterText}
                appliedText={session.logFilter}
                disabled={dialogOpen}
                failureCount={filterFailureCount}
                t={t}
                onChange={onFilterTextChange}
                onFilterChange={onLogFilterChange}
              />
            </div>
            <div className="condition-row-times" onKeyDown={handleKeyDown}>
              {SIDE_ORDER.map((side) => {
                const spec = SIDES[side];
                const dateReason = reasonText(reasons[`${side}Date`]);
                const timeReason = reasonText(reasons[`${side}Time`]);
                return (
                  <div
                    key={side}
                    role="group"
                    aria-label={t(spec.label, { zone })}
                    data-testid={spec.testId}
                  >
                    <Box variant="awsui-key-label">{t(spec.label, { zone })}</Box>
                    <div className="condition-date-time">
                      <div className="condition-date">
                        <FormField errorText={dateReason} data-testid={`${spec.testId}-date-field`}>
                          <DatePicker
                            value={slashedToIso(drafts[side].date)}
                            onChange={({ detail }) => handleDate(side, isoToSlashed(detail.value))}
                            isDateEnabled={isDateEnabled}
                            placeholder={t("form.date.placeholder")}
                            ariaLabel={t(spec.dateLabel)}
                            locale={locale}
                            disabled={locked}
                            expandToViewport
                            data-testid={`${spec.testId}-date`}
                          />
                        </FormField>
                      </div>
                      <div className="condition-time">
                        <FormField errorText={timeReason} data-testid={`${spec.testId}-time-field`}>
                          <TimeInput
                            value={drafts[side].time}
                            format="hh:mm:ss"
                            use24Hour
                            placeholder={t("form.time.placeholder")}
                            ariaLabel={t(spec.timeLabel)}
                            disabled={locked}
                            onChange={({ detail }) => handleTime(side, detail.value)}
                            onBlur={() => completeSide(side)}
                            data-testid={`${spec.testId}-time`}
                          />
                        </FormField>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
          {children}
        </div>
      </div>
    </Container>
  );
}
