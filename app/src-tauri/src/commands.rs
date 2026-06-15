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

#[tauri::command]
pub fn get_category_goals(state: State<AppState>) -> R<Vec<CategoryGoal>> {
    let conn = lock(&state.db)?;
    db::get_category_goals(&conn)
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_category_goal(state: State<AppState>, goal: CategoryGoalInput) -> R<()> {
    let conn = lock(&state.db)?;
    db::set_category_goal(&conn, &goal)
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_category_goal(state: State<AppState>, category_id: i64) -> R<()> {
    let conn = lock(&state.db)?;
    db::remove_category_goal(&conn, category_id)
}

#[tauri::command]
pub fn get_app_goals(state: State<AppState>) -> R<Vec<AppGoal>> {
    let conn = lock(&state.db)?;
    db::get_app_goals(&conn)
}

/// The app's real OS icon as raw RGBA pixels, if we know its path and can
/// extract one. The frontend caches the result and falls back to a letter
/// avatar when this is `None`.
#[tauri::command(rename_all = "snake_case")]
pub fn get_app_icon(state: State<AppState>, app_key: String) -> R<Option<AppIcon>> {
    #[allow(unused_mut)]
    let mut path = {
        let conn = lock(&state.db)?;
        db::get_app_path(&conn, &app_key)?
    };

    #[cfg(target_os = "linux")]
    if path.is_none() {
        if let Some(resolved) = crate::icon::resolve_linux_icon(&app_key) {
            let conn = lock(&state.db)?;
            let _ = db::set_app_path(&conn, &app_key, &resolved);
            path = Some(resolved);
        } else {
            let conn = lock(&state.db)?;
            let _ = db::set_app_path(&conn, &app_key, "none");
            path = Some("none".to_string());
        }
    }

    let Some(path) = path else {
        return Ok(None);
    };

    if path == "none" {
        return Ok(None);
    }

    Ok(
        crate::icon::extract_icon_rgba(&path).map(|(width, height, rgba)| AppIcon {
            width,
            height,
            rgba,
        }),
    )
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_app_goal(state: State<AppState>, goal: AppGoalInput) -> R<()> {
    let conn = lock(&state.db)?;
    db::set_app_goal(&conn, &goal)
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_app_goal(state: State<AppState>, app_id: i64) -> R<()> {
    let conn = lock(&state.db)?;
    db::remove_app_goal(&conn, app_id)
}

#[tauri::command]
pub fn get_focus_score(state: State<AppState>) -> R<FocusScore> {
    let conn = lock(&state.db)?;
    let day = Utc::now()
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d")
        .to_string();
    db::focus_score_for_day(&conn, &day)
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
pub fn export_data(
    state: State<AppState>,
    format: ExportFormat,
    path: String,
    from: Option<String>,
    to: Option<String>,
) -> R<ExportResult> {
    let conn = lock(&state.db)?;
    db::export_data(&conn, format, &path, from.as_deref(), to.as_deref())
}

#[tauri::command(rename_all = "snake_case")]
pub fn import_data(state: State<AppState>, path: String) -> R<ImportResult> {
    let conn = lock(&state.db)?;
    db::import_data(&conn, &path)
}

#[tauri::command]
pub fn wipe_all_data(state: State<AppState>) -> R<WipeResult> {
    let result = {
        let conn = lock(&state.db)?;
        let r = db::wipe_all_data(&conn)?;
        // Immediately overwrite the encrypted snapshot so the wiped data is
        // gone from disk too, not just from the in-memory database.
        if let Some((path, key)) = &state.enc {
            let _ = db::snapshot_encrypted(&conn, path, key);
        }
        r
    };
    Ok(result)
}

/// Write a consistent snapshot of the live database to a user-chosen path using
/// SQLite's online backup API. This is atomic with respect to other writers on
/// the same connection (we hold the lock), so the collector cannot slip a write
/// in between a checkpoint and a file copy the way a raw `fs::copy` allowed.
#[tauri::command(rename_all = "snake_case")]
pub fn backup_database(state: State<AppState>, path: String) -> R<BackupResult> {
    {
        let conn = lock(&state.db)?;
        conn.backup(rusqlite::DatabaseName::Main, &path, None)
            .map_err(|e| e.to_string())?;
    }
    let bytes = std::fs::metadata(&path)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    Ok(BackupResult { path, bytes })
}

/// Restore the database from a backup file using SQLite's online restore API,
/// straight into the live connection. Because the data is copied page-by-page
/// into the open database (rather than replacing the file under an open handle),
/// it is safe and takes effect immediately - no restart, no corruption.
#[tauri::command(rename_all = "snake_case")]
pub fn restore_database(state: State<AppState>, path: String) -> R<()> {
    // Basic sanity: the file must be a SQLite database (magic header) so we do
    // not feed garbage into the restore.
    let header = {
        let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
        use std::io::Read;
        let mut buf = [0u8; 16];
        f.read_exact(&mut buf).map_err(|e| e.to_string())?;
        buf
    };
    if &header[..15] != b"SQLite format 3" {
        return Err("That file is not a System Trace backup.".into());
    }

    // Hold the lock for the whole restore so the collector cannot write mid-copy.
    let mut conn = lock(&state.db)?;
    conn.restore(
        rusqlite::DatabaseName::Main,
        &path,
        None::<fn(rusqlite::backup::Progress)>,
    )
    .map_err(|e| e.to_string())?;
    // The backup may be from an older version; bring its schema up to date so
    // newer columns, tables, and settings exist.
    db::migrate(&conn)?;
    // Persist the restored data to the encrypted snapshot immediately, so a
    // crash before the next periodic snapshot can't lose the restore (the live
    // DB is in memory). Mirrors wipe_all_data.
    if let Some((enc_path, key)) = &state.enc {
        let _ = db::snapshot_encrypted(&conn, enc_path, key);
    }
    Ok(())
}

/* ------------------------------- search ----------------------------------- */

#[tauri::command(rename_all = "snake_case")]
pub fn search_usage(
    state: State<AppState>,
    query: String,
    from: Option<String>,
    to: Option<String>,
) -> R<Vec<SearchHit>> {
    let conn = lock(&state.db)?;
    db::search_usage(&conn, &query, from.as_deref(), to.as_deref())
}

/* ---------------------------- focus sessions ------------------------------ */

#[tauri::command(rename_all = "snake_case")]
pub fn save_focus_session(
    state: State<AppState>,
    start_ms: i64,
    end_ms: i64,
    note: String,
) -> R<()> {
    let conn = lock(&state.db)?;
    db::save_focus_session(&conn, start_ms, end_ms, &note)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_focus_sessions(state: State<AppState>, limit: i64) -> R<Vec<FocusSession>> {
    let conn = lock(&state.db)?;
    db::list_focus_sessions(&conn, limit)
}

/* -------------------------------- streaks --------------------------------- */

#[tauri::command]
pub fn get_goal_streaks(state: State<AppState>) -> R<Vec<GoalStreak>> {
    let conn = lock(&state.db)?;
    db::get_goal_streaks(&conn)
}

/* ----------------------------- collector control -------------------------- */

#[tauri::command]
pub fn get_collector_state(state: State<AppState>) -> R<CollectorState> {
    Ok(lock(&state.shared)?.state)
}

/// Whether the global pause/resume hotkey registered at startup. False means
/// another process already owns Ctrl+Alt+P; the UI shows the chord as
/// unavailable instead of letting it look broken.
#[tauri::command]
pub fn get_hotkey_status(state: State<AppState>) -> R<bool> {
    Ok(lock(&state.shared)?.hotkey_registered)
}

/// Bring the main window to the foreground. Called when the user clicks one of
/// our OS notifications (the frontend registers a notification-action handler)
/// so a reminder always opens the app instead of doing nothing - and works the
/// same on every platform.
#[tauri::command]
pub fn focus_main_window(app: tauri::AppHandle) -> R<()> {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}

/// Write raw bytes (a PDF the frontend generated) to a user-chosen path. The
/// path comes from the native save dialog, so this only ever writes where the
/// user explicitly pointed it.
#[tauri::command(rename_all = "snake_case")]
pub fn save_report_pdf(path: String, bytes: Vec<u8>) -> R<i64> {
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(bytes.len() as i64)
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
    } else {
        // Resuming: reflect a non-paused state immediately so the Topbar button
        // flips to "Pause" right away. The collector refines this to
        // Active / Idle / Locked on its next loop and the usage_tick event.
        s.state = CollectorState::Idle;
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
