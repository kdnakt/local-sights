import { useEffect, useState } from "react";

/**
 * The value as it was once it stopped changing for `delayMs` milliseconds
 * (U5:BR1.3). Every change starts the wait again, so only the last value of a
 * burst comes through.
 */
export function useDebouncedValue<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value);

  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(timer);
  }, [value, delayMs]);

  return debounced;
}
