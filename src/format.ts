/**
 * Display formatting of log rows. Times are not formatted here: the core sends
 * them already written in the chosen time zone (U4:BR3.4).
 */

/** Replaces line breaks with spaces for a one-line display (BR5.2). */
export function toSingleLine(message: string): string {
  return message.replace(/\r\n|\r|\n/g, " ");
}
