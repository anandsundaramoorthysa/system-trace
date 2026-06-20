/**
 * System Trace - shared IPC contract (TypeScript side).
 *
 * This file is one half of the shared contract between the React frontend and
 * the Rust core. Its twin is `src-tauri/src/models.rs`. Every command result and
 * event payload defined here has a matching serde struct there, with identical
 * field names and shapes. When you change one side, change the other.
 *
 * Sources of truth:
 *   - Commands and events: SYSTEM_DESIGN.md section 8.
 *   - Underlying data model: SYSTEM_DESIGN.md section 6.
 *
 * Conventions (must stay in sync with Rust):
 *   - Field names are snake_case on the wire (serde default, matching SQLite
 *     column names), so these TS interfaces use snake_case too. The thin
 *     `lib/api.ts` wrapper is the only place that may translate if we ever want
 *     camelCase in components; for now the contract is snake_case end to end.
 *   - Timestamps are UTC unix milliseconds (number). The UI converts to local
 *     time for display (SYSTEM_DESIGN.md section 6).
 *   - Durations are milliseconds (number), named `*_ms`.
 *   - A "day" key is a local-time 'YYYY-MM-DD' string (daily_app_usage.day).
 *   - Integer ids are `number` (SQLite INTEGER PRIMARY KEY).
 *   - Phase 1 (MVP) types are concrete. Phase 2+ types are declared as hooks so
 *     both sides have a shared shape, but no logic is built yet.
 */

/* ------------------------------------------------------------------ *
 * Scalar aliases (documentation-only; all resolve to number/string)  *
 * ------------------------------------------------------------------ */

/** UTC unix timestamp in milliseconds. */
export type UnixMillis = number;

/** A duration in milliseconds. */
export type Millis = number;

/** Local-time calendar day, formatted 'YYYY-MM-DD'. */
export type DayKey = string;

/** Row id from a SQLite INTEGER PRIMARY KEY. */
export type Id = number;

/* ------------------------------------------------------------------ *
 * Core entities (mirror the SQLite tables in SYSTEM_DESIGN.md s.6)    *
 * ------------------------------------------------------------------ */

/** A category as stored in the `category` table. */
export interface Category {
  id: Id;
  name: string;
  /** Optional hex color for charts (e.g. "#2DD4BF"); null falls back to the default mapping. */
  color: string | null;
  /** Nullable; only meaningful when optional productivity scoring is enabled. */
  productive: boolean | null;
}

/** An app as stored in the `app` table, with its resolved category name for display. */
export interface AppInfo {
  id: Id;
  /** Stable id: exe name on Windows, bundle id on macOS, WM_CLASS on Linux. */
  app_key: string;
  display_name: string;
  /** Null when uncategorized. */
  category_id: Id | null;
  /** Denormalized for convenience in the Apps screen; null when uncategorized. */
  category_name: string | null;
}

/** A single tracked usage row for a screen, joined with display fields. */
export interface UsageEntry {
  app_id: Id;
  app_key: string;
  display_name: string;
  category_id: Id | null;
  category_name: string | null;
  /** Color to render for this entry's category dot/bar (resolved by the core). */
  color: string | null;
  total_ms: Millis;
}

/* ------------------------------------------------------------------ *
 * Dashboard: get_today_overview()                                    *
 * SYSTEM_DESIGN.md s.8:                                              *
 *   -> { total_ms, top_apps[], by_category[], by_hour[] }            *
 * ------------------------------------------------------------------ */

/** One bucket of the "Today, by hour" time-series (24 buckets, hour 0..23). */
export interface HourBucket {
  /** Local hour of day, 0..23. */
  hour: number;
  /** Active milliseconds attributed to this hour. */
  active_ms: Millis;
}

/** Aggregate time for one category, used by donut/legend. */
export interface CategoryUsage {
  category_id: Id | null;
  /** Display name; "Uncategorized" when category_id is null. */
  name: string;
  color: string | null;
  total_ms: Millis;
}

/**
 * The Dashboard hero + cards payload. `delta_vs_yesterday_ms` powers the trend
 * chip (positive = more than yesterday). `active_app_*` reflect the live state
 * at query time; they are also refreshed by the `usage_tick` event.
 */
export interface TodayOverview {
  /** The 'YYYY-MM-DD' this overview is for (today, local time). */
  day: DayKey;
  /** Hero "Screen Time Today" number. */
  total_ms: Millis;
  /** Difference from yesterday's total at the same point; may be negative. */
  delta_vs_yesterday_ms: Millis;
  /** Up to N apps by time today (ordered desc), for the Top Apps card. */
  top_apps: UsageEntry[];
  /** Per-category totals for the split donut/legend. */
  by_category: CategoryUsage[];
  /** 24 buckets (hour 0..23) for the by-hour chart. */
  by_hour: HourBucket[];
  /** App switch count today, for the StatCard. */
  app_switches: number;
  /** Longest single session today, for the StatCard. */
  longest_session_ms: Millis;
  /** App of the longest session; null when there is no usage yet. */
  longest_session_app: string | null;
  /** Currently active app display name from the collector, or null when idle/locked. */
  active_app: string | null;
}

/* ------------------------------------------------------------------ *
 * Reports: get_range_overview(from, to)                              *
 * SYSTEM_DESIGN.md s.8: weekly/monthly aggregates                    *
 * ------------------------------------------------------------------ */

/** Total active time for one calendar day, for the Reports daily-bars chart. */
export interface DayTotal {
  day: DayKey;
  total_ms: Millis;
}

/**
 * Aggregates for an arbitrary inclusive date range (used for Week and Month).
 * `from`/`to` echo the requested local-day bounds.
 */
/** One aggregated search result: time on an app on a given day. */
export interface SearchHit {
  day: DayKey;
  app_key: string;
  display_name: string;
  total_ms: Millis;
  sample_title: string | null;
}

/** A completed manual focus session, optionally annotated. */
export interface FocusSession {
  id: Id;
  start_ms: UnixMillis;
  end_ms: UnixMillis;
  note: string | null;
}

/** A consecutive-days streak for one category goal. */
export interface GoalStreak {
  category_id: Id;
  category_name: string;
  color: string | null;
  kind: GoalKind;
  streak_days: number;
}

/** Result of an in-app database backup. */
export interface BackupResult {
  path: string;
  bytes: number;
}

/** Result of a check against the GitHub releases API (frontend-only). */
export interface UpdateInfo {
  current: string;
  latest: string;
  update_available: boolean;
  url: string;
}

/** Direction of a category goal. */
export type GoalKind = "under" | "over";

/** A daily target for one category, joined with today's usage for display. */
export interface CategoryGoal {
  category_id: Id;
  category_name: string;
  color: string | null;
  daily_ms: Millis;
  kind: GoalKind;
  today_ms: Millis;
}

/** Input for `set_category_goal`. */
export interface CategoryGoalInput {
  category_id: Id;
  daily_ms: Millis;
  kind: GoalKind;
}

/** A daily target for one app, with today's usage and its streak for display. */
export interface AppGoal {
  app_id: Id;
  app_key: string;
  display_name: string;
  daily_ms: Millis;
  kind: GoalKind;
  today_ms: Millis;
  streak_days: number;
}

/** Input for `set_app_goal`. */
export interface AppGoalInput {
  app_id: Id;
  daily_ms: Millis;
  kind: GoalKind;
}

/** Raw icon pixels for an app (RGBA, row-major, top-down). */
export interface AppIcon {
  width: number;
  height: number;
  rgba: number[];
}

/** A productivity Focus Score for a single day. 0..=100. Only surfaced when settings.scoring_enabled. */
export interface FocusScore {
  day: DayKey;
  score: number;
  productive_ms: Millis;
  distracting_ms: Millis;
  neutral_ms: Millis;
}

export interface RangeOverview {
  from: DayKey;
  to: DayKey;
  total_ms: Millis;
  /** Mean active time per day across the range (total_ms / day count). */
  daily_average_ms: Millis;
  /** One entry per day in the range (zero-filled days included). */
  by_day: DayTotal[];
  /** Top apps across the whole range. */
  top_apps: UsageEntry[];
  /** Per-category totals across the whole range. */
  by_category: CategoryUsage[];
  /** The day with the most usage in the range; null when the range is empty. */
  busiest_day: DayKey | null;
  /** Total for the immediately-preceding equal-length range, for delta chips. */
  prev_total_ms: Millis;
}

/* ------------------------------------------------------------------ *
 * Settings: get_settings() / set_setting(key, value)                 *
 * SYSTEM_DESIGN.md s.6: `setting` is a key/value table.              *
 * ------------------------------------------------------------------ */

/**
 * The known settings keys. The `setting` table stores strings, but the contract
 * documents the expected logical type per key so both sides parse consistently.
 */
export type ThemePreference = "system" | "dark" | "light";

/** Summary notification cadence. */
export type SummaryCadence = "off" | "daily" | "weekly" | "both";

/**
 * Typed view of the settings the UI reads. The core returns this resolved object
 * (parsed from the string-valued `setting` rows, with defaults applied) from
 * `get_settings()`. Writes go through `set_setting(key, value)` one key at a time.
 */
export interface Settings {
  theme: ThemePreference;
  /** Idle threshold in seconds (default 120). */
  idle_threshold_secs: number;
  /** Window-title capture; OFF by default. */
  capture_titles: boolean;
  /** Raw `event` retention in days (default 90). Summaries are kept forever. */
  retention_days: number;
  /** Collector paused state. */
  tracking_paused: boolean;
  /** Launch the app at login. */
  launch_at_login: boolean;
  /** Start hidden to the tray. */
  start_minimized: boolean;
  /** Optional productivity scoring; hides the Focus Score when false. */
  scoring_enabled: boolean;
  /** Summary notification cadence (off / daily / weekly / both). */
  summary_cadence: SummaryCadence;
  /** Phase 3: break reminders enabled. */
  breaks_enabled: boolean;
  /** Minutes of continuous active use before a break is due (default 30). */
  break_interval_mins: number;
  /** Break overlay rest suggestion, in seconds (default 20). */
  break_duration_secs: number;
  /** Strict breaks cannot be skipped. */
  break_strict: boolean;
  /** Phase 3: bedtime / wind-down quiet hours enabled. */
  bedtime_enabled: boolean;
  /** Quiet-hours start, "HH:MM" local (default "22:00"). */
  bedtime_start: string;
  /** Quiet-hours end, "HH:MM" local (default "07:00"). */
  bedtime_end: string;
  /** True until onboarding completes; gates the first-run flow. */
  onboarding_complete: boolean;
  /** Phase 3+: emit a calm nudge after N minutes of continuous distracting use. */
  distraction_nudges_enabled: boolean;
  /** How many minutes on a distracting category before the nudge fires. */
  distraction_threshold_mins: number;
  /** Phase 3+: apply a best-effort OS grayscale during quiet hours. */
  bedtime_grayscale_enabled: boolean;
  /** Accent palette name (signal | slate | solar | cocoa). */
  palette: string;
  /** UI language code (e.g. "en"). */
  language: string;
}

/** The string keys accepted by `set_setting`. Keep in sync with `Settings`. */
export type SettingKey =
  | "theme"
  | "idle_threshold_secs"
  | "capture_titles"
  | "retention_days"
  | "tracking_paused"
  | "launch_at_login"
  | "start_minimized"
  | "scoring_enabled"
  | "summary_cadence"
  | "breaks_enabled"
  | "break_interval_mins"
  | "break_duration_secs"
  | "break_strict"
  | "bedtime_enabled"
  | "bedtime_start"
  | "bedtime_end"
  | "onboarding_complete"
  | "distraction_nudges_enabled"
  | "distraction_threshold_mins"
  | "bedtime_grayscale_enabled"
  | "palette"
  | "language";

/** Payload of the `break_due` event. */
export interface BreakDue {
  duration_secs: number;
  strict: boolean;
}

/** Payload of `distraction_nudge`. */
export interface DistractionNudge {
  app_key: string;
  app_name: string;
  mins: number;
}

/* ------------------------------------------------------------------ *
 * Exclusions: get_exclusions() / add_exclusion() / remove_exclusion()*
 * SYSTEM_DESIGN.md s.6: `exclusion(match_type, pattern)`             *
 * ------------------------------------------------------------------ */

/** How an exclusion pattern is matched. */
export type ExclusionMatchType = "app" | "title_contains";

export interface Exclusion {
  id: Id;
  match_type: ExclusionMatchType;
  pattern: string;
}

/** Input shape for `add_exclusion` (no id yet). */
export interface NewExclusion {
  match_type: ExclusionMatchType;
  pattern: string;
}

/* ------------------------------------------------------------------ *
 * Data commands: export / import / wipe                              *
 * ------------------------------------------------------------------ */

export type ExportFormat = "csv" | "json";

/** Result of `export_data`: where it wrote and how much. */
export interface ExportResult {
  path: string;
  format: ExportFormat;
  rows_written: number;
}

/** Result of `import_data`: a summary the UI shows before/after merging. */
export interface ImportResult {
  apps_added: number;
  events_merged: number;
  days_affected: number;
}

/** Result of `wipe_all_data`. */
export interface WipeResult {
  ok: boolean;
}

/* ------------------------------------------------------------------ *
 * Phase 2: limits, blocking, focus (Control). Mirrors models.rs.     *
 * ------------------------------------------------------------------ */

/** How strict a per-app daily limit is when reached. */
export type LimitStrictness = "soft" | "medium" | "strict" | "hard";

/** Argument for `set_limit`. */
export interface LimitInput {
  app_id: Id;
  daily_ms: Millis;
  strictness: LimitStrictness;
}

/** A limit joined with today's usage for display. */
export interface LimitView {
  app_id: Id;
  app_key: string;
  display_name: string;
  daily_ms: Millis;
  used_ms: Millis;
  strictness: LimitStrictness;
  exceeded: boolean;
}

/** What a block rule targets. */
export type BlockKind = "app" | "website";

export interface BlockRule {
  id: Id;
  kind: BlockKind;
  pattern: string;
  enabled: boolean;
  /** When true, the rule only applies inside [schedule_start, schedule_end) (mins since midnight). */
  schedule_enabled: boolean;
  schedule_start: number | null;
  schedule_end: number | null;
}

/** Argument for `set_block_rule` (id null = insert). */
export interface BlockRuleInput {
  id: Id | null;
  kind: BlockKind;
  pattern: string;
  enabled: boolean;
  schedule_enabled: boolean;
  schedule_start: number | null;
  schedule_end: number | null;
}

/** Live focus-mode state. */
export interface FocusState {
  active: boolean;
  ends_at_ms: UnixMillis | null;
  rules_count: number;
}

/* ------------------------------------------------------------------ *
 * Live events (Rust -> UI). SYSTEM_DESIGN.md s.8.                     *
 * ------------------------------------------------------------------ */

/** Tauri event channel names. The UI subscribes; the core emits. */
export const EVENT = {
  /** Throttled (~5s) live update for the hero number and "Active now". */
  USAGE_TICK: "usage_tick",
  /** Phase 2: a per-app daily limit was reached. */
  LIMIT_REACHED: "limit_reached",
  /** Phase 2: a blocked app was opened during focus mode. */
  FOCUS_BLOCKED: "focus_blocked",
  /** Phase 3: a wellbeing break is due. */
  BREAK_DUE: "break_due",
  /** Phase 3: gentle nudge after sustained use of a distracting category. */
  DISTRACTION_NUDGE: "distraction_nudge",
  /** Phase 3: a focus session ended. */
  FOCUS_ENDED: "focus_ended",
} as const;

/** Payload of `limit_reached`. */
export interface LimitReached {
  app_id: Id;
  display_name: string;
  daily_ms: Millis;
  used_ms: Millis;
  strictness: LimitStrictness;
}

/** Payload of `focus_blocked`. */
export interface FocusBlocked {
  app: string;
}

export type EventName = (typeof EVENT)[keyof typeof EVENT];

/** Payload of `usage_tick`: lightweight, no arrays, safe to emit often. */
export interface UsageTick {
  /** Day the tick applies to (local 'YYYY-MM-DD'); UI ignores stale days at rollover. */
  day: DayKey;
  /** Running total for today at emit time. */
  total_ms: Millis;
  /** Active app display name, or null when idle/locked. */
  active_app: string | null;
  /** Collector state, so the UI can show the paused/idle indicator. */
  state: CollectorState;
}

/** Collector run-state, surfaced to the Topbar indicator. */
export type CollectorState = "active" | "idle" | "locked" | "paused";

/* ------------------------------------------------------------------ *
 * Command name registry + typed signatures                           *
 * ------------------------------------------------------------------ */

/**
 * Canonical command names, exactly as registered on the Rust side via
 * `#[tauri::command]` and invoked from `lib/api.ts`. Phase 1 commands are live;
 * Phase 2+ are reserved hooks (declared, not implemented).
 */
export const COMMAND = {
  // Dashboard / Reports
  GET_TODAY_OVERVIEW: "get_today_overview",
  GET_RANGE_OVERVIEW: "get_range_overview",
  GET_DAY_OVERVIEW: "get_day_overview",
  GET_FOCUS_SCORE: "get_focus_score",
  GET_CATEGORY_GOALS: "get_category_goals",
  SET_CATEGORY_GOAL: "set_category_goal",
  REMOVE_CATEGORY_GOAL: "remove_category_goal",
  GET_GOAL_STREAKS: "get_goal_streaks",
  GET_APP_GOALS: "get_app_goals",
  SET_APP_GOAL: "set_app_goal",
  REMOVE_APP_GOAL: "remove_app_goal",
  GET_APP_ICON: "get_app_icon",
  SEARCH_USAGE: "search_usage",
  SAVE_FOCUS_SESSION: "save_focus_session",
  LIST_FOCUS_SESSIONS: "list_focus_sessions",
  BACKUP_DATABASE: "backup_database",
  RESTORE_DATABASE: "restore_database",
  // Apps / Categories
  GET_APPS: "get_apps",
  SET_APP_CATEGORY: "set_app_category",
  GET_CATEGORIES: "get_categories",
  UPSERT_CATEGORY: "upsert_category",
  DELETE_CATEGORY: "delete_category",
  // Settings
  GET_SETTINGS: "get_settings",
  SET_SETTING: "set_setting",
  // Exclusions
  GET_EXCLUSIONS: "get_exclusions",
  ADD_EXCLUSION: "add_exclusion",
  REMOVE_EXCLUSION: "remove_exclusion",
  // Data
  EXPORT_DATA: "export_data",
  IMPORT_DATA: "import_data",
  WIPE_ALL_DATA: "wipe_all_data",
  // Collector control (drives `tracking_paused` + the live indicator)
  GET_COLLECTOR_STATE: "get_collector_state",
  GET_HOTKEY_STATUS: "get_hotkey_status",
  FOCUS_MAIN_WINDOW: "focus_main_window",
  SAVE_REPORT_PDF: "save_report_pdf",
  SET_TRACKING_PAUSED: "set_tracking_paused",
  // Phase 2: limits
  GET_LIMITS: "get_limits",
  SET_LIMIT: "set_limit",
  REMOVE_LIMIT: "remove_limit",
  // Phase 2: blocking
  GET_BLOCK_RULES: "get_block_rules",
  SET_BLOCK_RULE: "set_block_rule",
  REMOVE_BLOCK_RULE: "remove_block_rule",
  // Phase 2: focus mode
  START_FOCUS_SESSION: "start_focus_session",
  STOP_FOCUS_SESSION: "stop_focus_session",
  GET_FOCUS_STATE: "get_focus_state",
  // Phase 4: system-wide website blocking (gated; needs admin)
  APPLY_WEBSITE_BLOCK: "apply_website_block",
  CLEAR_WEBSITE_BLOCK: "clear_website_block",
} as const;

export type CommandName = (typeof COMMAND)[keyof typeof COMMAND];

/** Argument shape for `upsert_category` (id omitted = insert). */
export interface CategoryInput {
  id: Id | null;
  name: string;
  color: string | null;
  productive: boolean | null;
}

/**
 * The full request/response signature for every Phase 1 command. `lib/api.ts`
 * uses this map to give `invoke` end-to-end type safety. Argument objects use the
 * exact parameter names the Rust handlers expect (Tauri passes args by name).
 */
export interface CommandMap {
  [COMMAND.GET_TODAY_OVERVIEW]: {
    args: Record<string, never>;
    result: TodayOverview;
  };
  [COMMAND.GET_RANGE_OVERVIEW]: {
    args: { from: DayKey; to: DayKey };
    result: RangeOverview;
  };
  [COMMAND.GET_DAY_OVERVIEW]: {
    args: { day: DayKey };
    result: TodayOverview;
  };
  [COMMAND.GET_FOCUS_SCORE]: {
    args: Record<string, never>;
    result: FocusScore;
  };
  [COMMAND.GET_CATEGORY_GOALS]: {
    args: Record<string, never>;
    result: CategoryGoal[];
  };
  [COMMAND.SET_CATEGORY_GOAL]: {
    args: { goal: CategoryGoalInput };
    result: void;
  };
  [COMMAND.REMOVE_CATEGORY_GOAL]: {
    args: { category_id: Id };
    result: void;
  };
  [COMMAND.GET_GOAL_STREAKS]: {
    args: Record<string, never>;
    result: GoalStreak[];
  };
  [COMMAND.GET_APP_GOALS]: {
    args: Record<string, never>;
    result: AppGoal[];
  };
  [COMMAND.SET_APP_GOAL]: {
    args: { goal: AppGoalInput };
    result: void;
  };
  [COMMAND.REMOVE_APP_GOAL]: {
    args: { app_id: Id };
    result: void;
  };
  [COMMAND.SEARCH_USAGE]: {
    args: { query: string; from: DayKey | null; to: DayKey | null };
    result: SearchHit[];
  };
  [COMMAND.SAVE_FOCUS_SESSION]: {
    args: { start_ms: UnixMillis; end_ms: UnixMillis; note: string };
    result: void;
  };
  [COMMAND.LIST_FOCUS_SESSIONS]: {
    args: { limit: number };
    result: FocusSession[];
  };
  [COMMAND.BACKUP_DATABASE]: {
    args: { path: string };
    result: BackupResult;
  };
  [COMMAND.RESTORE_DATABASE]: {
    args: { path: string };
    result: void;
  };
  [COMMAND.GET_APPS]: {
    args: Record<string, never>;
    result: AppInfo[];
  };
  [COMMAND.SET_APP_CATEGORY]: {
    args: { app_id: Id; category_id: Id | null };
    result: void;
  };
  [COMMAND.GET_CATEGORIES]: {
    args: Record<string, never>;
    result: Category[];
  };
  [COMMAND.UPSERT_CATEGORY]: {
    args: { category: CategoryInput };
    result: Category;
  };
  [COMMAND.DELETE_CATEGORY]: {
    args: { id: Id };
    result: void;
  };
  [COMMAND.GET_SETTINGS]: {
    args: Record<string, never>;
    result: Settings;
  };
  [COMMAND.SET_SETTING]: {
    args: { key: SettingKey; value: string };
    result: void;
  };
  [COMMAND.GET_EXCLUSIONS]: {
    args: Record<string, never>;
    result: Exclusion[];
  };
  [COMMAND.ADD_EXCLUSION]: {
    args: { exclusion: NewExclusion };
    result: Exclusion;
  };
  [COMMAND.REMOVE_EXCLUSION]: {
    args: { id: Id };
    result: void;
  };
  [COMMAND.EXPORT_DATA]: {
    args: { format: ExportFormat; path: string; from: DayKey | null; to: DayKey | null };
    result: ExportResult;
  };
  [COMMAND.IMPORT_DATA]: {
    args: { path: string };
    result: ImportResult;
  };
  [COMMAND.WIPE_ALL_DATA]: {
    args: Record<string, never>;
    result: WipeResult;
  };
  [COMMAND.GET_COLLECTOR_STATE]: {
    args: Record<string, never>;
    result: CollectorState;
  };
  [COMMAND.SET_TRACKING_PAUSED]: {
    args: { paused: boolean };
    result: CollectorState;
  };
  [COMMAND.GET_LIMITS]: {
    args: Record<string, never>;
    result: LimitView[];
  };
  [COMMAND.SET_LIMIT]: {
    args: { limit: LimitInput };
    result: void;
  };
  [COMMAND.REMOVE_LIMIT]: {
    args: { app_id: Id };
    result: void;
  };
  [COMMAND.GET_BLOCK_RULES]: {
    args: Record<string, never>;
    result: BlockRule[];
  };
  [COMMAND.SET_BLOCK_RULE]: {
    args: { rule: BlockRuleInput };
    result: BlockRule;
  };
  [COMMAND.REMOVE_BLOCK_RULE]: {
    args: { id: Id };
    result: void;
  };
  [COMMAND.START_FOCUS_SESSION]: {
    args: { minutes: number };
    result: FocusState;
  };
  [COMMAND.STOP_FOCUS_SESSION]: {
    args: Record<string, never>;
    result: FocusState;
  };
  [COMMAND.GET_FOCUS_STATE]: {
    args: Record<string, never>;
    result: FocusState;
  };
  [COMMAND.APPLY_WEBSITE_BLOCK]: {
    args: Record<string, never>;
    result: number;
  };
  [COMMAND.CLEAR_WEBSITE_BLOCK]: {
    args: Record<string, never>;
    result: void;
  };
}
