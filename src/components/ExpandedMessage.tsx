import { useEffect, useRef, type KeyboardEvent } from "react";

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
}

/**
 * The expansion below an expanded row (U7:BR1.2, BR1.3, BR1.10): the whole
 * message as a text node only (never parsed as HTML, no links, JSON not
 * reformatted), wrapped anywhere, at most 20 lines (372 px) high, scrolling
 * inside beyond that. Its text can be selected and copied; pressing inside
 * it never opens or closes the row (only the row's line does). It sits in a
 * full-width grid cell of its row. When it is the selected row's and it
 * scrolls, Tab enters it (tabIndex 0, review R-13); Escape or Shift+Tab goes
 * back to the list. The scrolling text is a named region ("Full message"),
 * so screen readers announce its name (code generation review R-06).
 */
export function ExpandedMessage({
  message,
  focusable,
  onMeasure,
  onLeave,
  label,
}: ExpandedMessageProps) {
  const textRef = useRef<HTMLDivElement>(null);
  const measureRef = useRef(onMeasure);

  useEffect(() => {
    measureRef.current = onMeasure;
  }, [onMeasure]);

  useEffect(() => {
    const element = textRef.current;
    if (element === null || typeof ResizeObserver === "undefined") {
      return undefined;
    }
    const observer = new ResizeObserver((entries) => {
      const entry = entries[entries.length - 1];
      const height = entry?.borderBoxSize?.[0]?.blockSize ?? element.getBoundingClientRect().height;
      measureRef.current(height, element.scrollHeight > element.clientHeight + 1);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape" || (event.key === "Tab" && event.shiftKey)) {
      event.preventDefault();
      event.stopPropagation();
      onLeave();
    }
  };

  return (
    <div className="log-expanded" role="gridcell" aria-colspan={3}>
      <div
        ref={textRef}
        className="log-expanded-text"
        role="region"
        tabIndex={focusable ? 0 : -1}
        aria-label={label}
        data-testid="log-table-expanded"
        onKeyDown={handleKeyDown}
      >
        {message}
      </div>
    </div>
  );
}
