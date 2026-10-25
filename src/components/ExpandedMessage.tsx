import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import SegmentedControl from "@cloudscape-design/components/segmented-control";
import type { Translate } from "../i18n/messages";
import { findJson } from "../jsonPretty";

export interface ExpandedMessageProps {
  /** The message exactly as the API returned it. */
  message: string;
  /** Takes the focus with Tab (the selected row's scrolling expansion, BR1.10). */
  focusable: boolean;
  /** Reports the measured height and whether the text scrolls (BR1.9). */
  onMeasure: (height: number, scrolls: boolean) => void;
  /** Escape or Shift+Tab: back to the list (BR1.10). */
  onLeave: () => void;
  label: string;
  t: Translate;
}

/** The "Formatted / Original" choice of one opened row (entities.md PrettyPrintMode). */
type PrettyPrintMode = "pretty" | "raw";

/**
 * The expansion below an expanded row (U7:BR1.2, BR1.3, BR1.10): the whole
 * message as a text node only (never parsed as HTML, no links), wrapped
 * anywhere, at most 20 lines (372 px) high, scrolling inside beyond that. Its
 * text can be selected and copied; pressing inside it never opens or closes
 * the row (only the row's line does). It sits in a full-width grid cell of
 * its row. When it is the selected row's and it scrolls, Tab enters it
 * (tabIndex 0, review R-13); Escape or Shift+Tab goes back to the list. The
 * scrolling text is a named region ("Full message"), so screen readers
 * announce its name (code generation review R-06).
 *
 * Since 261022-small-features U2 (BR5.1-BR5.4, ADR-003) the first readable
 * JSON object or array of the message is pretty-printed, token for token,
 * with a "Formatted / Original" switch above the text; it starts on
 * "Formatted", belongs to this opened row alone and goes back to "Formatted"
 * when the row is closed and opened again, since the component is remade.
 * The shown text (the prefix, the formatted JSON and the suffix, or the
 * message as it is) is still one text node, so what is selected and copied
 * is what is shown (FR5.8). A message without readable JSON gets no switch
 * and is shown as before (FR5.6). The switch stops its clicks and keys from
 * reaching the row and the grid, so choosing it never opens, closes or
 * selects a row (functional-design review R-05); the 20-line limit applies
 * to the text alone, and the measured height is the whole expansion's
 * (switch and text).
 */
export function ExpandedMessage({
  message,
  focusable,
  onMeasure,
  onLeave,
  label,
  t,
}: ExpandedMessageProps) {
  const innerRef = useRef<HTMLDivElement>(null);
  const textRef = useRef<HTMLDivElement>(null);
  const measureRef = useRef(onMeasure);
  const segments = useMemo(() => findJson(message), [message]);
  const [mode, setMode] = useState<PrettyPrintMode>("pretty");

  useEffect(() => {
    measureRef.current = onMeasure;
  }, [onMeasure]);

  useEffect(() => {
    const inner = innerRef.current;
    if (inner === null || typeof ResizeObserver === "undefined") {
      return undefined;
    }
    const observer = new ResizeObserver((entries) => {
      const entry = entries[entries.length - 1];
      const height = entry?.borderBoxSize?.[0]?.blockSize ?? inner.getBoundingClientRect().height;
      const text = textRef.current;
      const scrolls = text !== null && text.scrollHeight > text.clientHeight + 1;
      measureRef.current(height, scrolls);
    });
    observer.observe(inner);
    return () => observer.disconnect();
  }, []);

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape" || (event.key === "Tab" && event.shiftKey)) {
      event.preventDefault();
      event.stopPropagation();
      onLeave();
    }
  };

  // R-05: nothing from the switch reaches the row's line or the grid.
  const stopPropagation = (event: KeyboardEvent<HTMLDivElement> | MouseEvent<HTMLDivElement>) => {
    event.stopPropagation();
  };

  const shown =
    segments === null || mode === "raw"
      ? message
      : segments.prefix + segments.pretty + segments.suffix;

  return (
    <div className="log-expanded" role="gridcell" aria-colspan={3}>
      <div ref={innerRef} className="log-expanded-inner" data-testid="log-table-expanded-inner">
        {segments !== null && (
          <div
            className="log-expanded-mode"
            data-testid="log-table-expanded-mode"
            onKeyDown={stopPropagation}
            onClick={stopPropagation}
          >
            <SegmentedControl
              selectedId={mode}
              label={t("expanded.mode")}
              options={[
                { id: "pretty", text: t("expanded.mode.pretty") },
                { id: "raw", text: t("expanded.mode.raw") },
              ]}
              onChange={({ detail }) => setMode(detail.selectedId === "raw" ? "raw" : "pretty")}
            />
          </div>
        )}
        <div
          ref={textRef}
          className="log-expanded-text"
          role="region"
          tabIndex={focusable ? 0 : -1}
          aria-label={label}
          data-testid="log-table-expanded"
          onKeyDown={handleKeyDown}
        >
          {shown}
        </div>
      </div>
    </div>
  );
}
