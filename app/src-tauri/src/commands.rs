//! Tauri command handlers (SYSTEM_DESIGN.md section 8). Thin wrappers over `db`
//! that lock the connection and map errors to strings for the frontend. Command
//! names and argument names match `app/src/lib/types.ts` exactly; `rename_all`
//! keeps the JS-side argument keys snake_case to match the shared contract.

use crate::db;
use crate::models::*;
use crate::state::AppState;
use chrono::Utc;
use tauri::State;

type R<T> = Result<T, String>;

fn lock<'a, T>(m: &'a std::sync::Mutex<T>) -> R<std::sync::MutexGuard<'a, T>> {
    m.lock().map_err(|e| e.to_string())
}

/* ------------------------------- dashboard -------------------------------- */

#[tauri::command]
pub fn get_today_overview(state: State<AppState>) -> R<TodayOverview> {
    let conn = lock(&state.db)?;
    let mut ov = db::today_overview(&conn)?;
    ov.active_app = lock(&state.shared)?.active_app.clone();
    Ok(ov)
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_range_overview(state: State<AppState>, from: String, to: String) -> R<RangeOverview> {
    let conn = lock(&state.db)?;
    db::range_overview(&conn, &from, &to)
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_day_overview(state: State<AppState>, day: String) -> R<TodayOverview> {
    let conn = lock(&state.db)?;
    db::day_overview(&conn, &day)
}

/* ----------------------------- apps + categories -------------------------- */

#[tauri::command]
pub fn get_apps(state: State<AppState>) -> R<Vec<AppInfo>> {
    let conn = lock(&state.db)?;
    db::get_apps(&conn)
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_app_category(state: State<AppState>, app_id: i64, category_id: Option<i64>) -> R<()> {
    let conn = lock(&state.db)?;
    db::set_app_category(&conn, app_id, category_id)
}

#[tauri::command]
pub fn get_categories(state: State<AppState>) -> R<Vec<Category>> {
    let conn = lock(&state.db)?;
    db::get_categories(&conn)
}

#[tauri::command(rename_all = "snake_case")]
pub fn upsert_category(state: State<AppState>, category: CategoryInput) -> R<Category> {
    let conn = lock(&state.db)?;
    db::upsert_category(&conn, &category)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_category(state: State<AppState>, id: i64) -> R<()> {
    let conn = lock(&state.db)?;
    db::delete_category(&conn, id)
}

/* --------------------------------- settings ------------------------------- */

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> R<Settings> {
    let conn = lock(&state.db)?;
    db::get_settings(&conn)
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_setting(
    app: tauri::AppHandle,
    state: State<AppState>,
    key: String,
    value: String,
) -> R<()> {
    {
        let conn = lock(&state.db)?;
        db::set_setting(&conn, &key, &value)?;
    }
    // Apply launch-at-login immediately (the autostart plugin manages the OS entry).
    if key == "launch_at_login" {
        use tauri_plugin_autostart::ManagerExt;
        let on = value == "true" || value == "1";
        let mgr = app.autolaunch();
        let _ = if on { mgr.enable() } else { mgr.disable() };
    }
    // Mirror collector-affecting settings into the live shared state.
    let mut s = lock(&state.shared)?;
    match key.as_str() {
        "idle_threshold_secs" => {
            if let Ok(v) = value.parse::<u64>() {
                s.idle_threshold_ms = v * 1000;
            }
        }
        "capture_titles" => s.capture_titles = value == "true" || value == "1",
        "tracking_paused" => s.paused = value == "true" || value == "1",
        _ => {}
    }
    Ok(())
}

/* -------------------------------- exclusions ------------------------------ */

#[tauri::command]
pub fn get_exclusions(state: State<AppState>) -> R<Vec<Exclusion>> {
    let conn = lock(&state.db)?;
    db::get_exclusions(&conn)
}

#[tauri::command(rename_all = "snake_case")]
pub fn add_exclusion(state: State<AppState>, exclusion: NewExclusion) -> R<Exclusion> {
    let conn = lock(&state.db)?;
    db::add_exclusion(&conn, &exclusion)
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_exclusion(state: State<AppState>, id: i64) -> R<()> {
    let conn = lock(&state.db)?;
    db::remove_exclusion(&conn, id)
}

/* ------------------------------- data commands ---------------------------- */

#[tauri::command(rename_all = "snake_case")]
pub fn export_data(state: State<AppState>, format: ExportFormat, path: String) -> R<ExportResult> {
    let conn = lock(&state.db)?;
    db::export_data(&conn, format, &path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn import_data(state: State<AppState>, path: String) -> R<ImportResult> {
    let conn = lock(&state.db)?;
    db::import_data(&conn, &path)
}

#[tauri::command]
pub fn wipe_all_data(state: State<AppState>) -> R<WipeResult> {
    let conn = lock(&state.db)?;
    db::wipe_all_data(&conn)
}

/* ----------------------------- collector control -------------------------- */

#[tauri::command]
pub fn get_collector_state(state: State<AppState>) -> R<CollectorState> {
    Ok(lock(&state.shared)?.state)
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_tracking_paused(state: State<AppState>, paused: bool) -> R<CollectorState> {
    {
        let conn = lock(&state.db)?;
        db::set_setting(
            &conn,
            "tracking_paused",
            if paused { "true" } else { "false" },
        )?;
    }
    let mut s = lock(&state.shared)?;
    s.paused = paused;
    if paused {
        s.state = CollectorState::Paused;
        s.active_app = None;
    }
    Ok(s.state)
}

/* ------------------------------ phase 2: limits --------------------------- */

#[tauri::command]
pub fn get_limits(state: State<AppState>) -> R<Vec<LimitView>> {
    let conn = lock(&state.db)?;
    db::get_limits(&conn)
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_limit(state: State<AppState>, limit: LimitInput) -> R<()> {
    let conn = lock(&state.db)?;
    db::set_limit(&conn, &limit)
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_limit(state: State<AppState>, app_id: i64) -> R<()> {
    let conn = lock(&state.db)?;
    db::remove_limit(&conn, app_id)
}

/* ----------------------------- phase 2: blocking -------------------------- */

#[tauri::command]
pub fn get_block_rules(state: State<AppState>) -> R<Vec<BlockRule>> {
    let conn = lock(&state.db)?;
    db::get_block_rules(&conn)
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_block_rule(state: State<AppState>, rule: BlockRuleInput) -> R<BlockRule> {
    let conn = lock(&state.db)?;
    db::set_block_rule(&conn, &rule)
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_block_rule(state: State<AppState>, id: i64) -> R<()> {
    let conn = lock(&state.db)?;
    db::remove_block_rule(&conn, id)
}

/* ------------------------------ phase 2: focus ---------------------------- */

fn build_focus_state(state: &State<AppState>) -> R<FocusState> {
    let rules_count = {
        let conn = lock(&state.db)?;
        db::enabled_block_rules_count(&conn)?
    };
    let s = lock(&state.shared)?;
    Ok(FocusState {
        active: s.focus_active,
        ends_at_ms: s.focus_ends_ms,
        rules_count,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn start_focus_session(state: State<AppState>, minutes: i64) -> R<FocusState> {
    {
        let mut s = lock(&state.shared)?;
        s.focus_active = true;
        s.focus_ends_ms = if minutes > 0 {
            Some(Utc::now().timestamp_millis() + minutes * 60_000)
        } else {
            None
        };
    }
    build_focus_state(&state)
}

#[tauri::command]
pub fn stop_focus_session(state: State<AppState>) -> R<FocusState> {
    {
        let mut s = lock(&state.shared)?;
        s.focus_active = false;
        s.focus_ends_ms = None;
    }
    build_focus_state(&state)
}

#[tauri::command]
pub fn get_focus_state(state: State<AppState>) -> R<FocusState> {
    build_focus_state(&state)
}

/* ------------- phase 4: system-wide website blocking (gated) -------------- */

/// Write enabled website block rules into the hosts file. Needs administrator
/// rights; returns the number of domains written or an error to surface.
#[tauri::command]
pub fn apply_website_block(state: State<AppState>) -> R<usize> {
    let domains = {
        let conn = lock(&state.db)?;
        db::enabled_website_block_patterns(&conn)?
    };
    crate::blocker::apply(&domains)
}

/// Remove System Trace's managed block from the hosts file.
#[tauri::command]
pub fn clear_website_block() -> R<()> {
    crate::blocker::clear()
}
