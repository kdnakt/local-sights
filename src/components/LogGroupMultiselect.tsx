import Multiselect from "@cloudscape-design/components/multiselect";
import { newestSelection } from "../analytics";
import type { ListingStatus } from "../api";
import type { Translate } from "../i18n/messages";

export interface LogGroupMultiselectProps {
  /** Every log group of the current list, ascending (U1:BR1.3). */
  options: readonly string[];
  /** The selected log group; U1 keeps one at most. */
  selected: string | null;
  /** How the list stands; `null` without a complete connection. */
  status: ListingStatus | null;
  disabled: boolean;
  t: Translate;
  /** The new selection, `null` when the token was removed. */
  onChange: (name: string | null) => void;
}

/**
 * The log group of the query screen (261017-insights-query refined-mockups
 * screen 2, Q2): Cloudscape's Multiselect, searched by part of the name,
 * with the chosen group as a token under it that × removes. The options are
 * the current connection's list, the same one Log Management's left pane
 * shows, whatever that pane's own filter (U1:BR1.3). U1 keeps one group: a
 * new choice replaces the previous one; up to 50 come with U3. The list's
 * loading and failure show inside the dropdown (screen 6e). Disabled while
 * the query runs or a dialog is open (U1:BR2.4).
 */
export function LogGroupMultiselect({
  options,
  selected,
  status,
  disabled,
  t,
  onChange,
}: LogGroupMultiselectProps) {
  const items = options.map((name) => ({ label: name, value: name }));
  const statusType = status === "Loading" ? "loading" : status === "Partial" ? "error" : "finished";
  return (
    <Multiselect
      selectedOptions={selected === null ? [] : [{ label: selected, value: selected }]}
      options={items}
      onChange={({ detail }) =>
        onChange(
          newestSelection(
            detail.selectedOptions.map((option) => option.value ?? ""),
            selected,
          ),
        )
      }
      filteringType="auto"
      filteringPlaceholder={t("insights.logGroups.filter")}
      filteringAriaLabel={t("insights.logGroups.filter")}
      placeholder={t("insights.logGroups.placeholder")}
      ariaLabel={t("insights.logGroups.label")}
      statusType={statusType}
      loadingText={t("insights.logGroups.loading")}
      errorText={t("insights.logGroups.error")}
      empty={t("insights.logGroups.empty")}
      noMatch={t("insights.logGroups.noMatch")}
      deselectAriaLabel={(option) => t("insights.logGroups.remove", { name: option.label ?? "" })}
      disabled={disabled}
      keepOpen={false}
      expandToViewport
      data-testid="log-group-multiselect"
    />
  );
}
