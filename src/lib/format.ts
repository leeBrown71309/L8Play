/** Locale-aware formatting for durations, sizes and dates shown in the library. */

const MINUTE = 60;
const HOUR = 3600;

/** `0` → `"—"`, `2700` → `"45 min"`, `230400` → `"64 h"`. */
export function formatPlaytime(seconds: number): string {
  if (seconds < MINUTE) return "—";
  if (seconds < HOUR) return `${Math.round(seconds / MINUTE)} min`;
  return `${Math.round(seconds / HOUR).toLocaleString()} h`;
}

/** Binary units, one decimal under 100 (`25.1 GB`, `122 GB`). */
export function formatBytes(bytes: number | null, locale: string): string {
  if (bytes === null || bytes <= 0) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = value < 100 && unit > 0 ? 1 : 0;
  return `${value.toLocaleString(locale, { maximumFractionDigits: digits })} ${units[unit]}`;
}

const RELATIVE_STEPS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["year", 365 * 24 * HOUR],
  ["month", 30 * 24 * HOUR],
  ["week", 7 * 24 * HOUR],
  ["day", 24 * HOUR],
  ["hour", HOUR],
  ["minute", MINUTE],
];

/** `"28 minutes ago"`, `"yesterday"`, `"il y a 3 jours"`. */
export function formatRelative(iso: string, locale: string, now: Date = new Date()): string {
  const elapsed = (now.getTime() - new Date(iso).getTime()) / 1000;
  const formatter = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  for (const [unit, size] of RELATIVE_STEPS) {
    if (Math.abs(elapsed) >= size) return formatter.format(-Math.round(elapsed / size), unit);
  }
  return formatter.format(0, "minute");
}

/** `"Wed 4 Nov 2026, 21:12"` in the user's language. */
export function formatDateTime(iso: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    weekday: "short",
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(iso));
}
