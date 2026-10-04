/** Display formatting of log rows. */

/** UTC `yyyy-mm-dd hh:mm:ss.mmm` (BR5.3); the raw number if out of range. */
export function formatUtcMillis(timestamp: number): string {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) {
    return String(timestamp);
  }
  const iso = date.toISOString();
  // A year beyond 9999 renders as "+010000-..."; keep the raw number then.
  if (!/^\d{4}-/.test(iso)) {
    return String(timestamp);
  }
  return iso.slice(0, 23).replace("T", " ");
}

/** Replaces line breaks with spaces for a one-line display (BR5.2). */
export function toSingleLine(message: string): string {
  return message.replace(/\r\n|\r|\n/g, " ");
}
