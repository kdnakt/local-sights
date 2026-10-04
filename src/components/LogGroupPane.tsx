import { useState, type KeyboardEvent } from "react";
import type { SessionView } from "../api";
import type { Translate } from "../i18n/messages";

export interface LogGroupPaneProps {
  session: SessionView;
  t: Translate;
  onFilterChange: (text: string) => void;
  onReload: () => void;
  onSelect: (name: string) => void;
}

/**
 * Left pane: filter (U2:BR3.7), the single-choice log group list (U2:BR3.8)
 * and Reload (U2:BR3.5), in that Tab order (U2:BR5.2), with the list's
 * loading, partial (kind and safe detail, U2:BR3.4), empty (U2:BR3.10) and
 * prompt (U2:BR2.1, BR2.2) messages. The list is a keyboard listbox: arrow
 * keys move, Enter or Space selects (U2:BR5.2). Mount with a `key` to reset
 * the filter draft from a session.
 */
export function LogGroupPane({
  session,
  t,
  onFilterChange,
  onReload,
  onSelect,
}: LogGroupPaneProps) {
  const [filter, setFilter] = useState(session.logGroupFilter);
  const [activeIndex, setActiveIndex] = useState(0);
  const disabled = !session.canChangeConnection;
  const groups = session.logGroups?.visibleGroups ?? [];
  const active = Math.min(activeIndex, Math.max(groups.length - 1, 0));

  const choose = (name: string | undefined) => {
    if (!disabled && name !== undefined) {
      onSelect(name);
    }
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLUListElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex(Math.min(active + 1, groups.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex(Math.max(active - 1, 0));
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      choose(groups[active]);
    }
  };

  return (
    <nav className="log-group-pane" aria-label={t("logGroups.label")} data-testid="log-group-pane">
      <div className="log-group-pane-tools">
        <label className="log-group-pane-filter">
          <span>{t("logGroups.filter.label")}</span>
          <input
            type="text"
            value={filter}
            placeholder={t("logGroups.filter.placeholder")}
            autoComplete="off"
            spellCheck={false}
            disabled={session.pendingChange !== null}
            data-testid="log-group-pane-filter-input"
            onChange={(event) => {
              setFilter(event.target.value);
              onFilterChange(event.target.value);
            }}
          />
        </label>
      </div>
      <div className="log-group-pane-status" role="status" data-testid="log-group-pane-status">
        {renderStatus(session, t)}
      </div>
      {groups.length > 0 && (
        <ul
          className="log-group-pane-list"
          role="listbox"
          aria-label={t("logGroups.label")}
          aria-disabled={disabled}
          aria-activedescendant={`log-group-option-${active}`}
          tabIndex={0}
          onKeyDown={handleKeyDown}
          data-testid="log-group-pane-list"
        >
          {groups.map((name, index) => (
            <li
              key={name}
              id={`log-group-option-${index}`}
              role="option"
              aria-selected={name === session.selectedLogGroupName}
              className={
                index === active ? "log-group-pane-option active" : "log-group-pane-option"
              }
              onClick={() => {
                setActiveIndex(index);
                choose(name);
              }}
              data-testid="log-group-pane-option"
            >
              {name}
            </li>
          ))}
        </ul>
      )}
      <button
        type="button"
        disabled={!session.canReload}
        onClick={onReload}
        data-testid="log-group-pane-reload-button"
      >
        {t("logGroups.reload")}
      </button>
    </nav>
  );
}

function renderStatus(session: SessionView, t: Translate) {
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
  if (list.status === "Partial" && list.failure) {
    // A partial list can still be filtered to nothing: say both (U2:BR3.10).
    return (
      <>
        <span data-testid="log-group-pane-partial">
          {t("logGroups.partial", { kind: t(`failure.kind.${list.failure.kind}`) })}
        </span>{" "}
        <span data-testid="log-group-pane-detail">
          {t("status.detail", { detail: list.failure.safeDetail })}
        </span>
        {list.emptyState === "NoMatches" && (
          <>
            {" "}
            <span data-testid="log-group-pane-no-matches">{t("logGroups.empty.noMatches")}</span>
          </>
        )}
      </>
    );
  }
  if (list.status === "Loading") {
    return t("logGroups.loading");
  }
  if (list.emptyState === "NoGroups") {
    return t("logGroups.empty.noGroups");
  }
  if (list.emptyState === "NoMatches") {
    return t("logGroups.empty.noMatches");
  }
  return null;
}
