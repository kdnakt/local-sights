import { useEffect } from "react";

/**
 * Calls `onEscape` when Escape is pressed anywhere in the window (BR6.2).
 * Used to dismiss transient panels such as the error banner; dialogs added in
 * later units use the same hook.
 */
export function useEscapeKey(onEscape: () => void, enabled = true): void {
  useEffect(() => {
    if (!enabled) {
      return undefined;
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onEscape();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onEscape, enabled]);
}
