import { useRef, type KeyboardEvent, type PointerEvent } from "react";
import {
  PANE_WIDTH_MAX_PERCENT,
  PANE_WIDTH_MIN_PERCENT,
  PANE_WIDTH_STEP_PERCENT,
  clampPaneWidth,
} from "../hooks/usePaneWidth";
import type { Translate } from "../i18n/messages";

export interface PaneResizerProps {
  /** The current width of the pane on the left, in percent of the app body. */
  value: number;
  /** Called with the new width, in percent, while dragging and on every key press. */
  onChange: (width: number) => void;
  t: Translate;
}

/** The fallback width when nothing is laid out (jsdom, a detached element). */
const FALLBACK_CONTAINER_WIDTH_PX = 1024;

/**
 * The width in pixels that 100% stands for: the handle's parent (`.app-body`,
 * the flex container whose width the pane's percentage refers to), else the
 * window.
 */
function containerWidthInPixels(handle: HTMLElement): number {
  const parentWidth = handle.parentElement?.clientWidth ?? 0;
  if (parentWidth > 0) {
    return parentWidth;
  }
  return window.innerWidth > 0 ? window.innerWidth : FALLBACK_CONTAINER_WIDTH_PX;
}

/**
 * The vertical handle between the log group pane and the right pane (gate
 * fix 14d). Dragging it with the mouse or a finger changes the width of the
 * pane on its left; with the focus on it, Left and Right move by one percent,
 * Home and End go to the narrowest and the widest (gate fix 15b: the width
 * is a share of the app body, 20% to 50%). It is a separator with the ARIA
 * values of a slider (`aria-valuenow` in percent), so a screen reader
 * announces the width (accessibility K-series). The width itself is applied
 * by the parent through the `--ls-log-group-pane-width` custom property; the
 * parent does not render the handle while the pane is collapsed.
 */
export function PaneResizer({ value, onChange, t }: PaneResizerProps) {
  const drag = useRef<{
    pointerId: number;
    startX: number;
    startWidth: number;
    containerWidth: number;
  } | null>(null);

  const handlePointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) {
      return;
    }
    drag.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startWidth: value,
      containerWidth: containerWidthInPixels(event.currentTarget),
    };
    event.currentTarget.setPointerCapture?.(event.pointerId);
    event.preventDefault();
  };

  const handlePointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const current = drag.current;
    if (current === null || current.pointerId !== event.pointerId) {
      return;
    }
    const deltaPercent = ((event.clientX - current.startX) / current.containerWidth) * 100;
    onChange(clampPaneWidth(current.startWidth + deltaPercent));
  };

  const handlePointerEnd = (event: PointerEvent<HTMLDivElement>) => {
    if (drag.current?.pointerId === event.pointerId) {
      drag.current = null;
      event.currentTarget.releasePointerCapture?.(event.pointerId);
    }
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    let next: number;
    switch (event.key) {
      case "ArrowLeft":
        next = value - PANE_WIDTH_STEP_PERCENT;
        break;
      case "ArrowRight":
        next = value + PANE_WIDTH_STEP_PERCENT;
        break;
      case "Home":
        next = PANE_WIDTH_MIN_PERCENT;
        break;
      case "End":
        next = PANE_WIDTH_MAX_PERCENT;
        break;
      default:
        return;
    }
    event.preventDefault();
    onChange(clampPaneWidth(next));
  };

  return (
    <div
      role="separator"
      tabIndex={0}
      aria-orientation="vertical"
      aria-label={t("pane.resizer")}
      aria-valuenow={Math.round(value)}
      aria-valuemin={PANE_WIDTH_MIN_PERCENT}
      aria-valuemax={PANE_WIDTH_MAX_PERCENT}
      aria-valuetext={`${Math.round(value)}%`}
      className="pane-resizer"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerEnd}
      onPointerCancel={handlePointerEnd}
      onKeyDown={handleKeyDown}
      data-testid="pane-resizer"
    />
  );
}
