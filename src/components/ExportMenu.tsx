import ButtonDropdown from "@cloudscape-design/components/button-dropdown";
import type { ExportFormat, ExportTarget } from "../api";
import type { MessageKey, Translate } from "../i18n/messages";

export interface ExportMenuProps {
  /** The screen's list cannot be exported now (U2:BR2.3, BR3.2). */
  disabled: boolean;
  /** An export is under way: disabled and shown as busy (U2:BR4.8). */
  loading: boolean;
  onSelect: (format: ExportFormat, target: ExportTarget) => void;
  t: Translate;
}

interface Item {
  id: string;
  key: MessageKey;
  format: ExportFormat;
  target: ExportTarget;
}

/** The four items, in the order of U2:BR4.1 (ADR-006). */
const ITEMS: readonly Item[] = [
  { id: "csv-file", key: "export.item.csvFile", format: "csv", target: "file" },
  { id: "markdown-file", key: "export.item.markdownFile", format: "markdown", target: "file" },
  { id: "csv-clipboard", key: "export.item.csvClipboard", format: "csv", target: "clipboard" },
  {
    id: "markdown-clipboard",
    key: "export.item.markdownClipboard",
    format: "markdown",
    target: "clipboard",
  },
];

/**
 * The [Export] menu of either screen (261022-small-features U2:BR4.1,
 * ADR-006): one dropdown with "Save as CSV", "Save as Markdown", "Copy as
 * CSV" and "Copy as Markdown", in that order. The whole menu is disabled
 * while the list cannot be exported, and shown busy (and disabled) while an
 * export is under way (U2:BR4.8). It can be opened and chosen from with the
 * keyboard and is named "Export" for screen readers (U2:BR5.5).
 */
export function ExportMenu({ disabled, loading, onSelect, t }: ExportMenuProps) {
  return (
    <ButtonDropdown
      items={ITEMS.map((item) => ({ id: item.id, text: t(item.key) }))}
      disabled={disabled || loading}
      loading={loading}
      loadingText={t("export.exporting")}
      ariaLabel={t("export.menu")}
      onItemClick={({ detail }) => {
        const item = ITEMS.find((candidate) => candidate.id === detail.id);
        if (item !== undefined) {
          onSelect(item.format, item.target);
        }
      }}
      data-testid="export-menu"
    >
      {t("export.menu")}
    </ButtonDropdown>
  );
}
