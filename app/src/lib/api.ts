/**
 * Typed bridge to the Rust core. In Tauri it calls real commands via `invoke`;
 * in a plain browser it returns mock data so the UI renders for design and tests.
 * Command and argument names match the shared contract in `types.ts` exactly.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  COMMAND,
  EVENT,
  type AppInfo,
  type BlockRule,
  type BlockRuleInput,
  type BreakDue,
  type DistractionNudge,
  type LimitReached,
  type BackupResult,
  type Category,
  type CategoryGoal,
  type CategoryGoalInput,
  type AppGoal,
  type AppGoalInput,
  type AppIcon,
  type CategoryInput,
  type FocusSession,
  type GoalStreak,
  type SearchHit,
  type CollectorState,
  type Exclusion,
  type ExportFormat,
  type ExportResult,
  type FocusScore,
  type FocusState,
  type ImportResult,
  type LimitInput,
  type LimitView,
  type NewExclusion,
  type RangeOverview,
  type Settings,
  type SettingKey,
  type TodayOverview,
  type UsageTick,
  type WipeResult,
} from "./types";
import {
  mockApps,
  mockBlockRules,
  mockCategories,
  mockExclusions,
  mockFocusState,
  mockLimits,
  mockRange,
  mockSettings,
  mockToday,
} from "./mock";

/** True when running inside the Tauri webview (real backend available). */
export const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/* ------------------------------ dashboard --------------------------------- */

export function getTodayOverview(): Promise<TodayOverview> {
  if (!isTauri) return Promise.resolve(mockToday());
  return invoke(COMMAND.GET_TODAY_OVERVIEW);
}

export function getRangeOverview(from: string, to: string): Promise<RangeOverview> {
  if (!isTauri) return Promise.resolve(mockRange(from, to));
  return invoke(COMMAND.GET_RANGE_OVERVIEW, { from, to });
}

export function getDayOverview(day: string): Promise<TodayOverview> {
  if (!isTauri) return Promise.resolve(mockToday());
  return invoke(COMMAND.GET_DAY_OVERVIEW, { day });
}

export function getCategoryGoals(): Promise<CategoryGoal[]> {
  if (!isTauri) return Promise.resolve([]);
  return invoke(COMMAND.GET_CATEGORY_GOALS);
}

export function getGoalStreaks(): Promise<GoalStreak[]> {
  if (!isTauri) return Promise.resolve([]);
  return invoke(COMMAND.GET_GOAL_STREAKS);
}

export function searchUsage(
  query: string,
  from: string | null,
  to: string | null,
): Promise<SearchHit[]> {
  if (!isTauri) return Promise.resolve([]);
  return invoke(COMMAND.SEARCH_USAGE, { query, from, to });
}

export function saveFocusSession(
  start_ms: number,
  end_ms: number,
  note: string,
): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.SAVE_FOCUS_SESSION, { start_ms, end_ms, note });
}

export function listFocusSessions(limit: number): Promise<FocusSession[]> {
  if (!isTauri) return Promise.resolve([]);
  return invoke(COMMAND.LIST_FOCUS_SESSIONS, { limit });
}

export function backupDatabase(path: string): Promise<BackupResult> {
  if (!isTauri) return Promise.resolve({ path, bytes: 0 });
  return invoke(COMMAND.BACKUP_DATABASE, { path });
}

export function restoreDatabase(path: string): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.RESTORE_DATABASE, { path });
}

export function setCategoryGoal(goal: CategoryGoalInput): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.SET_CATEGORY_GOAL, { goal });
}

export function removeCategoryGoal(category_id: number): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.REMOVE_CATEGORY_GOAL, { category_id });
}

export function getAppGoals(): Promise<AppGoal[]> {
  if (!isTauri) return Promise.resolve([]);
  return invoke(COMMAND.GET_APP_GOALS);
}

export function setAppGoal(goal: AppGoalInput): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.SET_APP_GOAL, { goal });
}

export function removeAppGoal(app_id: number): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.REMOVE_APP_GOAL, { app_id });
}

/** The app's real OS icon (raw RGBA), or null to fall back to a letter avatar. */
export function getAppIcon(app_key: string): Promise<AppIcon | null> {
  if (!isTauri) return Promise.resolve(null);
  return invoke(COMMAND.GET_APP_ICON, { app_key });
}

export function getFocusScore(): Promise<FocusScore> {
  if (!isTauri)
    return Promise.resolve({
      day: new Date().toISOString().slice(0, 10),
      score: 72,
      productive_ms: 90 * 60_000,
      distracting_ms: 35 * 60_000,
      neutral_ms: 20 * 60_000,
    });
  return invoke(COMMAND.GET_FOCUS_SCORE);
}

/* --------------------------- apps + categories ---------------------------- */

export function getApps(): Promise<AppInfo[]> {
  if (!isTauri) return Promise.resolve(mockApps());
  return invoke(COMMAND.GET_APPS);
}

export function setAppCategory(app_id: number, category_id: number | null): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.SET_APP_CATEGORY, { app_id, category_id });
}

export function getCategories(): Promise<Category[]> {
  if (!isTauri) return Promise.resolve(mockCategories());
  return invoke(COMMAND.GET_CATEGORIES);
}

export function upsertCategory(category: CategoryInput): Promise<Category> {
  if (!isTauri)
    return Promise.resolve({
      id: category.id ?? Math.floor(Math.random() * 1e6),
      name: category.name,
      color: category.color,
      productive: category.productive,
    });
  return invoke(COMMAND.UPSERT_CATEGORY, { category });
}

export function deleteCategory(id: number): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.DELETE_CATEGORY, { id });
}

/* -------------------------------- settings -------------------------------- */

export function getSettings(): Promise<Settings> {
  if (!isTauri) return Promise.resolve(mockSettings());
  return invoke(COMMAND.GET_SETTINGS);
}

export function setSetting(key: SettingKey, value: string): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.SET_SETTING, { key, value });
}

/* ------------------------------- exclusions ------------------------------- */

export function getExclusions(): Promise<Exclusion[]> {
  if (!isTauri) return Promise.resolve(mockExclusions());
  return invoke(COMMAND.GET_EXCLUSIONS);
}

export function addExclusion(exclusion: NewExclusion): Promise<Exclusion> {
  if (!isTauri)
    return Promise.resolve({ id: Math.floor(Math.random() * 1e6), ...exclusion });
  return invoke(COMMAND.ADD_EXCLUSION, { exclusion });
}

export function removeExclusion(id: number): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.REMOVE_EXCLUSION, { id });
}

/* ------------------------------ data commands ----------------------------- */

export function exportData(
  format: ExportFormat,
  path: string,
  from: string | null = null,
  to: string | null = null,
): Promise<ExportResult> {
  if (!isTauri) return Promise.resolve({ path, format, rows_written: 0 });
  return invoke(COMMAND.EXPORT_DATA, { format, path, from, to });
}

export function importData(path: string): Promise<ImportResult> {
  if (!isTauri)
    return Promise.resolve({ apps_added: 0, events_merged: 0, days_affected: 0 });
  return invoke(COMMAND.IMPORT_DATA, { path });
}

export function wipeAllData(): Promise<WipeResult> {
  if (!isTauri) return Promise.resolve({ ok: true });
  return invoke(COMMAND.WIPE_ALL_DATA);
}

/* --------------------------- collector control ---------------------------- */

export function getCollectorState(): Promise<CollectorState> {
  if (!isTauri) return Promise.resolve("active");
  return invoke(COMMAND.GET_COLLECTOR_STATE);
}

/** Whether the global pause/resume hotkey registered (false = chord taken). */
export function getHotkeyStatus(): Promise<boolean> {
  if (!isTauri) return Promise.resolve(true);
  return invoke(COMMAND.GET_HOTKEY_STATUS);
}

/** Bring the main window to the foreground (used on notification click). */
export function focusMainWindow(): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.FOCUS_MAIN_WINDOW);
}

/** Write a generated PDF (raw bytes) to a user-chosen path. Returns bytes written. */
export function saveReportPdf(path: string, bytes: number[]): Promise<number> {
  if (!isTauri) return Promise.resolve(0);
  return invoke(COMMAND.SAVE_REPORT_PDF, { path, bytes });
}

export function setTrackingPaused(paused: boolean): Promise<CollectorState> {
  if (!isTauri) return Promise.resolve(paused ? "paused" : "active");
  return invoke(COMMAND.SET_TRACKING_PAUSED, { paused });
}

/* ----------------------------- phase 2: limits ---------------------------- */

export function getLimits(): Promise<LimitView[]> {
  if (!isTauri) return Promise.resolve(mockLimits());
  return invoke(COMMAND.GET_LIMITS);
}

export function setLimit(limit: LimitInput): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.SET_LIMIT, { limit });
}

export function removeLimit(app_id: number): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.REMOVE_LIMIT, { app_id });
}

/* ---------------------------- phase 2: blocking --------------------------- */

export function getBlockRules(): Promise<BlockRule[]> {
  if (!isTauri) return Promise.resolve(mockBlockRules());
  return invoke(COMMAND.GET_BLOCK_RULES);
}

export function setBlockRule(rule: BlockRuleInput): Promise<BlockRule> {
  if (!isTauri)
    return Promise.resolve({ ...rule, id: rule.id ?? Math.floor(Math.random() * 1e6) });
  return invoke(COMMAND.SET_BLOCK_RULE, { rule });
}

export function removeBlockRule(id: number): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.REMOVE_BLOCK_RULE, { id });
}

/* ------------------------------ phase 2: focus ---------------------------- */

export function getFocusState(): Promise<FocusState> {
  if (!isTauri) return Promise.resolve(mockFocusState());
  return invoke(COMMAND.GET_FOCUS_STATE);
}

export function startFocusSession(minutes: number): Promise<FocusState> {
  if (!isTauri)
    return Promise.resolve({
      active: true,
      ends_at_ms: minutes > 0 ? Date.now() + minutes * 60_000 : null,
      rules_count: mockBlockRules().filter((r) => r.enabled).length,
    });
  return invoke(COMMAND.START_FOCUS_SESSION, { minutes });
}

export function stopFocusSession(): Promise<FocusState> {
  if (!isTauri)
    return Promise.resolve({
      active: false,
      ends_at_ms: null,
      rules_count: mockBlockRules().filter((r) => r.enabled).length,
    });
  return invoke(COMMAND.STOP_FOCUS_SESSION);
}

/** Apply enabled website block rules to the hosts file. Needs admin rights. */
export function applyWebsiteBlock(): Promise<number> {
  if (!isTauri) return Promise.resolve(0);
  return invoke(COMMAND.APPLY_WEBSITE_BLOCK);
}

/** Remove System Trace's managed hosts-file block. Needs admin rights. */
export function clearWebsiteBlock(): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke(COMMAND.CLEAR_WEBSITE_BLOCK);
}

/**
 * Whether this is the Microsoft Store edition, where system-wide website
 * blocking is compiled out (it needs admin rights and edits the hosts file,
 * which Store policy disallows). The UI uses this to disable those controls.
 */
export function isStoreBuild(): Promise<boolean> {
  if (!isTauri) return Promise.resolve(false);
  return invoke(COMMAND.IS_STORE_BUILD);
}

/* --------------------------------- events --------------------------------- */

/** Subscribe to the throttled live `usage_tick`. Returns an unlisten function. */
export async function onUsageTick(
  cb: (tick: UsageTick) => void,
): Promise<UnlistenFn> {
  if (!isTauri) return () => {};
  return listen<UsageTick>(EVENT.USAGE_TICK, (e) => cb(e.payload));
}

/** Subscribe to `break_due`. Returns an unlisten function. */
export async function onBreakDue(cb: (b: BreakDue) => void): Promise<UnlistenFn> {
  if (!isTauri) return () => {};
  return listen<BreakDue>(EVENT.BREAK_DUE, (e) => cb(e.payload));
}

/** Subscribe to `distraction_nudge`. Returns an unlisten function. */
export async function onDistractionNudge(
  cb: (n: DistractionNudge) => void,
): Promise<UnlistenFn> {
  if (!isTauri) return () => {};
  return listen<DistractionNudge>(EVENT.DISTRACTION_NUDGE, (e) => cb(e.payload));
}

/** Subscribe to `limit_reached`. Returns an unlisten function. */
export async function onLimitReached(
  cb: (l: LimitReached) => void,
): Promise<UnlistenFn> {
  if (!isTauri) return () => {};
  return listen<LimitReached>(EVENT.LIMIT_REACHED, (e) => cb(e.payload));
}
