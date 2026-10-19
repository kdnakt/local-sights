/**
 * Which modifier key the keyboard shortcuts of the screen use
 * (261017-insights-query U1:BR1.1, BR1.6): Cmd on macOS, Ctrl elsewhere
 * (Windows 11). The answer comes from the browser's platform string; nothing
 * else about the machine is read. Every function here is pure.
 */

/** The parts of `KeyboardEvent` the shortcuts look at. */
export interface ModifierKeys {
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

/** Whether `platform` (`navigator.platform` or the user agent) names a Mac. */
export function isMacPlatform(platform: string): boolean {
  return /mac/i.test(platform);
}

/** The platform string of this browser, empty when there is none. */
export function currentPlatform(): string {
  if (typeof navigator === "undefined") {
    return "";
  }
  return navigator.platform || navigator.userAgent || "";
}

/**
 * Whether only the shortcut modifier is held: Cmd on a Mac, Ctrl elsewhere,
 * without Alt or Shift.
 */
export function isShortcutModifier(event: ModifierKeys, mac: boolean): boolean {
  if (event.altKey || event.shiftKey) {
    return false;
  }
  return mac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
}
