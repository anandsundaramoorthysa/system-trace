//! The collector: a pure, testable session-builder plus the background runtime
//! that drives it (SYSTEM_DESIGN.md sections 3-4).
//!
//! `SessionBuilder` is OS- and IO-free: feed it samples, it emits finished
//! `RawEvent`s. That seam lets `cargo test` verify the state machine with a
//! scripted fake watcher (no real OS, no clock). `spawn` wires the real watcher,
//! clock, database, and Tauri event channel around it.

use crate::platform::ActiveWindow;

/// One finished active session, ready to persist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEvent {
    pub app_key: String,
    pub app_name: String,
    pub title: Option<String>,
    pub start_ms: i64,
    pub end_ms: i64,
}

struct OpenSession {
    app_key: String,
    app_name: String,
    title: Option<String>,
    start_ms: i64,
    last_ms: i64,
}

/// Builds usage sessions from a stream of samples. Pure and deterministic.
pub struct SessionBuilder {
    idle_threshold_ms: u64,
    capture_titles: bool,
    open: Option<OpenSession>,
}

impl SessionBuilder {
    pub fn new(idle_threshold_ms: u64, capture_titles: bool) -> Self {
        SessionBuilder {
            idle_threshold_ms,
            capture_titles,
            open: None,
        }
    }

    /// Apply live settings changes without losing the open session.
    pub fn set_config(&mut self, idle_threshold_ms: u64, capture_titles: bool) {
        self.idle_threshold_ms = idle_threshold_ms;
        self.capture_titles = capture_titles;
    }

    /// True when the sample means the user is actively using the computer.
    pub fn is_active(&self, has_window: bool, idle_ms: u64, locked: bool, media: bool) -> bool {
        if locked || !has_window {
            return false;
        }
        // Media playing keeps us active even with no input (fixes the
        // "watching a video counts as idle" problem).
        media || idle_ms < self.idle_threshold_ms
    }

    /// Feed one sample. Returns a finished event when a session just closed
    /// (an app switch or going idle/locked closes the previous session).
    pub fn observe(
        &mut self,
        now_ms: i64,
        win: Option<ActiveWindow>,
        idle_ms: u64,
        locked: bool,
        media: bool,
    ) -> Option<RawEvent> {
        let active = self.is_active(win.is_some(), idle_ms, locked, media);

        if !active {
            return self.close();
        }

        // Safe: active implies win.is_some().
        let win = win.unwrap();
        let title = if self.capture_titles {
            win.title.clone()
        } else {
            None
        };

        match &mut self.open {
            Some(cur) if cur.app_key == win.app_key && cur.title == title => {
                // Same screen: extend it.
                cur.last_ms = now_ms;
                None
            }
            _ => {
                // Different screen (or first sample): close the old, open a new.
                let closed = self.close();
                self.open = Some(OpenSession {
                    app_key: win.app_key,
                    app_name: win.app_name,
                    title,
                    start_ms: now_ms,
                    last_ms: now_ms,
                });
                closed
            }
        }
    }

    /// Close and emit the open session, if any (used on app switch, idle, and
    /// shutdown). A zero-length session (opened but never extended) is dropped.
    pub fn close(&mut self) -> Option<RawEvent> {
        let s = self.open.take()?;
        if s.last_ms <= s.start_ms {
            return None;
        }
        Some(RawEvent {
            app_key: s.app_key,
            app_name: s.app_name,
            title: s.title,
            start_ms: s.start_ms,
            end_ms: s.last_ms,
        })
    }

    /// Alias used by the runtime on shutdown/pause.
    pub fn flush(&mut self) -> Option<RawEvent> {
        self.close()
    }
}

// ----------------------------------------------------------------------------
// Background runtime (not unit-tested; uses the real watcher, clock, and DB).
// ----------------------------------------------------------------------------

mod runtime {
    use super::{RawEvent, SessionBuilder};
    use crate::db;
    use crate::models::{event, CollectorState, LimitStrictness, UsageTick};
    use crate::platform;
    use crate::state::Shared;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tauri::{AppHandle, Emitter, Manager};
    use tauri_plugin_notification::NotificationExt;

    const TICK: Duration = Duration::from_secs(1);
    const FLUSH_MS: i64 = 15_000;
    const EMIT_MS: i64 = 5_000;
    const MAX_BUFFER: usize = 64;

    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    /// Format a duration in ms as "Xh Ym" (or "Ym" under an hour).
    fn fmt_hm(ms: i64) -> String {
        let mins = ms / 60_000;
        let (h, m) = (mins / 60, mins % 60);
        if h > 0 {
            format!("{h}h {m}m")
        } else {
            format!("{m}m")
        }
    }

    /// Phase 3 wellbeing config, refreshed from settings periodically.
    struct Wellbeing {
        breaks_enabled: bool,
        break_interval_ms: i64,
        break_duration_secs: u32,
        break_strict: bool,
        bedtime_enabled: bool,
        bed_start: i32,
        bed_end: i32,
    }

    fn parse_hhmm(s: &str) -> i32 {
        let mut it = s.split(':');
        let h: i32 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        let m: i32 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        (h * 60 + m).clamp(0, 1439)
    }

    fn load_wellbeing(db: &Arc<Mutex<rusqlite::Connection>>) -> Wellbeing {
        if let Ok(c) = db.lock() {
            if let Ok(s) = db::get_settings(&c) {
                return Wellbeing {
                    breaks_enabled: s.breaks_enabled,
                    break_interval_ms: (s.break_interval_mins as i64) * 60_000,
                    break_duration_secs: s.break_duration_secs,
                    break_strict: s.break_strict,
                    bedtime_enabled: s.bedtime_enabled,
                    bed_start: parse_hhmm(&s.bedtime_start),
                    bed_end: parse_hhmm(&s.bedtime_end),
                };
            }
        }
        Wellbeing {
            breaks_enabled: false,
            break_interval_ms: 30 * 60_000,
            break_duration_secs: 20,
            break_strict: false,
            bedtime_enabled: false,
            bed_start: 22 * 60,
            bed_end: 7 * 60,
        }
    }

    /// Is the local wall-clock currently inside the (possibly overnight) window?
    fn in_bedtime_now(start: i32, end: i32, now_ms: i64) -> bool {
        use chrono::Timelike;
        let mins = chrono::DateTime::from_timestamp_millis(now_ms)
            .map(|d| d.with_timezone(&chrono::Local))
            .map(|d| (d.hour() * 60 + d.minute()) as i32)
            .unwrap_or(0);
        if start == end {
            false
        } else if start < end {
            mins >= start && mins < end
        } else {
            mins >= start || mins < end
        }
    }

    /// Send any due daily/weekly summary notifications, at most once each per
    /// period. Markers persisted in `setting` make it catch up after the app was
    /// closed across a midnight or a week boundary. Safe to call on startup and at
    /// each day rollover.
    fn check_summaries(app: &AppHandle, db: &Arc<Mutex<rusqlite::Connection>>) {
        use crate::models::SummaryCadence as SC;
        use chrono::{Datelike, Duration as ChronoDuration, Local};

        let (cadence, last_daily, last_weekly) = {
            let Ok(c) = db.lock() else {
                return;
            };
            let cadence = db::get_settings(&c)
                .map(|s| s.summary_cadence)
                .unwrap_or(SC::Daily);
            let ld = db::get_raw_setting(&c, "last_daily_summary_day")
                .ok()
                .flatten();
            let lw = db::get_raw_setting(&c, "last_weekly_summary_week")
                .ok()
                .flatten();
            (cadence, ld, lw)
        };

        if cadence == SC::Off {
            return;
        }
        let today = Local::now().date_naive();

        // ---- Daily: summarize yesterday, once. ----
        if matches!(cadence, SC::Daily | SC::Both) {
            let key = (today - ChronoDuration::days(1))
                .format("%Y-%m-%d")
                .to_string();
            if last_daily.as_deref() != Some(key.as_str()) {
                let total = db
                    .lock()
                    .ok()
                    .and_then(|c| db::day_total(&c, &key).ok())
                    .unwrap_or(0);
                if total > 0 {
                    let _ = app
                        .notification()
                        .builder()
                        .title("Yesterday on System Trace")
                        .body(format!("{} of screen time on {}.", fmt_hm(total), key))
                        .show();
                }
                if let Ok(c) = db.lock() {
                    let _ = db::set_setting(&c, "last_daily_summary_day", &key);
                }
            }
        }

        // ---- Weekly: summarize the previous completed week (Mon-Sun), once. ----
        if matches!(cadence, SC::Weekly | SC::Both) {
            let dow = today.weekday().num_days_from_monday() as i64;
            let prev_monday = today - ChronoDuration::days(dow + 7);
            let prev_sunday = prev_monday + ChronoDuration::days(6);
            let iso = prev_monday.iso_week();
            let week_key = format!("{}-W{:02}", iso.year(), iso.week());

            if last_weekly.as_deref() != Some(week_key.as_str()) {
                let from = prev_monday.format("%Y-%m-%d").to_string();
                let to = prev_sunday.format("%Y-%m-%d").to_string();
                let before_from = (prev_monday - ChronoDuration::days(7))
                    .format("%Y-%m-%d")
                    .to_string();
                let before_to = (prev_sunday - ChronoDuration::days(7))
                    .format("%Y-%m-%d")
                    .to_string();
                let (total, before) = match db.lock() {
                    Ok(c) => (
                        db::range_total(&c, &from, &to).unwrap_or(0),
                        db::range_total(&c, &before_from, &before_to).unwrap_or(0),
                    ),
                    Err(_) => (0, 0),
                };
                if total > 0 {
                    let delta = if before > 0 {
                        let pct =
                            (((total - before) as f64 / before as f64) * 100.0).round() as i64;
                        if pct >= 0 {
                            format!(", up {pct}% from the week before")
                        } else {
                            format!(", down {}% from the week before", -pct)
                        }
                    } else {
                        String::new()
                    };
                    let _ = app
                        .notification()
                        .builder()
                        .title("Last week on System Trace")
                        .body(format!("{} total{}.", fmt_hm(total), delta))
                        .show();
                }
                if let Ok(c) = db.lock() {
                    let _ = db::set_setting(&c, "last_weekly_summary_week", &week_key);
                }
            }
        }
    }

    /// Start the collector thread. It runs for the life of the process.
    pub fn spawn(app: AppHandle, db: Arc<Mutex<rusqlite::Connection>>, shared: Arc<Mutex<Shared>>) {
        std::thread::Builder::new()
            .name("system-trace-collector".into())
            .spawn(move || {
                let mut watcher = platform::make_watcher();
                let (mut thr, mut cap) = {
                    let s = shared.lock().unwrap();
                    (s.idle_threshold_ms, s.capture_titles)
                };
                let mut builder = SessionBuilder::new(thr, cap);
                let mut buffer: Vec<RawEvent> = Vec::new();
                let mut last_flush = now_ms();
                let mut last_emit = 0i64;
                // Phase 2 control state.
                let mut fired_limits: std::collections::HashSet<i64> =
                    std::collections::HashSet::new();
                let mut fired_day = db::local_day(now_ms());
                let mut block_patterns: Vec<String> = Vec::new();
                // Phase 3 wellbeing state.
                let mut well = load_wellbeing(&db);
                let mut active_run_ms: i64 = 0;
                let mut last_tick = now_ms();
                // Catch up on any summary missed while the app was closed.
                check_summaries(&app, &db);

                loop {
                    let now = now_ms();
                    let delta = (now - last_tick).max(0);
                    last_tick = now;

                    // Pull live settings each loop so changes apply without restart.
                    let paused = {
                        let s = shared.lock().unwrap();
                        thr = s.idle_threshold_ms;
                        cap = s.capture_titles;
                        s.paused
                    };
                    builder.set_config(thr, cap);

                    let mut cur_key: Option<String> = None;
                    let mut cur_name: Option<String> = None;
                    let new_state;
                    let active_app;

                    if paused {
                        if let Some(ev) = builder.flush() {
                            buffer.push(ev);
                        }
                        new_state = CollectorState::Paused;
                        active_app = None;
                    } else {
                        let win = watcher.active_window();
                        let idle = watcher.idle_ms();
                        let locked = watcher.session_locked();
                        let media = watcher.is_media_playing();
                        cur_key = win.as_ref().map(|w| w.app_key.clone());
                        cur_name = win.as_ref().map(|w| w.app_name.clone());

                        if let Some(ev) = builder.observe(now, win.clone(), idle, locked, media) {
                            buffer.push(ev);
                        }

                        if locked {
                            new_state = CollectorState::Locked;
                            active_app = None;
                        } else if builder.is_active(win.is_some(), idle, locked, media) {
                            new_state = CollectorState::Active;
                            active_app = win.map(|w| w.app_name);
                        } else {
                            new_state = CollectorState::Idle;
                            active_app = None;
                        }
                    }

                    // Persist in batches, never on every tick.
                    if (now - last_flush >= FLUSH_MS && !buffer.is_empty())
                        || buffer.len() >= MAX_BUFFER
                    {
                        if let Ok(conn) = db.lock() {
                            if let Err(e) = db::insert_events(&conn, &buffer) {
                                log::warn!("collector flush failed: {e}");
                            } else {
                                buffer.clear();
                            }
                        }
                        last_flush = now;
                    }

                    // Update shared live state for the commands.
                    {
                        let mut s = shared.lock().unwrap();
                        s.state = new_state;
                        s.active_app = active_app.clone();
                    }

                    // ---- Phase 3 wellbeing: break reminders + bedtime quiet hours ----
                    let in_bed =
                        well.bedtime_enabled && in_bedtime_now(well.bed_start, well.bed_end, now);
                    if matches!(new_state, CollectorState::Active) {
                        active_run_ms += delta;
                    } else {
                        active_run_ms = 0;
                    }
                    if well.breaks_enabled && !in_bed && active_run_ms >= well.break_interval_ms {
                        active_run_ms = 0;
                        let _ = app.emit(
                            event::BREAK_DUE,
                            serde_json::json!({
                                "duration_secs": well.break_duration_secs,
                                "strict": well.break_strict,
                            }),
                        );
                        let _ = app
                            .notification()
                            .builder()
                            .title("Time for a break")
                            .body("Look away from the screen and rest your eyes.")
                            .show();
                        // Bring the window forward so the overlay is seen.
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.unminimize();
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }

                    // Throttled live update for the UI hero number.
                    if now - last_emit >= EMIT_MS {
                        let (day, total_ms) = db
                            .lock()
                            .ok()
                            .and_then(|c| db::today_total(&c).ok())
                            .unwrap_or_else(|| (db::local_day(now), 0));
                        let _ = app.emit(
                            event::USAGE_TICK,
                            UsageTick {
                                day: day.clone(),
                                total_ms,
                                active_app,
                                state: new_state,
                            },
                        );
                        last_emit = now;
                        // Refresh wellbeing config so settings changes apply live.
                        well = load_wellbeing(&db);

                        // ---- Phase 2 control: limits, focus auto-end, block nudges ----
                        if day != fired_day {
                            fired_day = day;
                            fired_limits.clear();
                            // Day rolled over: send any due daily/weekly summary.
                            check_summaries(&app, &db);
                        }

                        // Auto-end a focus session whose timer elapsed.
                        let (focus_active, focus_ends) = {
                            let s = shared.lock().unwrap();
                            (s.focus_active, s.focus_ends_ms)
                        };
                        if focus_active {
                            if let Some(end) = focus_ends {
                                if now >= end {
                                    {
                                        let mut s = shared.lock().unwrap();
                                        s.focus_active = false;
                                        s.focus_ends_ms = None;
                                    }
                                    let _ = app.emit(event::FOCUS_ENDED, ());
                                }
                            }
                        }

                        // Refresh block patterns and collect newly-exceeded limits.
                        let mut newly_exceeded: Vec<crate::models::LimitView> = Vec::new();
                        if let Ok(c) = db.lock() {
                            block_patterns = db::enabled_app_block_patterns(&c).unwrap_or_default();
                            if let Ok(limits) = db::get_limits(&c) {
                                for l in limits {
                                    if l.exceeded
                                        && l.strictness != LimitStrictness::Soft
                                        && !fired_limits.contains(&l.app_id)
                                    {
                                        fired_limits.insert(l.app_id);
                                        newly_exceeded.push(l);
                                    }
                                }
                            }
                        }
                        // Quiet hours suppress limit nudges (focus blocks stay explicit).
                        if !in_bed {
                            for l in newly_exceeded {
                                let _ = app
                                    .notification()
                                    .builder()
                                    .title("Daily limit reached")
                                    .body(format!(
                                        "{} - {} of {} used today.",
                                        l.display_name,
                                        fmt_hm(l.used_ms),
                                        fmt_hm(l.daily_ms)
                                    ))
                                    .show();
                                let _ = app.emit(event::LIMIT_REACHED, l);
                            }
                        }

                        // Nudge if a blocked app is in the foreground during focus mode.
                        let focus_on = { shared.lock().unwrap().focus_active };
                        if focus_on {
                            if let Some(key) = &cur_key {
                                let kl = key.to_lowercase();
                                if block_patterns
                                    .iter()
                                    .any(|p| kl.contains(&p.to_lowercase()))
                                {
                                    let name = cur_name.clone().unwrap_or_else(|| key.clone());
                                    let _ = app.emit(
                                        event::FOCUS_BLOCKED,
                                        serde_json::json!({ "app": name }),
                                    );
                                }
                            }
                        }
                    }

                    std::thread::sleep(TICK);
                }
            })
            .expect("failed to spawn collector thread");
    }
}

pub use runtime::spawn;

#[cfg(test)]
mod tests {
    use super::*;

    fn win(app: &str, title: Option<&str>) -> ActiveWindow {
        ActiveWindow {
            app_key: format!("{app}.exe"),
            app_name: app.to_string(),
            title: title.map(|t| t.to_string()),
        }
    }

    #[test]
    fn extends_same_app_then_closes_on_switch() {
        let mut b = SessionBuilder::new(120_000, false);
        assert_eq!(
            b.observe(0, Some(win("chrome", None)), 0, false, false),
            None
        );
        assert_eq!(
            b.observe(1000, Some(win("chrome", None)), 0, false, false),
            None
        );
        assert_eq!(
            b.observe(2000, Some(win("chrome", None)), 0, false, false),
            None
        );
        // Switch to code: closes the chrome session 0..2000.
        let ev = b
            .observe(3000, Some(win("code", None)), 0, false, false)
            .unwrap();
        assert_eq!(ev.app_key, "chrome.exe");
        assert_eq!(ev.start_ms, 0);
        assert_eq!(ev.end_ms, 2000);
    }

    #[test]
    fn idle_closes_the_session() {
        let mut b = SessionBuilder::new(120_000, false);
        b.observe(0, Some(win("chrome", None)), 0, false, false);
        b.observe(1000, Some(win("chrome", None)), 0, false, false);
        // Idle beyond threshold closes it.
        let ev = b
            .observe(2000, Some(win("chrome", None)), 200_000, false, false)
            .unwrap();
        assert_eq!(ev.end_ms, 1000);
        // Next idle sample emits nothing.
        assert_eq!(
            b.observe(3000, Some(win("chrome", None)), 200_000, false, false),
            None
        );
    }

    #[test]
    fn media_keeps_session_active_while_idle() {
        let mut b = SessionBuilder::new(120_000, false);
        b.observe(0, Some(win("vlc", None)), 0, false, true);
        // No input for a long time, but media is playing: stays active.
        assert_eq!(
            b.observe(300_000, Some(win("vlc", None)), 290_000, false, true),
            None
        );
    }

    #[test]
    fn zero_length_session_is_dropped() {
        let mut b = SessionBuilder::new(120_000, false);
        // Single sample then idle: opened and immediately closed, no event.
        b.observe(0, Some(win("chrome", None)), 0, false, false);
        assert_eq!(b.observe(0, None, 0, true, false), None);
    }

    #[test]
    fn title_change_splits_only_when_capture_enabled() {
        // capture off: title change does not split.
        let mut off = SessionBuilder::new(120_000, false);
        off.observe(0, Some(win("chrome", Some("A"))), 0, false, false);
        assert_eq!(
            off.observe(1000, Some(win("chrome", Some("B"))), 0, false, false),
            None
        );

        // capture on: title change closes the previous session. Session "A" is
        // extended first so it is not dropped as a zero-length session.
        let mut on = SessionBuilder::new(120_000, true);
        on.observe(0, Some(win("chrome", Some("A"))), 0, false, false);
        on.observe(1000, Some(win("chrome", Some("A"))), 0, false, false);
        let ev = on
            .observe(2000, Some(win("chrome", Some("B"))), 0, false, false)
            .unwrap();
        assert_eq!(ev.title.as_deref(), Some("A"));
        assert_eq!(ev.start_ms, 0);
        assert_eq!(ev.end_ms, 1000);
    }
}
