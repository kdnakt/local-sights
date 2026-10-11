import { useCallback, useEffect, useRef, useState } from "react";

/** Anything Cloudscape hands out as a ref: a handle with `focus()`. */
export interface Focusable {
  focus(): void;
}

/**
 * Focuses a Cloudscape control once it is mounted. A Cloudscape Modal
 * renders its content through a portal one commit after the dialog itself,
 * and its focus lock then focuses the first focusable element (the close
 * button); a plain mount effect would run too early or be overridden. The
 * callback ref returned here records the control's handle in state, so the
 * focus is set in the commit after the control appeared, after the focus
 * lock, and only once (U2:BR2.6, U3:BR6.8, U6:BR5.1, U7:BR3.3: the dialogs
 * start on the safe button or on the checkbox).
 */
export function useInitialFocus(): (handle: Focusable | null) => void {
  const [handle, setHandle] = useState<Focusable | null>(null);
  const focused = useRef(false);
  useEffect(() => {
    if (handle !== null && !focused.current) {
      focused.current = true;
      handle.focus();
    }
  }, [handle]);
  return useCallback((next: Focusable | null) => setHandle(next), []);
}
