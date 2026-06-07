//! System Trace - shared IPC contract (Rust side).
//!
//! This module is one half of the shared contract between the Rust core and the
//! React frontend. Its twin is `app/src/lib/types.ts`. Every struct here that is
//! returned from a `#[tauri::command]` serializes to JSON whose field names and
//! shapes exactly match a TypeScript interface there. When you change one side,
//! change the other.
//!
//! Sources of truth:
//!   - Commands and events: `docs/SYSTEM_DESIGN.md` section 8.
//!   - Underlying data model: `docs/SYSTEM_DESIGN.md` section 6.
//!
//! Conventions (must stay in sync with TypeScript):
//!   - Wire format is snake_case (serde default), matching the SQLite column
//!     names and the TS interfaces.
//!   - Timestamps are UTC unix milliseconds (`i64`). The UI converts to local
//!     time for display.
//!   - Durations are milliseconds (`i64`), named `*_ms`.
//!   - A "day" key is a local-time 'YYYY-MM-DD' `String` (daily_app_usage.day).
//!   - Integer ids are `i64` (SQLite INTEGER PRIMARY KEY).
//!   - `Option<T>` maps to `T | null` in TS.
//!   - Phase 1 (MVP) types are concrete. Phase 2+ types are declared as hooks so
//!     both sides share a shape; no logic is built yet.
//!
//! No business logic lives here. This is the contract only.

use serde::{Deserialize, Serialize};

/* ------------------------------------------------------------------ *
 * Core entities (mirror the SQLite tables in SYSTEM_DESIGN.md s.6)    *
 * ------------------------------------------------------------------ */

/// A category as stored in the `category` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    /// Optional hex color for charts (e.g. "#2DD4BF"); `None` -> default mapping.
    pub color: Option<String>,
    /// Nullable; only meaningful when optional productivity scoring is enabled.
    pub productive: Option<bool>,
}

/// An app as stored in the `app` table, with its resolved category name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub id: i64,
    /// Stable id: exe name (Windows), bundle id (macOS), WM_CLASS (Linux).
    pub app_key: String,
    pub display_name: String,
    /// `None` when uncategorized.
    pub category_id: Option<i64>,
    /// Denormalized for the Apps screen; `None` when uncategorized.
    pub category_name: Option<String>,
}

/// A single tracked usage row for a screen, joined with display fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEntry {
    pub app_id: i64,
    pub app_key: String,
    pub display_name: String,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    /// Resolved color for this entry's category dot/bar.
    pub color: Option<String>,
    pub total_ms: i64,
}

/* ------------------------------------------------------------------ *
 * Dashboard: get_today_overview()                                    *
 * SYSTEM_DESIGN.md s.8:                                              *
 *   -> { total_ms, top_apps[], by_category[], by_hour[] }            *
 * ------------------------------------------------------------------ */

/// One bucket of the "Today, by hour" time-series (24 buckets, hour 0..23).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourBucket {
    /// Local hour of day, 0..23.
    pub hour: u8,
    /// Active milliseconds attributed to this hour.
    pub active_ms: i64,
}

/// Aggregate time for one category, used by the donut/legend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryUsage {
    pub category_id: Option<i64>,
    /// Display name; "Uncategorized" when `category_id` is `None`.
    pub name: String,
    pub color: Option<String>,
    pub total_ms: i64,
}

/// The Dashboard hero + cards payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodayOverview {
    /// The 'YYYY-MM-DD' this overview is for (today, local time).
    pub day: String,
    /// Hero "Screen Time Today" number.
    pub total_ms: i64,
    /// Difference from yesterday's total at the same point; may be negative.
    pub delta_vs_yesterday_ms: i64,
    /// Up to N apps by time today (ordered desc), for the Top Apps card.
    pub top_apps: Vec<UsageEntry>,
    /// Per-category totals for the split donut/legend.
    pub by_category: Vec<CategoryUsage>,
    /// 24 buckets (hour 0..23) for the by-hour chart.
    pub by_hour: Vec<HourBucket>,
    /// App switch count today, for the StatCard.
    pub app_switches: i64,
    /// Longest single session today, for the StatCard.
    pub longest_session_ms: i64,
    /// App of the longest session; `None` when there is no usage yet.
    pub longest_session_app: Option<String>,
    /// Currently active app display name, or `None` when idle/locked.
    pub active_app: Option<String>,
}

/* ------------------------------------------------------------------ *
 * Reports: get_range_overview(from, to)                              *
 * ------------------------------------------------------------------ */

/// Total active time for one calendar day, for the Reports daily-bars chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayTotal {
    pub day: String,
    pub total_ms: i64,
}

/// Aggregates for an arbitrary inclusive date range (Week and Month).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeOverview {
    pub from: String,
    pub to: String,
    pub total_ms: i64,
    /// Mean active time per day across the range.
    pub daily_average_ms: i64,
    /// One entry per day in the range (zero-filled days included).
    pub by_day: Vec<DayTotal>,
    /// Top apps across the whole range.
    pub top_apps: Vec<UsageEntry>,
    /// Per-category totals across the whole range.
    pub by_category: Vec<CategoryUsage>,
    /// The day with the most usage; `None` when the range is empty.
    pub busiest_day: Option<String>,
    /// Total for the immediately-preceding equal-length range, for delta chips.
    pub prev_total_ms: i64,
}

/* ------------------------------------------------------------------ *
 * Settings: get_settings() / set_setting(key, value)                 *
 * ------------------------------------------------------------------ */

/// Theme preference; matches the TS `ThemePreference` union.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    System,
    Dark,
    Light,
}

/// Summary notification cadence. Matches the TS `SummaryCadence` union.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SummaryCadence {
    Off,
    Daily,
    Weekly,
    Both,
}

/// Typed view of settings the UI reads, parsed from string-valued `setting` rows
/// with defaults applied. Returned by `get_settings()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: ThemePreference,
    /// Idle threshold in seconds (default 120).
    pub idle_threshold_secs: u32,
    /// Window-title capture; OFF by default.
    pub capture_titles: bool,
    /// Raw `event` retention in days (default 90). Summaries kept forever.
    pub retention_days: u32,
    /// Collector paused state.
    pub tracking_paused: bool,
    /// Launch the app at login.
    pub launch_at_login: bool,
    /// Start hidden to the tray.
    pub start_minimized: bool,
    /// Optional productivity scoring; hides the Focus Score when false.
    pub scoring_enabled: bool,
    /// Summary notification cadence (off / daily / weekly / both).
    pub summary_cadence: SummaryCadence,
    /// Phase 3: break reminders enabled.
    pub breaks_enabled: bool,
    /// Minutes of continuous active use before a break is due (default 30).
    pub break_interval_mins: u32,
    /// How long the break overlay suggests resting, in seconds (default 20).
    pub break_duration_secs: u32,
    /// Strict breaks cannot be skipped.
    pub break_strict: bool,
    /// Phase 3: bedtime / wind-down quiet hours enabled.
    pub bedtime_enabled: bool,
    /// Quiet-hours start, "HH:MM" local (default "22:00").
    pub bedtime_start: String,
    /// Quiet-hours end, "HH:MM" local (default "07:00").
    pub bedtime_end: String,
    /// True until onboarding completes; gates the first-run flow.
    pub onboarding_complete: bool,
}

/* ------------------------------------------------------------------ *
 * Exclusions                                                         *
 * ------------------------------------------------------------------ */

/// How an exclusion pattern is matched. Wire values match the TS union.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionMatchType {
    App,
    TitleContains,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exclusion {
    pub id: i64,
    pub match_type: ExclusionMatchType,
    pub pattern: String,
}

/// Input shape for `add_exclusion` (no id yet).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewExclusion {
    pub match_type: ExclusionMatchType,
    pub pattern: String,
}

/* ------------------------------------------------------------------ *
 * Data commands: export / import / wipe                              *
 * ------------------------------------------------------------------ */

/// Export file format. Wire values match the TS union.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Csv,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub path: String,
    pub format: ExportFormat,
    pub rows_written: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub apps_added: i64,
    pub events_merged: i64,
    pub days_affected: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WipeResult {
    pub ok: bool,
}

/* ------------------------------------------------------------------ *
 * Category input (upsert_category argument)                          *
 * ------------------------------------------------------------------ */

/// Argument shape for `upsert_category` (`id == None` = insert).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryInput {
    pub id: Option<i64>,
    pub name: String,
    pub color: Option<String>,
    pub productive: Option<bool>,
}

/* ------------------------------------------------------------------ *
 * Phase 2: limits, blocking, focus (Control). Mirrors types.ts.       *
 * ------------------------------------------------------------------ */

/// How strict a per-app daily limit is when reached.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LimitStrictness {
    /// Track only; no interruption.
    Soft,
    /// A dismissible nudge (default).
    Medium,
    /// A strong, repeated nudge (a hard OS block is a later, elevated feature).
    Strict,
}

/// Argument for `set_limit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitInput {
    pub app_id: i64,
    pub daily_ms: i64,
    pub strictness: LimitStrictness,
}

/// A limit joined with today's usage for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitView {
    pub app_id: i64,
    pub app_key: String,
    pub display_name: String,
    pub daily_ms: i64,
    pub used_ms: i64,
    pub strictness: LimitStrictness,
    pub exceeded: bool,
}

/// What a block rule targets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BlockKind {
    App,
    Website,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRule {
    pub id: i64,
    pub kind: BlockKind,
    pub pattern: String,
    pub enabled: bool,
}

/// Argument for `set_block_rule` (`id == None` = insert).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRuleInput {
    pub id: Option<i64>,
    pub kind: BlockKind,
    pub pattern: String,
    pub enabled: bool,
}

/// Live focus-mode state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusState {
    pub active: bool,
    /// When the current focus session ends (UTC unix-millis), or None.
    pub ends_at_ms: Option<i64>,
    /// Number of enabled block rules.
    pub rules_count: i64,
}

/* ------------------------------------------------------------------ *
 * Collector state + live events (Rust -> UI). SYSTEM_DESIGN.md s.8.   *
 * ------------------------------------------------------------------ */

/// Collector run-state, surfaced to the Topbar indicator. Matches the TS union.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CollectorState {
    Active,
    Idle,
    Locked,
    Paused,
}

/// Payload of the `usage_tick` event: lightweight, safe to emit often (~5s).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageTick {
    /// Day the tick applies to (local 'YYYY-MM-DD').
    pub day: String,
    /// Running total for today at emit time.
    pub total_ms: i64,
    /// Active app display name, or `None` when idle/locked.
    pub active_app: Option<String>,
    /// Collector state, for the paused/idle indicator.
    pub state: CollectorState,
}

/// Canonical Tauri event channel names (Rust emits; UI subscribes).
/// Keep in sync with the TS `EVENT` registry.
pub mod event {
    /// Throttled (~5s) live update for the hero number and "Active now".
    pub const USAGE_TICK: &str = "usage_tick";
    /// Phase 2: a per-app daily limit was reached.
    pub const LIMIT_REACHED: &str = "limit_reached";
    /// Phase 2: a blocked app was opened during focus mode.
    pub const FOCUS_BLOCKED: &str = "focus_blocked";
    /// Phase 3: a wellbeing break is due.
    pub const BREAK_DUE: &str = "break_due";
    /// Phase 3: a focus session ended.
    pub const FOCUS_ENDED: &str = "focus_ended";
}

/// Canonical command names, matching the TS `COMMAND` registry and the
/// `#[tauri::command]` handler names in `commands.rs`. Phase 2+ are reserved.
pub mod command {
    // Dashboard / Reports
    pub const GET_TODAY_OVERVIEW: &str = "get_today_overview";
    pub const GET_RANGE_OVERVIEW: &str = "get_range_overview";
    // Apps / Categories
    pub const GET_APPS: &str = "get_apps";
    pub const SET_APP_CATEGORY: &str = "set_app_category";
    pub const GET_CATEGORIES: &str = "get_categories";
    pub const UPSERT_CATEGORY: &str = "upsert_category";
    pub const DELETE_CATEGORY: &str = "delete_category";
    // Settings
    pub const GET_SETTINGS: &str = "get_settings";
    pub const SET_SETTING: &str = "set_setting";
    // Exclusions
    pub const GET_EXCLUSIONS: &str = "get_exclusions";
    pub const ADD_EXCLUSION: &str = "add_exclusion";
    pub const REMOVE_EXCLUSION: &str = "remove_exclusion";
    // Data
    pub const EXPORT_DATA: &str = "export_data";
    pub const IMPORT_DATA: &str = "import_data";
    pub const WIPE_ALL_DATA: &str = "wipe_all_data";
    // Collector control
    pub const GET_COLLECTOR_STATE: &str = "get_collector_state";
    pub const SET_TRACKING_PAUSED: &str = "set_tracking_paused";
    // Phase 2: limits
    pub const GET_LIMITS: &str = "get_limits";
    pub const SET_LIMIT: &str = "set_limit";
    pub const REMOVE_LIMIT: &str = "remove_limit";
    // Phase 2: blocking
    pub const GET_BLOCK_RULES: &str = "get_block_rules";
    pub const SET_BLOCK_RULE: &str = "set_block_rule";
    pub const REMOVE_BLOCK_RULE: &str = "remove_block_rule";
    // Phase 2: focus mode
    pub const START_FOCUS_SESSION: &str = "start_focus_session";
    pub const STOP_FOCUS_SESSION: &str = "stop_focus_session";
    pub const GET_FOCUS_STATE: &str = "get_focus_state";
}
