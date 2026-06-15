//! Local SQLite storage, migrations, aggregation, and data import/export
//! (SYSTEM_DESIGN.md sections 6-7, 11).
//!
//! One writer (the caller wraps the `Connection` in a `Mutex`). WAL is enabled at
//! open. Raw `event` rows feed `daily_app_usage` rollups; the dashboard reads the
//! rollups so it stays fast and survives retention trimming.

use crate::collector::RawEvent;
use crate::models::*;
use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone, Timelike};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeMap;
use std::path::Path;

pub type DbResult<T> = Result<T, String>;

fn map_err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/* ----------------------------- open + migrate ----------------------------- */

/// Open (or create) the database at `path`, enable WAL, run migrations, and seed
/// default categories and settings.
pub fn open(path: &std::path::Path) -> DbResult<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(map_err)?;
    }
    let conn = Connection::open(path).map_err(map_err)?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(map_err)?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(map_err)?;
    migrate(&conn)?;
    Ok(conn)
}

/// Open the live database **in memory**, loading it from the encrypted snapshot
/// at `enc_path` if one exists, or one-time-migrating a legacy plaintext file at
/// `legacy_plain`. Migrations then run as usual. Because the working database
/// only ever lives in memory, no plaintext database is written to disk.
pub fn open_encrypted(
    enc_path: &std::path::Path,
    key: &[u8; 32],
    legacy_plain: &std::path::Path,
) -> DbResult<Connection> {
    let mut conn = Connection::open_in_memory().map_err(map_err)?;
    if enc_path.exists() {
        let blob = std::fs::read(enc_path).map_err(map_err)?;
        let bytes = crate::crypto::decrypt(key, &blob)?;
        deserialize_into(&mut conn, &bytes)?;
    } else if legacy_plain.exists() {
        // Pre-encryption installs have a plaintext SQLite file; its raw bytes
        // are a valid serialized database, so load and (later) re-encrypt them.
        let bytes = std::fs::read(legacy_plain).map_err(map_err)?;
        if !bytes.is_empty() {
            deserialize_into(&mut conn, &bytes)?;
        }
    }
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(map_err)?;
    migrate(&conn)?;
    Ok(conn)
}

/// Load a serialized SQLite image (`bytes`) into an open in-memory connection.
/// `sqlite3_deserialize` takes ownership of an `sqlite3_malloc`-allocated buffer,
/// so we allocate one, copy the bytes in, and hand it over.
fn deserialize_into(conn: &mut Connection, bytes: &[u8]) -> DbResult<()> {
    use std::ptr::NonNull;
    unsafe {
        let sz = bytes.len();
        let ptr = rusqlite::ffi::sqlite3_malloc64(sz as u64) as *mut u8;
        let nn = NonNull::new(ptr).ok_or_else(|| "sqlite3_malloc failed".to_string())?;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, sz);
        let owned = rusqlite::serialize::OwnedData::from_raw_nonnull(nn, sz);
        conn.deserialize(rusqlite::DatabaseName::Main, owned, false)
            .map_err(map_err)?;
    }
    Ok(())
}

/// Write an encrypted snapshot of the in-memory database to `enc_path`. Atomic:
/// writes a temp sibling then renames over the target so a crash mid-write can
/// never leave a half-written (unreadable) snapshot.
pub fn snapshot_encrypted(
    conn: &Connection,
    enc_path: &std::path::Path,
    key: &[u8; 32],
) -> DbResult<()> {
    let data = conn
        .serialize(rusqlite::DatabaseName::Main)
        .map_err(map_err)?;
    let blob = crate::crypto::encrypt(key, &data)?;
    if let Some(dir) = enc_path.parent() {
        std::fs::create_dir_all(dir).map_err(map_err)?;
    }
    let tmp = enc_path.with_extension("enc.tmp");
    std::fs::write(&tmp, &blob).map_err(map_err)?;
    std::fs::rename(&tmp, enc_path).map_err(map_err)?;
    Ok(())
}

/// Remove a stale quarantined snapshot left behind from a
/// previous recovery.
pub fn prune_corrupt_snapshot(enc_path: &Path) -> DbResult<()> {
    let corrupt = enc_path.with_extension("enc.corrupt");

    if corrupt.exists() {
        std::fs::remove_file(&corrupt).map_err(map_err)?;
    }

    Ok(())
}

/// Idempotent schema creation. Versioned via the `setting` table.
pub fn migrate(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS category (
            id         INTEGER PRIMARY KEY,
            name       TEXT NOT NULL UNIQUE,
            color      TEXT,
            productive INTEGER
        );
        CREATE TABLE IF NOT EXISTS app (
            id           INTEGER PRIMARY KEY,
            app_key      TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            category_id  INTEGER REFERENCES category(id) ON DELETE SET NULL
        );
        CREATE TABLE IF NOT EXISTS event (
            id          INTEGER PRIMARY KEY,
            app_id      INTEGER NOT NULL REFERENCES app(id) ON DELETE CASCADE,
            title       TEXT,
            start_ms    INTEGER NOT NULL,
            end_ms      INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_event_start ON event(start_ms);
        CREATE INDEX IF NOT EXISTS idx_event_app   ON event(app_id);
        CREATE TABLE IF NOT EXISTS daily_app_usage (
            day      TEXT NOT NULL,
            app_id   INTEGER NOT NULL REFERENCES app(id) ON DELETE CASCADE,
            total_ms INTEGER NOT NULL,
            PRIMARY KEY (day, app_id)
        );
        CREATE TABLE IF NOT EXISTS setting (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS exclusion (
            id         INTEGER PRIMARY KEY,
            match_type TEXT NOT NULL,
            pattern    TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS app_limit (
            app_id     INTEGER PRIMARY KEY REFERENCES app(id) ON DELETE CASCADE,
            daily_ms   INTEGER NOT NULL,
            strictness TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS block_rule (
            id               INTEGER PRIMARY KEY,
            kind             TEXT NOT NULL,
            pattern          TEXT NOT NULL,
            enabled          INTEGER NOT NULL DEFAULT 1,
            schedule_enabled INTEGER NOT NULL DEFAULT 0,
            schedule_start   INTEGER,
            schedule_end     INTEGER
        );
        CREATE TABLE IF NOT EXISTS category_goal (
            category_id    INTEGER PRIMARY KEY REFERENCES category(id) ON DELETE CASCADE,
            daily_ms       INTEGER NOT NULL,
            kind           TEXT NOT NULL CHECK (kind IN ('under','over'))
        );
        CREATE TABLE IF NOT EXISTS app_goal (
            app_id    INTEGER PRIMARY KEY REFERENCES app(id) ON DELETE CASCADE,
            daily_ms  INTEGER NOT NULL,
            kind      TEXT NOT NULL CHECK (kind IN ('under','over'))
        );
        CREATE TABLE IF NOT EXISTS focus_session (
            id        INTEGER PRIMARY KEY,
            start_ms  INTEGER NOT NULL,
            end_ms    INTEGER NOT NULL,
            note      TEXT
        );
        "#,
    )
    .map_err(map_err)?;

    // Additive migrations for upgraded installs. SQLite returns
    // "duplicate column" when the column already exists - ignore that case so
    // running migrate() on a fresh DB and an old DB both succeed.
    for stmt in [
        "ALTER TABLE block_rule ADD COLUMN schedule_enabled INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE block_rule ADD COLUMN schedule_start INTEGER",
        "ALTER TABLE block_rule ADD COLUMN schedule_end INTEGER",
        // Executable / bundle path, used to extract the app's real OS icon.
        "ALTER TABLE app ADD COLUMN exe_path TEXT",
    ] {
        if let Err(e) = conn.execute(stmt, []) {
            let msg = e.to_string();
            if !msg.contains("duplicate column") && !msg.contains("already exists") {
                return Err(map_err(e));
            }
        }
    }

    // Seed default neutral categories once.
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM category", [], |r| r.get(0))
        .map_err(map_err)?;
    if count == 0 {
        let defaults = [
            ("Work", "#2DD4BF"),
            ("Communication", "#0EA5A0"),
            ("Development", "#34D399"),
            ("Social", "#F59E0B"),
            ("Entertainment", "#F87171"),
            ("Reading", "#8B949E"),
            ("Other", "#656D76"),
        ];
        for (name, color) in defaults {
            conn.execute(
                "INSERT INTO category (name, color, productive) VALUES (?1, ?2, NULL)",
                params![name, color],
            )
            .map_err(map_err)?;
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO setting (key, value) VALUES ('schema_version', '1')",
        [],
    )
    .map_err(map_err)?;
    Ok(())
}

/* ------------------------------ time helpers ------------------------------ */

/// Local calendar day ('YYYY-MM-DD') for a UTC unix-millis timestamp.
pub fn local_day(ms: i64) -> String {
    DateTime::from_timestamp_millis(ms)
        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

/// Local hour-of-day (0..23) for a UTC unix-millis timestamp.
fn local_hour(ms: i64) -> u8 {
    DateTime::from_timestamp_millis(ms)
        .map(|dt| dt.with_timezone(&Local).hour() as u8)
        .unwrap_or(0)
}

/// UTC unix-millis bounds [start, end) of a local calendar date.
fn day_bounds(date: NaiveDate) -> (i64, i64) {
    let midnight = date.and_hms_opt(0, 0, 0).unwrap();
    let start = match Local.from_local_datetime(&midnight) {
        chrono::LocalResult::Single(dt) => dt.timestamp_millis(),
        // Clocks fell back: midnight happened twice; the earlier instant is the
        // true start of the day.
        chrono::LocalResult::Ambiguous(earlier, _later) => earlier.timestamp_millis(),
        // Clocks sprang forward *at* midnight, so 00:00 never existed locally.
        // Use the first instant that did - one hour later - instead of falling
        // back to the Unix epoch (which made the day's drill-down scan from
        // 1970 and mix in unrelated events).
        chrono::LocalResult::None => Local
            .from_local_datetime(&(midnight + Duration::hours(1)))
            .earliest()
            .map(|dt| dt.timestamp_millis())
            .unwrap_or_else(|| midnight.and_utc().timestamp_millis()),
    };
    let end = start + Duration::days(1).num_milliseconds();
    (start, end)
}

fn parse_day(s: &str) -> DbResult<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(map_err)
}

fn today_key() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/* ------------------------------ write events ------------------------------ */

/// Persist a batch of finished sessions: upsert the app, insert the event, and
/// fold the duration into the daily rollup.
pub fn insert_events(conn: &Connection, events: &[RawEvent]) -> DbResult<()> {
    if events.is_empty() {
        return Ok(());
    }
    let tx = conn.unchecked_transaction().map_err(map_err)?;
    for ev in events {
        let duration = (ev.end_ms - ev.start_ms).max(0);
        if duration == 0 {
            continue;
        }
        tx.execute(
            "INSERT INTO app (app_key, display_name) VALUES (?1, ?2)
             ON CONFLICT(app_key) DO UPDATE SET display_name = excluded.display_name",
            params![ev.app_key, ev.app_name],
        )
        .map_err(map_err)?;
        let app_id: i64 = tx
            .query_row(
                "SELECT id FROM app WHERE app_key = ?1",
                params![ev.app_key],
                |r| r.get(0),
            )
            .map_err(map_err)?;
        tx.execute(
            "INSERT INTO event (app_id, title, start_ms, end_ms, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![app_id, ev.title, ev.start_ms, ev.end_ms, duration],
        )
        .map_err(map_err)?;
        let day = local_day(ev.start_ms);
        tx.execute(
            "INSERT INTO daily_app_usage (day, app_id, total_ms) VALUES (?1, ?2, ?3)
             ON CONFLICT(day, app_id) DO UPDATE SET total_ms = total_ms + excluded.total_ms",
            params![day, app_id, duration],
        )
        .map_err(map_err)?;
    }
    tx.commit().map_err(map_err)?;
    Ok(())
}

/// (today, total active ms today) from the rollups. Used by the live tick.
pub fn today_total(conn: &Connection) -> DbResult<(String, i64)> {
    let day = today_key();
    let total: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total_ms), 0) FROM daily_app_usage WHERE day = ?1",
            params![day],
            |r| r.get(0),
        )
        .map_err(map_err)?;
    Ok((day, total))
}

/// Total active ms for a specific local day key (used by the daily summary).
pub fn day_total(conn: &Connection, day: &str) -> DbResult<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(total_ms), 0) FROM daily_app_usage WHERE day = ?1",
        params![day],
        |r| r.get(0),
    )
    .map_err(map_err)
}

/// Total active ms across an inclusive local-day range (used by the weekly
/// summary). Day keys are 'YYYY-MM-DD', so string comparison is chronological.
pub fn range_total(conn: &Connection, from_day: &str, to_day: &str) -> DbResult<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(total_ms), 0) FROM daily_app_usage WHERE day >= ?1 AND day <= ?2",
        params![from_day, to_day],
        |r| r.get(0),
    )
    .map_err(map_err)
}

/// Delete raw events older than `retention_days`. Rollups are kept forever.
pub fn trim_old_events(conn: &Connection, retention_days: u32) -> DbResult<()> {
    let cutoff = Local::now() - Duration::days(retention_days as i64);
    let cutoff_ms = cutoff.timestamp_millis();
    conn.execute("DELETE FROM event WHERE start_ms < ?1", params![cutoff_ms])
        .map_err(map_err)?;
    Ok(())
}

/* ----------------------------- read: helpers ------------------------------ */

fn usage_entries_for_days(
    conn: &Connection,
    from: &str,
    to: &str,
    limit: i64,
) -> DbResult<Vec<UsageEntry>> {
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.app_key, a.display_name, a.category_id, c.name, c.color,
                    SUM(d.total_ms) AS total
             FROM daily_app_usage d
             JOIN app a ON a.id = d.app_id
             LEFT JOIN category c ON c.id = a.category_id
             WHERE d.day BETWEEN ?1 AND ?2
             GROUP BY a.id
             ORDER BY total DESC
             LIMIT ?3",
        )
        .map_err(map_err)?;
    let rows = stmt
        .query_map(params![from, to, limit], |r| {
            Ok(UsageEntry {
                app_id: r.get(0)?,
                app_key: r.get(1)?,
                display_name: r.get(2)?,
                category_id: r.get(3)?,
                category_name: r.get(4)?,
                color: r.get(5)?,
                total_ms: r.get(6)?,
            })
        })
        .map_err(map_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_err)
}

fn category_usage_for_days(
    conn: &Connection,
    from: &str,
    to: &str,
) -> DbResult<Vec<CategoryUsage>> {
    let mut stmt = conn
        .prepare(
            "SELECT a.category_id, COALESCE(c.name, 'Uncategorized'), c.color, SUM(d.total_ms)
             FROM daily_app_usage d
             JOIN app a ON a.id = d.app_id
             LEFT JOIN category c ON c.id = a.category_id
             WHERE d.day BETWEEN ?1 AND ?2
             GROUP BY a.category_id
             ORDER BY SUM(d.total_ms) DESC",
        )
        .map_err(map_err)?;
    let rows = stmt
        .query_map(params![from, to], |r| {
            Ok(CategoryUsage {
                category_id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                total_ms: r.get(3)?,
            })
        })
        .map_err(map_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_err)
}

/* -------------------------------- goals ----------------------------------- */

pub fn get_category_goals(conn: &Connection) -> DbResult<Vec<CategoryGoal>> {
    let day = today_key();
    let mut stmt = conn
        .prepare(
            "SELECT g.category_id, c.name, c.color, g.daily_ms, g.kind,
                    COALESCE(SUM(d.total_ms), 0) AS today_ms
             FROM category_goal g
             JOIN category c ON c.id = g.category_id
             LEFT JOIN app a ON a.category_id = g.category_id
             LEFT JOIN daily_app_usage d ON d.app_id = a.id AND d.day = ?1
             GROUP BY g.category_id
             ORDER BY c.name COLLATE NOCASE",
        )
        .map_err(map_err)?;
    let rows = stmt
        .query_map(params![day], |r| {
            let kind_str: String = r.get(4)?;
            let kind = match kind_str.as_str() {
                "over" => GoalKind::Over,
                _ => GoalKind::Under,
            };
            Ok(CategoryGoal {
                category_id: r.get(0)?,
                category_name: r.get(1)?,
                color: r.get(2)?,
                daily_ms: r.get(3)?,
                kind,
                today_ms: r.get(5)?,
            })
        })
        .map_err(map_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_err)
}

pub fn set_category_goal(conn: &Connection, input: &CategoryGoalInput) -> DbResult<()> {
    let kind = match input.kind {
        GoalKind::Under => "under",
        GoalKind::Over => "over",
    };
    conn.execute(
        "INSERT INTO category_goal (category_id, daily_ms, kind)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(category_id) DO UPDATE SET daily_ms = excluded.daily_ms, kind = excluded.kind",
        params![input.category_id, input.daily_ms, kind],
    )
    .map_err(map_err)?;
    Ok(())
}

pub fn remove_category_goal(conn: &Connection, category_id: i64) -> DbResult<()> {
    conn.execute(
        "DELETE FROM category_goal WHERE category_id = ?1",
        params![category_id],
    )
    .map_err(map_err)?;
    Ok(())
}

/// Consecutive days an app goal has been met, bounded by tracked history so a
/// brand-new "under" goal doesn't report a streak stretching into empty days.
fn app_goal_streak(
    conn: &Connection,
    app_id: i64,
    daily_ms: i64,
    kind: GoalKind,
    earliest_day: Option<&str>,
) -> DbResult<i64> {
    let mut streak = 0i64;
    for offset in 0..366 {
        let day = (Local::now() - Duration::days(offset))
            .format("%Y-%m-%d")
            .to_string();
        match earliest_day {
            Some(first) if day.as_str() >= first => {}
            _ => break,
        }
        let used: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(total_ms), 0) FROM daily_app_usage
                 WHERE app_id = ?1 AND day = ?2",
                params![app_id, day],
                |r| r.get(0),
            )
            .map_err(map_err)?;
        let met = match kind {
            GoalKind::Under => used <= daily_ms,
            GoalKind::Over => used >= daily_ms,
        };
        if !met {
            // Today may be incomplete; for "over" goals don't break the streak
            // just because today's target isn't reached yet.
            if offset == 0 && matches!(kind, GoalKind::Over) {
                continue;
            }
            break;
        }
        streak += 1;
    }
    Ok(streak)
}

pub fn get_app_goals(conn: &Connection) -> DbResult<Vec<AppGoal>> {
    let day = today_key();
    let mut stmt = conn
        .prepare(
            "SELECT g.app_id, a.app_key, a.display_name, g.daily_ms, g.kind,
                    COALESCE(d.total_ms, 0) AS today_ms
             FROM app_goal g
             JOIN app a ON a.id = g.app_id
             LEFT JOIN daily_app_usage d ON d.app_id = g.app_id AND d.day = ?1
             ORDER BY a.display_name COLLATE NOCASE",
        )
        .map_err(map_err)?;
    let base = stmt
        .query_map(params![day], |r| {
            let kind_str: String = r.get(4)?;
            let kind = match kind_str.as_str() {
                "over" => GoalKind::Over,
                _ => GoalKind::Under,
            };
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                kind,
                r.get::<_, i64>(5)?,
            ))
        })
        .map_err(map_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_err)?;

    let earliest_day: Option<String> = conn
        .query_row("SELECT MIN(day) FROM daily_app_usage", [], |r| r.get(0))
        .map_err(map_err)?;

    let mut out = Vec::with_capacity(base.len());
    for (app_id, app_key, display_name, daily_ms, kind, today_ms) in base {
        let streak_days = app_goal_streak(conn, app_id, daily_ms, kind, earliest_day.as_deref())?;
        out.push(AppGoal {
            app_id,
            app_key,
            display_name,
            daily_ms,
            kind,
            today_ms,
            streak_days,
        });
    }
    Ok(out)
}

pub fn set_app_goal(conn: &Connection, input: &AppGoalInput) -> DbResult<()> {
    let kind = match input.kind {
        GoalKind::Under => "under",
        GoalKind::Over => "over",
    };
    conn.execute(
        "INSERT INTO app_goal (app_id, daily_ms, kind)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(app_id) DO UPDATE SET daily_ms = excluded.daily_ms, kind = excluded.kind",
        params![input.app_id, input.daily_ms, kind],
    )
    .map_err(map_err)?;
    Ok(())
}

pub fn remove_app_goal(conn: &Connection, app_id: i64) -> DbResult<()> {
    conn.execute("DELETE FROM app_goal WHERE app_id = ?1", params![app_id])
        .map_err(map_err)?;
    Ok(())
}

/// Roll up today's usage into productive / distracting / neutral buckets based
/// on the category's `productive` flag. Score is 0..=100, defined as
/// productive_ms / (productive_ms + distracting_ms); neutral time does not
/// help or hurt the score. Returns score=0 when nothing scored has been used.
pub fn focus_score_for_day(conn: &Connection, day: &str) -> DbResult<FocusScore> {
    let mut stmt = conn
        .prepare(
            "SELECT c.productive, SUM(d.total_ms)
             FROM daily_app_usage d
             JOIN app a ON a.id = d.app_id
             LEFT JOIN category c ON c.id = a.category_id
             WHERE d.day = ?1
             GROUP BY c.productive",
        )
        .map_err(map_err)?;
    let rows = stmt
        .query_map(params![day], |r| {
            Ok((r.get::<_, Option<bool>>(0)?, r.get::<_, i64>(1)?))
        })
        .map_err(map_err)?;

    let mut productive_ms = 0i64;
    let mut distracting_ms = 0i64;
    let mut neutral_ms = 0i64;
    for row in rows {
        let (flag, total) = row.map_err(map_err)?;
        match flag {
            Some(true) => productive_ms += total,
            Some(false) => distracting_ms += total,
            None => neutral_ms += total,
        }
    }

    let scored = productive_ms + distracting_ms;
    let score = if scored > 0 {
        ((productive_ms as f64 / scored as f64) * 100.0).round() as u8
    } else {
        0
    };

    Ok(FocusScore {
        day: day.to_string(),
        score,
        productive_ms,
        distracting_ms,
        neutral_ms,
    })
}

fn total_for_days(conn: &Connection, from: &str, to: &str) -> DbResult<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(total_ms), 0) FROM daily_app_usage WHERE day BETWEEN ?1 AND ?2",
        params![from, to],
        |r| r.get(0),
    )
    .map_err(map_err)
}

/* ----------------------------- read: overviews ---------------------------- */

pub fn today_overview(conn: &Connection) -> DbResult<TodayOverview> {
    day_overview(conn, &today_key())
}

/// Same shape as `today_overview` but for any arbitrary day key. Used by the
/// Reports view's Day mode for historical drill-down. The `delta_vs_yesterday_ms`
/// field compares to the day immediately preceding `day`. `active_app` is always
/// `None` here; the live value only makes sense for today and is filled in by
/// the command handler when needed.
pub fn day_overview(conn: &Connection, day: &str) -> DbResult<TodayOverview> {
    let date = parse_day(day)?;
    let total_ms = total_for_days(conn, day, day)?;

    let prev = (date - Duration::days(1)).format("%Y-%m-%d").to_string();
    let prev_total = total_for_days(conn, &prev, &prev)?;

    let top_apps = usage_entries_for_days(conn, day, day, 8)?;
    let by_category = category_usage_for_days(conn, day, day)?;

    // by_hour from raw events (start-hour attribution).
    let (start_ms, end_ms) = day_bounds(date);
    let mut hours = vec![0i64; 24];
    let mut app_switches = 0i64;
    let mut longest_session_ms = 0i64;
    let mut longest_session_app: Option<String> = None;
    {
        let mut stmt = conn
            .prepare(
                "SELECT e.start_ms, e.duration_ms, a.display_name
                 FROM event e JOIN app a ON a.id = e.app_id
                 WHERE e.start_ms >= ?1 AND e.start_ms < ?2",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map(params![start_ms, end_ms], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(map_err)?;
        for row in rows {
            let (s, dur, name) = row.map_err(map_err)?;
            app_switches += 1;
            let h = local_hour(s) as usize;
            hours[h] += dur;
            if dur > longest_session_ms {
                longest_session_ms = dur;
                longest_session_app = Some(name);
            }
        }
    }
    let by_hour = hours
        .into_iter()
        .enumerate()
        .map(|(h, active_ms)| HourBucket {
            hour: h as u8,
            active_ms,
        })
        .collect();

    Ok(TodayOverview {
        day: day.to_string(),
        total_ms,
        delta_vs_yesterday_ms: total_ms - prev_total,
        top_apps,
        by_category,
        by_hour,
        app_switches,
        longest_session_ms,
        longest_session_app,
        active_app: None,
    })
}

pub fn range_overview(conn: &Connection, from: &str, to: &str) -> DbResult<RangeOverview> {
    let from_date = parse_day(from)?;
    let to_date = parse_day(to)?;
    if to_date < from_date {
        return Err("range end is before start".into());
    }

    // Day totals from rollups, zero-filled.
    let mut by_day_map: BTreeMap<String, i64> = BTreeMap::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT day, SUM(total_ms) FROM daily_app_usage
                 WHERE day BETWEEN ?1 AND ?2 GROUP BY day",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map(params![from, to], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .map_err(map_err)?;
        for row in rows {
            let (d, t) = row.map_err(map_err)?;
            by_day_map.insert(d, t);
        }
    }
    let mut by_day = Vec::new();
    let mut day_count = 0i64;
    let mut busiest_day: Option<String> = None;
    let mut busiest_val = -1i64;
    let mut cursor = from_date;
    while cursor <= to_date {
        let key = cursor.format("%Y-%m-%d").to_string();
        let total = *by_day_map.get(&key).unwrap_or(&0);
        if total > busiest_val {
            busiest_val = total;
            busiest_day = Some(key.clone());
        }
        by_day.push(DayTotal {
            day: key,
            total_ms: total,
        });
        cursor += Duration::days(1);
        day_count += 1;
    }

    let total_ms = total_for_days(conn, from, to)?;
    let daily_average_ms = if day_count > 0 {
        total_ms / day_count
    } else {
        0
    };
    let top_apps = usage_entries_for_days(conn, from, to, 10)?;
    let by_category = category_usage_for_days(conn, from, to)?;

    // Previous equal-length range for the delta chip.
    let span = (to_date - from_date).num_days() + 1;
    let prev_to = from_date - Duration::days(1);
    let prev_from = prev_to - Duration::days(span - 1);
    let prev_total_ms = total_for_days(
        conn,
        &prev_from.format("%Y-%m-%d").to_string(),
        &prev_to.format("%Y-%m-%d").to_string(),
    )?;

    Ok(RangeOverview {
        from: from.to_string(),
        to: to.to_string(),
        total_ms,
        daily_average_ms,
        by_day,
        top_apps,
        by_category,
        busiest_day: if busiest_val > 0 { busiest_day } else { None },
        prev_total_ms,
    })
}

/* ----------------------------- apps + categories -------------------------- */

/// Remember an app's executable / bundle path (for icon extraction). Only
/// fills it in when we don't already have one, to avoid per-tick writes.
pub fn set_app_path(conn: &Connection, app_key: &str, path: &str) -> DbResult<()> {
    conn.execute(
        "UPDATE app SET exe_path = ?2
         WHERE app_key = ?1 AND (exe_path IS NULL OR exe_path = '')",
        params![app_key, path],
    )
    .map_err(map_err)?;
    Ok(())
}

/// The stored executable / bundle path for an app, if known.
pub fn get_app_path(conn: &Connection, app_key: &str) -> DbResult<Option<String>> {
    let mut stmt = conn
        .prepare("SELECT exe_path FROM app WHERE app_key = ?1")
        .map_err(map_err)?;
    let mut rows = stmt.query(params![app_key]).map_err(map_err)?;
    match rows.next().map_err(map_err)? {
        Some(row) => row.get::<_, Option<String>>(0).map_err(map_err),
        None => Ok(None),
    }
}

pub fn get_apps(conn: &Connection) -> DbResult<Vec<AppInfo>> {
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.app_key, a.display_name, a.category_id, c.name
             FROM app a LEFT JOIN category c ON c.id = a.category_id
             ORDER BY a.display_name COLLATE NOCASE",
        )
        .map_err(map_err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok(AppInfo {
                id: r.get(0)?,
                app_key: r.get(1)?,
                display_name: r.get(2)?,
                category_id: r.get(3)?,
                category_name: r.get(4)?,
            })
        })
        .map_err(map_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_err)
}

pub fn set_app_category(conn: &Connection, app_id: i64, category_id: Option<i64>) -> DbResult<()> {
    conn.execute(
        "UPDATE app SET category_id = ?1 WHERE id = ?2",
        params![category_id, app_id],
    )
    .map_err(map_err)?;
    Ok(())
}

pub fn get_categories(conn: &Connection) -> DbResult<Vec<Category>> {
    let mut stmt = conn
        .prepare("SELECT id, name, color, productive FROM category ORDER BY name COLLATE NOCASE")
        .map_err(map_err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Category {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                productive: r.get(3)?,
            })
        })
        .map_err(map_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_err)
}

pub fn upsert_category(conn: &Connection, input: &CategoryInput) -> DbResult<Category> {
    let id: i64 = match input.id {
        Some(id) => {
            conn.execute(
                "UPDATE category SET name = ?1, color = ?2, productive = ?3 WHERE id = ?4",
                params![input.name, input.color, input.productive, id],
            )
            .map_err(map_err)?;
            id
        }
        None => {
            conn.execute(
                "INSERT INTO category (name, color, productive) VALUES (?1, ?2, ?3)",
                params![input.name, input.color, input.productive],
            )
            .map_err(map_err)?;
            conn.last_insert_rowid()
        }
    };
    conn.query_row(
        "SELECT id, name, color, productive FROM category WHERE id = ?1",
        params![id],
        |r| {
            Ok(Category {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                productive: r.get(3)?,
            })
        },
    )
    .map_err(map_err)
}

pub fn delete_category(conn: &Connection, id: i64) -> DbResult<()> {
    conn.execute("DELETE FROM category WHERE id = ?1", params![id])
        .map_err(map_err)?;
    Ok(())
}

/* --------------------------------- settings ------------------------------- */

pub fn get_raw_setting(conn: &Connection, key: &str) -> DbResult<Option<String>> {
    conn.query_row(
        "SELECT value FROM setting WHERE key = ?1",
        params![key],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .map_err(map_err)
}

fn parse_bool(s: &str) -> bool {
    s == "true" || s == "1"
}

pub fn get_settings(conn: &Connection) -> DbResult<Settings> {
    let theme = match get_raw_setting(conn, "theme")?.as_deref() {
        Some("dark") => ThemePreference::Dark,
        Some("light") => ThemePreference::Light,
        _ => ThemePreference::System,
    };
    let get = |k: &str| -> DbResult<Option<String>> { get_raw_setting(conn, k) };
    Ok(Settings {
        theme,
        idle_threshold_secs: get("idle_threshold_secs")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(120),
        capture_titles: get("capture_titles")?
            .map(|v| parse_bool(&v))
            .unwrap_or(false),
        retention_days: get("retention_days")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(90),
        tracking_paused: get("tracking_paused")?
            .map(|v| parse_bool(&v))
            .unwrap_or(false),
        launch_at_login: get("launch_at_login")?
            .map(|v| parse_bool(&v))
            .unwrap_or(false),
        start_minimized: get("start_minimized")?
            .map(|v| parse_bool(&v))
            .unwrap_or(false),
        scoring_enabled: get("scoring_enabled")?
            .map(|v| parse_bool(&v))
            .unwrap_or(false),
        summary_cadence: match get("summary_cadence")?.as_deref() {
            Some("off") => SummaryCadence::Off,
            Some("weekly") => SummaryCadence::Weekly,
            Some("both") => SummaryCadence::Both,
            _ => SummaryCadence::Daily,
        },
        breaks_enabled: get("breaks_enabled")?
            .map(|v| parse_bool(&v))
            .unwrap_or(false),
        break_interval_mins: get("break_interval_mins")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
        break_duration_secs: get("break_duration_secs")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(20),
        break_strict: get("break_strict")?
            .map(|v| parse_bool(&v))
            .unwrap_or(false),
        bedtime_enabled: get("bedtime_enabled")?
            .map(|v| parse_bool(&v))
            .unwrap_or(false),
        bedtime_start: get("bedtime_start")?.unwrap_or_else(|| "22:00".to_string()),
        bedtime_end: get("bedtime_end")?.unwrap_or_else(|| "07:00".to_string()),
        onboarding_complete: get("onboarding_complete")?
            .map(|v| parse_bool(&v))
            .unwrap_or(false),
        distraction_nudges_enabled: get("distraction_nudges_enabled")?
            .map(|v| parse_bool(&v))
            .unwrap_or(false),
        distraction_threshold_mins: get("distraction_threshold_mins")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(20),
        bedtime_grayscale_enabled: get("bedtime_grayscale_enabled")?
            .map(|v| parse_bool(&v))
            .unwrap_or(false),
        palette: get("palette")?.unwrap_or_else(|| "signal".to_string()),
        language: get("language")?.unwrap_or_else(|| "en".to_string()),
    })
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> DbResult<()> {
    conn.execute(
        "INSERT INTO setting (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(map_err)?;
    Ok(())
}

/* -------------------------------- exclusions ------------------------------ */

pub fn get_exclusions(conn: &Connection) -> DbResult<Vec<Exclusion>> {
    let mut stmt = conn
        .prepare("SELECT id, match_type, pattern FROM exclusion ORDER BY id")
        .map_err(map_err)?;
    let rows = stmt
        .query_map([], |r| {
            let mt: String = r.get(1)?;
            Ok(Exclusion {
                id: r.get(0)?,
                match_type: if mt == "title_contains" {
                    ExclusionMatchType::TitleContains
                } else {
                    ExclusionMatchType::App
                },
                pattern: r.get(2)?,
            })
        })
        .map_err(map_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_err)
}

pub fn add_exclusion(conn: &Connection, ex: &NewExclusion) -> DbResult<Exclusion> {
    let mt = match ex.match_type {
        ExclusionMatchType::App => "app",
        ExclusionMatchType::TitleContains => "title_contains",
    };
    conn.execute(
        "INSERT INTO exclusion (match_type, pattern) VALUES (?1, ?2)",
        params![mt, ex.pattern],
    )
    .map_err(map_err)?;
    Ok(Exclusion {
        id: conn.last_insert_rowid(),
        match_type: ex.match_type,
        pattern: ex.pattern.clone(),
    })
}

pub fn remove_exclusion(conn: &Connection, id: i64) -> DbResult<()> {
    conn.execute("DELETE FROM exclusion WHERE id = ?1", params![id])
        .map_err(map_err)?;
    Ok(())
}

/* ------------------------------ export / import --------------------------- */

#[derive(serde::Serialize, serde::Deserialize)]
struct ExportRow {
    app_key: String,
    display_name: String,
    title: Option<String>,
    start_ms: i64,
    end_ms: i64,
}

/// Read raw event rows, optionally bounded to a local-day range [from, to].
/// When both are `None`, returns everything (the original behavior).
fn read_rows_in_range(
    conn: &Connection,
    from: Option<&str>,
    to: Option<&str>,
) -> DbResult<Vec<ExportRow>> {
    // Translate the inclusive local-day range to a UTC-millis window so the
    // event.start_ms comparison is correct regardless of timezone.
    let (lo, hi) = day_range_bounds(from, to)?;
    let mut stmt = conn
        .prepare(
            "SELECT a.app_key, a.display_name, e.title, e.start_ms, e.end_ms
             FROM event e JOIN app a ON a.id = e.app_id
             WHERE e.start_ms >= ?1 AND e.start_ms < ?2
             ORDER BY e.start_ms",
        )
        .map_err(map_err)?;
    let rows = stmt
        .query_map(params![lo, hi], |r| {
            Ok(ExportRow {
                app_key: r.get(0)?,
                display_name: r.get(1)?,
                title: r.get(2)?,
                start_ms: r.get(3)?,
                end_ms: r.get(4)?,
            })
        })
        .map_err(map_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_err)
}

/// Convert an optional inclusive local-day range to a [lo, hi) UTC-millis
/// window. Missing `from` => epoch; missing `to` => far future.
fn day_range_bounds(from: Option<&str>, to: Option<&str>) -> DbResult<(i64, i64)> {
    let lo = match from {
        Some(d) => day_bounds(parse_day(d)?).0,
        None => 0,
    };
    let hi = match to {
        Some(d) => day_bounds(parse_day(d)?).1,
        None => i64::MAX,
    };
    Ok((lo, hi))
}

/* -------------------------------- search ---------------------------------- */

/// Search app usage by name / app_key / title within an optional day range.
/// Groups by app and day so the UI can show "on this day you spent X on Y".
pub fn search_usage(
    conn: &Connection,
    query: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> DbResult<Vec<SearchHit>> {
    let (lo, hi) = day_range_bounds(from, to)?;
    let like = format!("%{}%", query.trim());
    let mut stmt = conn
        .prepare(
            "SELECT a.app_key, a.display_name, e.start_ms, e.duration_ms, e.title
             FROM event e JOIN app a ON a.id = e.app_id
             WHERE e.start_ms >= ?1 AND e.start_ms < ?2
               AND (a.display_name LIKE ?3 OR a.app_key LIKE ?3 OR e.title LIKE ?3)",
        )
        .map_err(map_err)?;
    let rows = stmt
        .query_map(params![lo, hi, like], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(map_err)?;

    // Aggregate (day, app) -> total ms + sample title.
    let mut acc: BTreeMap<(String, String), (String, i64, Option<String>)> = BTreeMap::new();
    for row in rows {
        let (app_key, display_name, start, dur, title) = row.map_err(map_err)?;
        let day = local_day(start);
        let entry = acc.entry((day.clone(), app_key.clone())).or_insert((
            display_name.clone(),
            0,
            title.clone(),
        ));
        entry.1 += dur;
        if entry.2.is_none() {
            entry.2 = title;
        }
    }

    let mut hits: Vec<SearchHit> = acc
        .into_iter()
        .map(
            |((day, app_key), (display_name, total_ms, title))| SearchHit {
                day,
                app_key,
                display_name,
                total_ms,
                sample_title: title,
            },
        )
        .collect();
    // Most time first.
    hits.sort_by_key(|h| std::cmp::Reverse(h.total_ms));
    hits.truncate(200);
    Ok(hits)
}

/* ----------------------------- focus sessions ----------------------------- */

pub fn save_focus_session(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
    note: &str,
) -> DbResult<()> {
    let trimmed = note.trim();
    let note_opt: Option<&str> = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    };
    conn.execute(
        "INSERT INTO focus_session (start_ms, end_ms, note) VALUES (?1, ?2, ?3)",
        params![start_ms, end_ms, note_opt],
    )
    .map_err(map_err)?;
    Ok(())
}

pub fn list_focus_sessions(conn: &Connection, limit: i64) -> DbResult<Vec<FocusSession>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, start_ms, end_ms, note FROM focus_session
             ORDER BY start_ms DESC LIMIT ?1",
        )
        .map_err(map_err)?;
    let rows = stmt
        .query_map(params![limit], |r| {
            Ok(FocusSession {
                id: r.get(0)?,
                start_ms: r.get(1)?,
                end_ms: r.get(2)?,
                note: r.get(3)?,
            })
        })
        .map_err(map_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_err)
}

/* -------------------------------- streaks --------------------------------- */

/// For each category goal, count the run of consecutive days (ending today or
/// yesterday) on which the goal was met. "under" goals are met when the day's
/// category usage was at or below the target; "over" goals when at or above.
/// A day with zero tracked data does not count as met for "over" goals and
/// breaks the streak; for "under" goals an empty day counts as met.
pub fn get_goal_streaks(conn: &Connection) -> DbResult<Vec<GoalStreak>> {
    let goals = get_category_goals(conn)?;

    // Don't count days before tracking began - otherwise a brand-new "under"
    // goal reports a year-long streak from empty history. Bound the walk-back
    // at the earliest day that has any rolled-up usage.
    let earliest_day: Option<String> = conn
        .query_row("SELECT MIN(day) FROM daily_app_usage", [], |r| r.get(0))
        .map_err(map_err)?;

    let mut out = Vec::with_capacity(goals.len());
    for g in goals {
        let mut streak = 0i64;
        // Walk back up to a year; stop at the first day the goal was not met
        // or once we pass the earliest tracked day.
        for offset in 0..366 {
            let day = (Local::now() - Duration::days(offset))
                .format("%Y-%m-%d")
                .to_string();
            if let Some(ref first) = earliest_day {
                if day.as_str() < first.as_str() {
                    break;
                }
            } else {
                // No tracked data at all yet.
                break;
            }
            let used: i64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(d.total_ms), 0)
                     FROM daily_app_usage d
                     JOIN app a ON a.id = d.app_id
                     WHERE a.category_id = ?1 AND d.day = ?2",
                    params![g.category_id, day],
                    |r| r.get(0),
                )
                .map_err(map_err)?;
            let met = match g.kind {
                GoalKind::Under => used <= g.daily_ms,
                GoalKind::Over => used >= g.daily_ms,
            };
            // Today may be incomplete; for "over" goals don't break the streak
            // just because today's target is not reached yet.
            if !met {
                if offset == 0 && matches!(g.kind, GoalKind::Over) {
                    continue;
                }
                break;
            }
            streak += 1;
        }
        out.push(GoalStreak {
            category_id: g.category_id,
            category_name: g.category_name,
            color: g.color,
            kind: g.kind,
            streak_days: streak,
        });
    }
    Ok(out)
}

fn csv_escape(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn export_data(
    conn: &Connection,
    format: ExportFormat,
    path: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> DbResult<ExportResult> {
    let rows = read_rows_in_range(conn, from, to)?;
    let count = rows.len() as i64;
    let body = match format {
        ExportFormat::Json => serde_json::to_string_pretty(&rows).map_err(map_err)?,
        ExportFormat::Csv => {
            let mut out = String::from("app_key,display_name,title,start_ms,end_ms\n");
            for r in &rows {
                out.push_str(&format!(
                    "{},{},{},{},{}\n",
                    csv_escape(&r.app_key),
                    csv_escape(&r.display_name),
                    csv_escape(r.title.as_deref().unwrap_or("")),
                    r.start_ms,
                    r.end_ms
                ));
            }
            out
        }
    };
    std::fs::write(path, body).map_err(map_err)?;
    Ok(ExportResult {
        path: path.to_string(),
        format,
        rows_written: count,
    })
}

pub fn import_data(conn: &Connection, path: &str) -> DbResult<ImportResult> {
    let text = std::fs::read_to_string(path).map_err(map_err)?;
    let rows: Vec<ExportRow> =
        serde_json::from_str(&text).map_err(|_| "import expects a JSON export file".to_string())?;

    let mut days: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let apps_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM app", [], |r| r.get(0))
        .map_err(map_err)?;

    let events: Vec<RawEvent> = rows
        .into_iter()
        .map(|r| {
            days.insert(local_day(r.start_ms));
            RawEvent {
                app_key: r.app_key,
                app_name: r.display_name,
                title: r.title,
                start_ms: r.start_ms,
                end_ms: r.end_ms,
            }
        })
        .collect();
    let merged = events.len() as i64;
    insert_events(conn, &events)?;

    let apps_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM app", [], |r| r.get(0))
        .map_err(map_err)?;

    Ok(ImportResult {
        apps_added: apps_after - apps_before,
        events_merged: merged,
        days_affected: days.len() as i64,
    })
}

pub fn wipe_all_data(conn: &Connection) -> DbResult<WipeResult> {
    conn.execute_batch(
        "DELETE FROM event; DELETE FROM daily_app_usage; DELETE FROM app; DELETE FROM exclusion;",
    )
    .map_err(map_err)?;
    Ok(WipeResult { ok: true })
}

/* --------------------------- phase 2: limits ------------------------------ */

fn parse_strictness(s: &str) -> LimitStrictness {
    match s {
        "soft" => LimitStrictness::Soft,
        "strict" => LimitStrictness::Strict,
        _ => LimitStrictness::Medium,
    }
}

fn strictness_str(s: LimitStrictness) -> &'static str {
    match s {
        LimitStrictness::Soft => "soft",
        LimitStrictness::Medium => "medium",
        LimitStrictness::Strict => "strict",
    }
}

pub fn get_limits(conn: &Connection) -> DbResult<Vec<LimitView>> {
    let day = today_key();
    let mut stmt = conn
        .prepare(
            "SELECT l.app_id, a.app_key, a.display_name, l.daily_ms, l.strictness,
                    COALESCE(d.total_ms, 0) AS used
             FROM app_limit l
             JOIN app a ON a.id = l.app_id
             LEFT JOIN daily_app_usage d ON d.app_id = l.app_id AND d.day = ?1
             ORDER BY a.display_name COLLATE NOCASE",
        )
        .map_err(map_err)?;
    let rows = stmt
        .query_map(params![day], |r| {
            let daily: i64 = r.get(3)?;
            let st: String = r.get(4)?;
            let used: i64 = r.get(5)?;
            Ok(LimitView {
                app_id: r.get(0)?,
                app_key: r.get(1)?,
                display_name: r.get(2)?,
                daily_ms: daily,
                used_ms: used,
                strictness: parse_strictness(&st),
                // A limit of 0 ms (or any non-positive value) is not a real
                // cap, so it must never count as "exceeded" - otherwise a
                // freshly-created or zero limit fires at 0 usage.
                exceeded: daily > 0 && used >= daily,
            })
        })
        .map_err(map_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_err)
}

pub fn set_limit(conn: &Connection, input: &LimitInput) -> DbResult<()> {
    conn.execute(
        "INSERT INTO app_limit (app_id, daily_ms, strictness) VALUES (?1, ?2, ?3)
         ON CONFLICT(app_id) DO UPDATE SET daily_ms = excluded.daily_ms, strictness = excluded.strictness",
        params![input.app_id, input.daily_ms, strictness_str(input.strictness)],
    )
    .map_err(map_err)?;
    Ok(())
}

pub fn remove_limit(conn: &Connection, app_id: i64) -> DbResult<()> {
    conn.execute("DELETE FROM app_limit WHERE app_id = ?1", params![app_id])
        .map_err(map_err)?;
    Ok(())
}

/* --------------------------- phase 2: blocking ---------------------------- */

fn parse_kind(s: &str) -> BlockKind {
    if s == "website" {
        BlockKind::Website
    } else {
        BlockKind::App
    }
}

fn kind_str(k: BlockKind) -> &'static str {
    match k {
        BlockKind::App => "app",
        BlockKind::Website => "website",
    }
}

pub fn get_block_rules(conn: &Connection) -> DbResult<Vec<BlockRule>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, kind, pattern, enabled, schedule_enabled, schedule_start, schedule_end
             FROM block_rule ORDER BY id",
        )
        .map_err(map_err)?;
    let rows = stmt
        .query_map([], |r| {
            let kind: String = r.get(1)?;
            let enabled: i64 = r.get(3)?;
            let sched_enabled: i64 = r.get(4)?;
            Ok(BlockRule {
                id: r.get(0)?,
                kind: parse_kind(&kind),
                pattern: r.get(2)?,
                enabled: enabled != 0,
                schedule_enabled: sched_enabled != 0,
                schedule_start: r.get(5)?,
                schedule_end: r.get(6)?,
            })
        })
        .map_err(map_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_err)
}

pub fn set_block_rule(conn: &Connection, input: &BlockRuleInput) -> DbResult<BlockRule> {
    let id = match input.id {
        Some(id) => {
            conn.execute(
                "UPDATE block_rule SET kind = ?1, pattern = ?2, enabled = ?3,
                 schedule_enabled = ?4, schedule_start = ?5, schedule_end = ?6 WHERE id = ?7",
                params![
                    kind_str(input.kind),
                    input.pattern,
                    input.enabled as i64,
                    input.schedule_enabled as i64,
                    input.schedule_start,
                    input.schedule_end,
                    id
                ],
            )
            .map_err(map_err)?;
            id
        }
        None => {
            conn.execute(
                "INSERT INTO block_rule (kind, pattern, enabled, schedule_enabled,
                 schedule_start, schedule_end) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    kind_str(input.kind),
                    input.pattern,
                    input.enabled as i64,
                    input.schedule_enabled as i64,
                    input.schedule_start,
                    input.schedule_end,
                ],
            )
            .map_err(map_err)?;
            conn.last_insert_rowid()
        }
    };
    Ok(BlockRule {
        id,
        kind: input.kind,
        pattern: input.pattern.clone(),
        enabled: input.enabled,
        schedule_enabled: input.schedule_enabled,
        schedule_start: input.schedule_start,
        schedule_end: input.schedule_end,
    })
}

/// True when a rule is in force right now. Rules with no schedule are always
/// in force when enabled; scheduled rules only fire inside their window.
pub fn rule_in_force_at(rule: &BlockRule, mins_now: i32) -> bool {
    if !rule.enabled {
        return false;
    }
    if !rule.schedule_enabled {
        return true;
    }
    match (rule.schedule_start, rule.schedule_end) {
        (Some(s), Some(e)) if s != e => {
            if s < e {
                mins_now >= s && mins_now < e
            } else {
                // Overnight window (e.g. 22:00 - 07:00).
                mins_now >= s || mins_now < e
            }
        }
        _ => true,
    }
}

pub fn remove_block_rule(conn: &Connection, id: i64) -> DbResult<()> {
    conn.execute("DELETE FROM block_rule WHERE id = ?1", params![id])
        .map_err(map_err)?;
    Ok(())
}

pub fn enabled_block_rules_count(conn: &Connection) -> DbResult<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM block_rule WHERE enabled = 1",
        [],
        |r| r.get(0),
    )
    .map_err(map_err)
}

/// Local-time minutes since midnight, used by `rule_in_force_at`.
fn mins_since_midnight_now() -> i32 {
    use chrono::Timelike;
    let d = chrono::Local::now();
    (d.hour() * 60 + d.minute()) as i32
}

/// Enabled app-kind block patterns currently in force (respects schedules).
pub fn enabled_app_block_patterns(conn: &Connection) -> DbResult<Vec<String>> {
    let mins = mins_since_midnight_now();
    let rules = get_block_rules(conn)?;
    Ok(rules
        .into_iter()
        .filter(|r| matches!(r.kind, BlockKind::App) && rule_in_force_at(r, mins))
        .map(|r| r.pattern)
        .collect())
}

/// Enabled website-kind block patterns currently in force (respects schedules).
pub fn enabled_website_block_patterns(conn: &Connection) -> DbResult<Vec<String>> {
    let mins = mins_since_midnight_now();
    let rules = get_block_rules(conn)?;
    Ok(rules
        .into_iter()
        .filter(|r| matches!(r.kind, BlockKind::Website) && rule_in_force_at(r, mins))
        .map(|r| r.pattern)
        .collect())
}

/// Snapshot the collector uses to keep the system hosts file in sync with
/// website rules each loop. Returns `(feature_in_use, in_force_domains)` where
/// `feature_in_use` is true when at least one website rule is enabled (so the
/// collector knows whether to touch the hosts file at all) and
/// `in_force_domains` are the domains whose schedule is active right now.
pub fn website_block_state(conn: &Connection) -> DbResult<(bool, Vec<String>)> {
    let mins = mins_since_midnight_now();
    let rules = get_block_rules(conn)?;
    let mut in_use = false;
    let mut in_force = Vec::new();
    for r in rules {
        if matches!(r.kind, BlockKind::Website) && r.enabled {
            in_use = true;
            if rule_in_force_at(&r, mins) {
                in_force.push(r.pattern);
            }
        }
    }
    Ok((in_use, in_force))
}

/* ---------------------------------- tests --------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn ev(app: &str, start: i64, end: i64) -> RawEvent {
        RawEvent {
            app_key: format!("{app}.exe"),
            app_name: app.to_string(),
            title: None,
            start_ms: start,
            end_ms: end,
        }
    }

    #[test]
    fn migrate_upgrades_old_database_without_data_loss() {
        // Simulate a v0.1.0-era database: original tables only, block_rule
        // WITHOUT the schedule columns, and no category_goal / focus_session
        // tables. Then run the current migration over it (an in-place upgrade).
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE category (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, color TEXT, productive INTEGER);
            CREATE TABLE app (id INTEGER PRIMARY KEY, app_key TEXT NOT NULL UNIQUE, display_name TEXT NOT NULL, category_id INTEGER);
            CREATE TABLE event (id INTEGER PRIMARY KEY, app_id INTEGER NOT NULL, title TEXT, start_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL, duration_ms INTEGER NOT NULL);
            CREATE TABLE daily_app_usage (day TEXT NOT NULL, app_id INTEGER NOT NULL, total_ms INTEGER NOT NULL, PRIMARY KEY (day, app_id));
            CREATE TABLE setting (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE exclusion (id INTEGER PRIMARY KEY, match_type TEXT NOT NULL, pattern TEXT NOT NULL);
            CREATE TABLE app_limit (app_id INTEGER PRIMARY KEY, daily_ms INTEGER NOT NULL, strictness TEXT NOT NULL);
            CREATE TABLE block_rule (id INTEGER PRIMARY KEY, kind TEXT NOT NULL, pattern TEXT NOT NULL, enabled INTEGER NOT NULL DEFAULT 1);
            "#,
        )
        .unwrap();
        // Seed representative old user data across the core tables.
        conn.execute(
            "INSERT INTO app (app_key, display_name) VALUES ('chrome.exe', 'chrome')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event (app_id, title, start_ms, end_ms, duration_ms) VALUES (1, NULL, 1000, 5000, 4000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO daily_app_usage (day, app_id, total_ms) VALUES ('2026-01-01', 1, 4000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO block_rule (kind, pattern, enabled) VALUES ('app', 'game.exe', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO setting (key, value) VALUES ('idle_threshold_secs', '300')",
            [],
        )
        .unwrap();

        // The upgrade: run the current schema migration in place.
        migrate(&conn).unwrap();

        // 1) Old data is untouched.
        let events: i64 = conn
            .query_row("SELECT COUNT(*) FROM event", [], |r| r.get(0))
            .unwrap();
        assert_eq!(events, 1, "old events must survive the upgrade");
        let usage: i64 = conn
            .query_row(
                "SELECT total_ms FROM daily_app_usage WHERE day = '2026-01-01'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(usage, 4000, "old rollups must survive the upgrade");

        // 2) New columns exist; the pre-existing block_rule row gets the
        //    schedule defaults (disabled, no window).
        let rules = get_block_rules(&conn).unwrap();
        assert_eq!(rules.len(), 1);
        assert!(!rules[0].schedule_enabled);
        assert_eq!(rules[0].schedule_start, None);

        // 3) New tables exist and are usable.
        save_focus_session(&conn, 10, 20, "after upgrade").unwrap();
        assert_eq!(list_focus_sessions(&conn, 10).unwrap().len(), 1);
        assert!(get_category_goals(&conn).unwrap().is_empty());

        // 4) Old settings are preserved; new settings fall back to defaults.
        let s = get_settings(&conn).unwrap();
        assert_eq!(s.idle_threshold_secs, 300, "preserved old setting");
        assert_eq!(s.palette, "signal", "new setting default");
        assert_eq!(s.language, "en", "new setting default");
        assert!(!s.bedtime_grayscale_enabled, "new setting default");

        // 5) Running migrate() AGAIN (e.g. a later restart) is a no-op and
        //    still preserves everything - idempotency.
        migrate(&conn).unwrap();
        let events_again: i64 = conn
            .query_row("SELECT COUNT(*) FROM event", [], |r| r.get(0))
            .unwrap();
        assert_eq!(events_again, 1);
    }

    #[test]
    fn seeds_default_categories() {
        let conn = mem();
        let cats = get_categories(&conn).unwrap();
        assert!(cats.len() >= 5);
    }

    #[test]
    fn insert_and_rollup_today_total() {
        let conn = mem();
        let now = Local::now().timestamp_millis();
        insert_events(
            &conn,
            &[ev("chrome", now, now + 5_000), ev("code", now, now + 3_000)],
        )
        .unwrap();
        let (_, total) = today_total(&conn).unwrap();
        assert_eq!(total, 8_000);
        let ov = today_overview(&conn).unwrap();
        assert_eq!(ov.total_ms, 8_000);
        assert_eq!(ov.app_switches, 2);
        assert_eq!(ov.longest_session_ms, 5_000);
    }

    #[test]
    fn day_overview_picks_arbitrary_past_day() {
        let conn = mem();
        // Seed today and two days ago with different totals.
        let now = Local::now().timestamp_millis();
        let two_days_ago = now - Duration::days(2).num_milliseconds();
        insert_events(
            &conn,
            &[
                ev("chrome", now, now + 4_000),
                ev("code", two_days_ago, two_days_ago + 9_000),
            ],
        )
        .unwrap();
        let past_day = (Local::now() - Duration::days(2))
            .format("%Y-%m-%d")
            .to_string();
        let ov = day_overview(&conn, &past_day).unwrap();
        assert_eq!(ov.day, past_day);
        assert_eq!(ov.total_ms, 9_000);
        assert_eq!(ov.app_switches, 1);
        assert_eq!(ov.longest_session_ms, 9_000);
        assert_eq!(ov.longest_session_app.as_deref(), Some("code"));
        // delta is vs the day before this past day (which has no usage).
        assert_eq!(ov.delta_vs_yesterday_ms, 9_000);
    }

    #[test]
    fn search_groups_by_app_and_day() {
        let conn = mem();
        let now = Local::now().timestamp_millis();
        insert_events(
            &conn,
            &[
                ev("chrome", now, now + 4_000),
                ev("chrome", now + 5_000, now + 9_000),
                ev("code", now, now + 2_000),
            ],
        )
        .unwrap();
        let hits = search_usage(&conn, "chrome", None, None).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].display_name, "chrome");
        // Two chrome events on the same day are summed.
        assert_eq!(hits[0].total_ms, 8_000);

        // A query that matches nothing returns empty.
        let none = search_usage(&conn, "zzz-no-match", None, None).unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn focus_sessions_round_trip() {
        let conn = mem();
        let now = Local::now().timestamp_millis();
        save_focus_session(&conn, now, now + 25 * 60_000, "deep work on auth").unwrap();
        save_focus_session(&conn, now + 1, now + 2, "").unwrap();
        let list = list_focus_sessions(&conn, 10).unwrap();
        assert_eq!(list.len(), 2);
        // Most recent first; the empty note is stored as None.
        assert_eq!(list[0].note, None);
        assert_eq!(list[1].note.as_deref(), Some("deep work on auth"));
    }

    #[test]
    fn export_respects_date_range() {
        let conn = mem();
        let now = Local::now().timestamp_millis();
        let three_days_ago = now - Duration::days(3).num_milliseconds();
        insert_events(
            &conn,
            &[
                ev("chrome", now, now + 4_000),
                ev("code", three_days_ago, three_days_ago + 9_000),
            ],
        )
        .unwrap();
        // Range covering only today should see one row.
        let today = today_key();
        let rows = read_rows_in_range(&conn, Some(&today), Some(&today)).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].app_key, "chrome.exe");
        // No range = everything.
        let all = read_rows_in_range(&conn, None, None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn retention_trims_old_events_but_keeps_rollups() {
        let conn = mem();
        let old = Local::now().timestamp_millis() - Duration::days(200).num_milliseconds();
        insert_events(&conn, &[ev("chrome", old, old + 4_000)]).unwrap();
        trim_old_events(&conn, 90).unwrap();
        let events: i64 = conn
            .query_row("SELECT COUNT(*) FROM event", [], |r| r.get(0))
            .unwrap();
        let rollups: i64 = conn
            .query_row("SELECT COUNT(*) FROM daily_app_usage", [], |r| r.get(0))
            .unwrap();
        assert_eq!(events, 0);
        assert_eq!(rollups, 1);
    }

    #[test]
    fn limits_compute_used_and_exceeded() {
        let conn = mem();
        let now = Local::now().timestamp_millis();
        insert_events(&conn, &[ev("game", now, now + 10_000)]).unwrap();
        let app_id: i64 = conn
            .query_row("SELECT id FROM app WHERE app_key = 'game.exe'", [], |r| {
                r.get(0)
            })
            .unwrap();
        set_limit(
            &conn,
            &LimitInput {
                app_id,
                daily_ms: 5_000,
                strictness: LimitStrictness::Medium,
            },
        )
        .unwrap();
        let limits = get_limits(&conn).unwrap();
        assert_eq!(limits.len(), 1);
        assert_eq!(limits[0].used_ms, 10_000);
        assert!(limits[0].exceeded);
    }

    #[test]
    fn zero_limit_is_never_exceeded() {
        // A 0 ms limit is not a real cap; it must not report exceeded at 0
        // usage (which would fire a "limit reached" nudge immediately).
        let conn = mem();
        conn.execute(
            "INSERT INTO app (app_key, display_name) VALUES ('idle.exe', 'idle')",
            [],
        )
        .unwrap();
        let app_id: i64 = conn
            .query_row("SELECT id FROM app WHERE app_key = 'idle.exe'", [], |r| {
                r.get(0)
            })
            .unwrap();
        set_limit(
            &conn,
            &LimitInput {
                app_id,
                daily_ms: 0,
                strictness: LimitStrictness::Medium,
            },
        )
        .unwrap();
        let limits = get_limits(&conn).unwrap();
        assert_eq!(limits.len(), 1);
        assert_eq!(limits[0].used_ms, 0);
        assert!(!limits[0].exceeded);
    }

    #[test]
    fn encrypted_snapshot_round_trips_and_rejects_wrong_key() {
        let key = [9u8; 32];
        let tmp = std::env::temp_dir().join(format!("st-enc-test-{}.enc", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        let legacy = std::env::temp_dir().join("st-no-such-legacy.sqlite");

        // Fresh in-memory DB, insert some usage, persist an encrypted snapshot.
        {
            let conn = open_encrypted(&tmp, &key, &legacy).unwrap();
            let now = Local::now().timestamp_millis();
            insert_events(&conn, &[ev("chrome", now, now + 5_000)]).unwrap();
            snapshot_encrypted(&conn, &tmp, &key).unwrap();
        }
        // The on-disk snapshot must NOT be a readable SQLite file (it's encrypted).
        let raw = std::fs::read(&tmp).unwrap();
        assert!(&raw[..16.min(raw.len())] != b"SQLite format 3\0");

        // Reopen from the encrypted snapshot: the data survives a round-trip.
        {
            let conn = open_encrypted(&tmp, &key, &legacy).unwrap();
            let total: i64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(total_ms), 0) FROM daily_app_usage",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(total, 5_000);
        }
        // A wrong key cannot open it.
        assert!(open_encrypted(&tmp, &[1u8; 32], &legacy).is_err());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn prune_corrupt_snapshot_removes_stale_quarantine() {
        let base = std::env::temp_dir().join(format!("st-prune-test-{}", std::process::id()));

        let corrupt = base.with_extension("enc.corrupt");

        let _ = std::fs::remove_file(&corrupt);

        std::fs::write(&corrupt, b"old").unwrap();

        assert!(corrupt.exists());

        prune_corrupt_snapshot(&base).unwrap();

        assert!(!corrupt.exists());
    }

    #[test]
    fn app_goals_round_trip() {
        let conn = mem();
        let now = Local::now().timestamp_millis();
        insert_events(&conn, &[ev("game", now, now + 600_000)]).unwrap();
        let app_id: i64 = conn
            .query_row("SELECT id FROM app WHERE app_key = 'game.exe'", [], |r| {
                r.get(0)
            })
            .unwrap();
        set_app_goal(
            &conn,
            &AppGoalInput {
                app_id,
                daily_ms: 1_800_000,
                kind: GoalKind::Under,
            },
        )
        .unwrap();
        let goals = get_app_goals(&conn).unwrap();
        assert_eq!(goals.len(), 1);
        assert_eq!(goals[0].app_id, app_id);
        assert_eq!(goals[0].today_ms, 600_000);
        // Under 30m today, so the goal is met and the streak counts today.
        assert!(goals[0].streak_days >= 1);
        remove_app_goal(&conn, app_id).unwrap();
        assert!(get_app_goals(&conn).unwrap().is_empty());
    }

    #[test]
    fn day_and_range_totals() {
        let conn = mem();
        let now = Local::now().timestamp_millis();
        insert_events(&conn, &[ev("chrome", now, now + 5_000)]).unwrap();
        let day = local_day(now);
        assert_eq!(day_total(&conn, &day).unwrap(), 5_000);
        assert_eq!(range_total(&conn, &day, &day).unwrap(), 5_000);
        assert_eq!(range_total(&conn, "1999-01-01", "1999-01-02").unwrap(), 0);
    }
}
