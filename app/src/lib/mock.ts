/**
 * Mock data for running the UI in a plain browser (no Tauri backend), used by
 * `lib/api.ts` when `isTauri` is false. This keeps `pnpm dev` and the build
 * usable for design work and Playwright UI tests without launching the Rust core.
 * It is never bundled into behavior that ships to users beyond the dev fallback.
 */

import type {
  AppInfo,
  BlockRule,
  Category,
  CategoryUsage,
  Exclusion,
  FocusState,
  HourBucket,
  LimitView,
  RangeOverview,
  Settings,
  TodayOverview,
  UsageEntry,
} from "./types";
import { dayKeyOffset, toDayKey } from "./format";

const CATEGORIES: Category[] = [
  { id: 1, name: "Work", color: "#2DD4BF", productive: null },
  { id: 2, name: "Communication", color: "#0EA5A0", productive: null },
  { id: 3, name: "Development", color: "#34D399", productive: null },
  { id: 4, name: "Social", color: "#F59E0B", productive: null },
  { id: 5, name: "Entertainment", color: "#F87171", productive: null },
  { id: 6, name: "Reading", color: "#8B949E", productive: null },
];

const APPS: AppInfo[] = [
  { id: 1, app_key: "code.exe", display_name: "code", category_id: 3, category_name: "Development" },
  { id: 2, app_key: "chrome.exe", display_name: "chrome", category_id: 4, category_name: "Social" },
  { id: 3, app_key: "slack.exe", display_name: "slack", category_id: 2, category_name: "Communication" },
  { id: 4, app_key: "figma.exe", display_name: "figma", category_id: 1, category_name: "Work" },
  { id: 5, app_key: "spotify.exe", display_name: "spotify", category_id: 5, category_name: "Entertainment" },
  { id: 6, app_key: "notion.exe", display_name: "notion", category_id: 6, category_name: "Reading" },
];

const MIN = 60_000;

function colorFor(categoryId: number | null): string | null {
  return CATEGORIES.find((c) => c.id === categoryId)?.color ?? null;
}

function entry(app: AppInfo, totalMs: number): UsageEntry {
  return {
    app_id: app.id,
    app_key: app.app_key,
    display_name: app.display_name,
    category_id: app.category_id,
    category_name: app.category_name,
    color: colorFor(app.category_id),
    total_ms: totalMs,
  };
}

const TOP_TODAY: UsageEntry[] = [
  entry(APPS[0], 132 * MIN),
  entry(APPS[1], 96 * MIN),
  entry(APPS[2], 54 * MIN),
  entry(APPS[3], 38 * MIN),
  entry(APPS[4], 27 * MIN),
  entry(APPS[5], 14 * MIN),
];

function byCategory(entries: UsageEntry[]): CategoryUsage[] {
  const map = new Map<number | null, CategoryUsage>();
  for (const e of entries) {
    const key = e.category_id;
    const prev = map.get(key);
    if (prev) prev.total_ms += e.total_ms;
    else
      map.set(key, {
        category_id: key,
        name: e.category_name ?? "Uncategorized",
        color: e.color,
        total_ms: e.total_ms,
      });
  }
  return [...map.values()].sort((a, b) => b.total_ms - a.total_ms);
}

function byHour(): HourBucket[] {
  const shape = [
    0, 0, 0, 0, 0, 0, 2, 8, 22, 35, 41, 30, 18, 33, 44, 38, 26, 20, 14, 24, 30,
    16, 6, 1,
  ];
  return shape.map((m, hour) => ({ hour, active_ms: m * MIN }));
}

export function mockToday(): TodayOverview {
  const total = TOP_TODAY.reduce((s, e) => s + e.total_ms, 0);
  return {
    day: toDayKey(new Date()),
    total_ms: total,
    delta_vs_yesterday_ms: 38 * MIN,
    top_apps: TOP_TODAY,
    by_category: byCategory(TOP_TODAY),
    by_hour: byHour(),
    app_switches: 187,
    longest_session_ms: 52 * MIN,
    longest_session_app: "code",
    active_app: "code",
  };
}

export function mockRange(from: string, to: string): RangeOverview {
  const start = new Date(from);
  const end = new Date(to);
  const by_day = [];
  let total = 0;
  const samples = [180, 240, 95, 300, 210, 60, 140];
  let i = 0;
  for (let d = new Date(start); d <= end; d.setDate(d.getDate() + 1)) {
    const ms = samples[i % samples.length] * MIN;
    by_day.push({ day: toDayKey(d), total_ms: ms });
    total += ms;
    i++;
  }
  const busiest = [...by_day].sort((a, b) => b.total_ms - a.total_ms)[0];
  return {
    from,
    to,
    total_ms: total,
    daily_average_ms: by_day.length ? Math.round(total / by_day.length) : 0,
    by_day,
    top_apps: TOP_TODAY,
    by_category: byCategory(TOP_TODAY),
    busiest_day: busiest?.day ?? null,
    prev_total_ms: Math.round(total * 0.86),
  };
}

export function mockApps(): AppInfo[] {
  return APPS;
}

export function mockCategories(): Category[] {
  return CATEGORIES;
}

export function mockSettings(): Settings {
  return {
    theme: "system",
    idle_threshold_secs: 120,
    capture_titles: false,
    retention_days: 90,
    tracking_paused: false,
    launch_at_login: false,
    start_minimized: false,
    scoring_enabled: false,
    summary_cadence: "daily",
    breaks_enabled: false,
    break_interval_mins: 30,
    break_duration_secs: 20,
    break_strict: false,
    bedtime_enabled: false,
    bedtime_start: "22:00",
    bedtime_end: "07:00",
    onboarding_complete: true,
  };
}

export function mockExclusions(): Exclusion[] {
  return [{ id: 1, match_type: "app", pattern: "1password.exe" }];
}

export function mockLimits(): LimitView[] {
  return [
    {
      app_id: 2,
      app_key: "chrome.exe",
      display_name: "chrome",
      daily_ms: 60 * MIN,
      used_ms: 96 * MIN,
      strictness: "medium",
      exceeded: true,
    },
    {
      app_id: 5,
      app_key: "spotify.exe",
      display_name: "spotify",
      daily_ms: 60 * MIN,
      used_ms: 27 * MIN,
      strictness: "soft",
      exceeded: false,
    },
  ];
}

export function mockBlockRules(): BlockRule[] {
  return [
    { id: 1, kind: "app", pattern: "game.exe", enabled: true },
    { id: 2, kind: "website", pattern: "reddit.com", enabled: true },
  ];
}

export function mockFocusState(): FocusState {
  return { active: false, ends_at_ms: null, rules_count: 2 };
}

export const MOCK_RANGE_DEFAULT = { from: dayKeyOffset(6), to: dayKeyOffset(0) };
