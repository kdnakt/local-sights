import Box from "@cloudscape-design/components/box";
import Multiselect from "@cloudscape-design/components/multiselect";
import SpaceBetween from "@cloudscape-design/components/space-between";
import { limitSelection } from "../analytics";
import type { ListingStatus } from "../api";
import type { Translate } from "../i18n/messages";

export interface LogGroupMultiselectProps {
  /** Every log group of the current list, ascending (U1:BR1.3). */
  options: readonly string[];
  /** The chosen log groups, in the order chosen (U3:BR4.1). */
  selected: readonly string[];
  /** The most that can be chosen (50, U3:BR4.1). */
  max: number;
  /** How the list stands; `null` without a complete connection. */
  status: ListingStatus | null;
  disabled: boolean;
  t: Translate;
  /** The new choice; empty when every token was removed. */
  onChange: (names: string[]) => void;
}

/**
 * The log groups of the query screen (261017-insights-query refined-mockups
 * screen 2, Q2, interaction-spec LogGroupMultiselect): Cloudscape's
 * Multiselect, searched by part of the name, with each chosen group as a
 * token under it that × removes. The options are the current connection's
 * list, the same one Log Management's left pane shows, whatever that pane's
 * own filter (U1:BR1.3). Since U3 up to `max` (50) are chosen: once that many
 * are, the others cannot be chosen and a line under the field says why
 * (U3:BR4.1). The list's loading and failure show inside the dropdown
 * (screen 6e). Disabled while the query runs or a dialog is open (U1:BR2.4).
 */
export function LogGroupMultiselect({
  options,
  selected,
  max,
  status,
  disabled,
  t,
  onChange,
}: LogGroupMultiselectProps) {
  const full = selected.length >= max;
  const items = options.map((name) => ({
    label: name,
    value: name,
    disabled: full && !selected.includes(name),
  }));
  const statusType = status === "Loading" ? "loading" : status === "Partial" ? "error" : "finished";
  return (
    <SpaceBetween size="xxs">
      <Multiselect
        selectedOptions={selected.map((name) => ({ label: name, value: name }))}
        options={items}
        onChange={({ detail }) =>
          onChange(
            limitSelection(
              detail.selectedOptions.map((option) => option.value ?? ""),
              max,
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
        keepOpen
        expandToViewport
        data-testid="log-group-multiselect"
      />
      {full && (
        <Box variant="small" color="text-body-secondary" data-testid="log-group-limit">
          <span role="status">{t("insights.logGroups.limit", { max })}</span>
        </Box>
      )}
    </SpaceBetween>
  );
}
