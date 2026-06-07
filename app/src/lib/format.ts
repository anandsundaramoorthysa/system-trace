/** Small, pure formatting helpers shared across screens. */

import type { Millis, DayKey } from "./types";

/** "2h 14m", "47m", "<1m", "0m". Durations are milliseconds. */
export function formatDuration(ms: Millis): string {
  if (ms <= 0) return "0m";
  const totalMin = Math.floor(ms / 60000);
  if (totalMin < 1) return "<1m";
  const h = Math.floor(totalMin / 60);
  const m = totalMin % 60;
  if (h === 0) return `${m}m`;
  if (m === 0) return `${h}h`;
  return `${h}h ${m}m`;
}

/** Compact "2h", "47m" for axis labels. */
export function formatDurationShort(ms: Millis): string {
  if (ms <= 0) return "0";
  const totalMin = Math.round(ms / 60000);
  if (totalMin < 60) return `${totalMin}m`;
  return `${Math.round((totalMin / 60) * 10) / 10}h`;
}

/** Signed delta, e.g. "+38m" / "-1h 5m" / "even". */
export function formatDelta(ms: Millis): string {
  if (ms === 0) return "even";
  const sign = ms > 0 ? "+" : "-";
  return sign + formatDuration(Math.abs(ms));
}

/** Local short weekday + day-of-month from a 'YYYY-MM-DD' key, e.g. "Mon 3". */
export function formatDayLabel(day: DayKey): string {
  const d = parseDayKey(day);
  return d.toLocaleDateString(undefined, { weekday: "short", day: "numeric" });
}

/** 'YYYY-MM-DD' -> a local Date at midnight. */
export function parseDayKey(day: DayKey): Date {
  const [y, m, d] = day.split("-").map((n) => parseInt(n, 10));
  return new Date(y, (m || 1) - 1, d || 1);
}

/** A local Date -> 'YYYY-MM-DD'. */
export function toDayKey(d: Date): DayKey {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

/** today/yesterday/N days ago as a 'YYYY-MM-DD' key. */
export function dayKeyOffset(days: number): DayKey {
  const d = new Date();
  d.setDate(d.getDate() - days);
  return toDayKey(d);
}
