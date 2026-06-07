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
  type Category,
  type CategoryInput,
  type CollectorState,
  type Exclusion,
  type ExportFormat,
  type ExportResult,
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

export function exportData(format: ExportFormat, path: string): Promise<ExportResult> {
  if (!isTauri) return Promise.resolve({ path, format, rows_written: 0 });
  return invoke(COMMAND.EXPORT_DATA, { format, path });
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
