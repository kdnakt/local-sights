import { useEffect } from "react";
import { Mode, applyMode } from "@cloudscape-design/global-styles";

/** The media query that says the OS is in dark mode. */
export const DARK_SCHEME_QUERY = "(prefers-color-scheme: dark)";

/**
 * FR4.5: the screen follows the OS light or dark mode with Cloudscape's own
 * theme. At mount the current `prefers-color-scheme` is applied with
 * `applyMode`, and every later change of the OS setting is applied at once,
 * so no restart is needed. Without `matchMedia` (an old WebView, jsdom) the
 * light mode is applied and nothing is watched.
 */
export function useColorScheme(): void {
  useEffect(() => {
    if (typeof window.matchMedia !== "function") {
      applyMode(Mode.Light);
      return undefined;
    }
    const media = window.matchMedia(DARK_SCHEME_QUERY);
    const apply = () => applyMode(media.matches ? Mode.Dark : Mode.Light);
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, []);
}
