import { useState, type Ref } from "react";
import Alert from "@cloudscape-design/components/alert";
import Box from "@cloudscape-design/components/box";
import Button, { type ButtonProps } from "@cloudscape-design/components/button";
import Container from "@cloudscape-design/components/container";
import Header from "@cloudscape-design/components/header";
import StatusIndicator from "@cloudscape-design/components/status-indicator";
import Table from "@cloudscape-design/components/table";
import TextFilter from "@cloudscape-design/components/text-filter";
import type { SessionView } from "../api";
import { errorLines } from "../errorText";
import type { Translate } from "../i18n/messages";

export interface LogGroupPaneProps {
  session: SessionView;
  t: Translate;
  onFilterChange: (text: string) => void;
  onReload: () => void;
  onSelect: (name: string) => void;
  /** Closes the pane (gate fix 15b); without it no close button is shown. */
  onCollapse?: () => void;
  /** The close button, so the parent can focus it when the pane reopens. */
  collapseButtonRef?: Ref<ButtonProps.Ref>;
}

/** One row of the table: a log group name. */
interface LogGroupRow {
  name: string;
}

/**
 * Left pane (mockups screen 1, Q6, FR4.2): a container headed "Log groups"
 * with the search field (U2:BR3.7), a single-selection table of the names
 * (U2:BR3.8) and [Reload] (U2:BR3.5), in that Tab order (U2:BR5.2). The
 * table's empty state carries the prompts (U2:BR2.1, BR2.2), the loading
 * text and the empty texts (U2:BR3.10); a partial list shows a warning alert
 * above the table with the reason in words, what to do and the safe detail
 * (U2:BR3.4, U7:BR2.4). While fetching or while a change awaits confirmation
 * the rows cannot be selected and [Reload] is disabled (the search field stays
 * usable while fetching only). Clicking a row or its radio selects it; the
 * arrow keys move between the radios and select (U2:BR5.2). Mount with a `key`
 * to reset the filter draft from a session. The heading carries the button
 * that closes the pane (gate fix 15b: "close it completely so the messages are
 * easier to read"); the parent then shows a slim strip that opens it again.
 * The names are cut with an ellipsis when the pane is narrow (the full name
 * is the cell's title).
 */
export function LogGroupPane({
  session,
  t,
  onFilterChange,
  onReload,
  onSelect,
  onCollapse,
  collapseButtonRef,
}: LogGroupPaneProps) {
  const [filter, setFilter] = useState(session.logGroupFilter);
  const disabled = !session.canChangeConnection;
  const list = session.logGroups;
  const groups = list?.visibleGroups ?? [];
  const items: LogGroupRow[] = groups.map((name) => ({ name }));
  const selected = session.selectedLogGroupName;
  const selectedItems: LogGroupRow[] =
    selected !== null && groups.includes(selected) ? [{ name: selected }] : [];
  const loading = list?.status === "Loading";

  const choose = (name: string | undefined) => {
    if (!disabled && name !== undefined) {
      onSelect(name);
    }
  };

  return (
    <Container
      header={
        <Header
          variant="h2"
          actions={
            onCollapse && (
              <Button
                ref={collapseButtonRef}
                variant="icon"
                iconName="angle-left"
                ariaLabel={t("pane.collapse")}
                onClick={onCollapse}
                data-testid="log-group-pane-collapse-button"
              />
            )
          }
        >
          {t("logGroups.label")}
        </Header>
      }
      fitHeight
      className="log-group-pane"
      data-testid="log-group-pane"
    >
      <div className="log-group-pane-body">
        <TextFilter
          filteringText={filter}
          filteringPlaceholder={t("logGroups.filter.placeholder")}
          filteringAriaLabel={t("logGroups.filter.label")}
          filteringClearAriaLabel={t("logGroups.filter.clear")}
          disabled={session.pendingChange !== null}
          onChange={({ detail }) => {
            setFilter(detail.filteringText);
            onFilterChange(detail.filteringText);
          }}
          data-testid="log-group-pane-filter"
        />
        {list?.failure && (
          <Alert
            type="warning"
            header={<span data-testid="log-group-pane-partial">{t("logGroups.partial")}</span>}
            statusIconAriaLabel={t("failure.kind.Other")}
            data-testid="log-group-pane-error"
          >
            <ListingError failure={list.failure} t={t} />
          </Alert>
        )}
        {loading && groups.length > 0 && (
          <Box data-testid="log-group-pane-status">
            <StatusIndicator type="loading">{t("logGroups.loading")}</StatusIndicator>
          </Box>
        )}
        <div className="log-group-pane-table">
          <Table<LogGroupRow>
            variant="embedded"
            items={items}
            trackBy="name"
            selectionType="single"
            selectedItems={selectedItems}
            isItemDisabled={() => disabled}
            onSelectionChange={({ detail }) => choose(detail.selectedItems[0]?.name)}
            onRowClick={({ detail }) => choose(detail.item.name)}
            loading={loading && groups.length === 0}
            loadingText={t("logGroups.loading")}
            empty={
              <Box
                textAlign="center"
                color="text-body-secondary"
                data-testid="log-group-pane-status"
              >
                {emptyText(session, t)}
              </Box>
            }
            ariaLabels={{
              tableLabel: t("logGroups.label"),
              selectionGroupLabel: t("logGroups.selection"),
              itemSelectionLabel: (_, row) => row.name,
            }}
            columnDefinitions={[
              {
                id: "name",
                header: t("logGroups.column.name"),
                cell: (row) => (
                  <span
                    className="log-group-pane-name"
                    title={row.name}
                    data-testid="log-group-pane-option"
                  >
                    {row.name}
                  </span>
                ),
              },
            ]}
            data-testid="log-group-pane-table"
          />
        </div>
        <Box>
          <Button
            disabled={!session.canReload}
            onClick={onReload}
            data-testid="log-group-pane-reload-button"
          >
            {t("logGroups.reload")}
          </Button>
        </Box>
      </div>
    </Container>
  );
}

/** The three lines of a listing failure (U7:BR2.4), inside the warning alert. */
function ListingError({
  failure,
  t,
}: {
  failure: NonNullable<SessionView["logGroups"]>["failure"] & object;
  t: Translate;
}) {
  const lines = errorLines(failure.kind, "Listing", failure.safeDetail, t);
  return (
    <>
      <div data-testid="log-group-pane-error-what">{lines.what}</div>
      <div data-testid="log-group-pane-error-next">{lines.next}</div>
      {lines.detail !== null && <div data-testid="log-group-pane-error-detail">{lines.detail}</div>}
    </>
  );
}

/** What the empty table says: the prompts, or why there are no rows. */
function emptyText(session: SessionView, t: Translate): string | null {
  if (session.connection.profile === null) {
    return t("logGroups.prompt.profile");
  }
  if (session.connection.region === null) {
    return t("logGroups.prompt.region");
  }
  const list = session.logGroups;
  if (list === null) {
    return null;
  }
  if (list.emptyState === "NoMatches") {
    return t("logGroups.empty.noMatches");
  }
  if (list.emptyState === "NoGroups") {
    return t("logGroups.empty.noGroups");
  }
  return null;
}
