# System Trace - System Design

Author role: System Design Architect. This document turns the decided product
plan into a buildable architecture. Every other role (designer, developer, tester)
must follow this together with `BRAND.md`, `FEATURES.md`, `TECH_STACK.md`,
`CONVENTIONS.md`, and `DECISIONS.md`.

## 1. Goals and Principles (recall)

System Trace fills the market gap of being cross-platform (Windows, macOS, Linux),
privacy-first (all data local, no telemetry), and well-designed at the same time,
and it grows from tracking into limits, blocking, and wellbeing (see
`FEATURES.md`). The architecture must protect these as invariants:

- Local-first: no activity data ever leaves the machine.
- Cross-platform: shared core, swappable per-OS parts. All three ship together.
- Lean background footprint: the collector runs all day; keep CPU and RAM low.
- No emoji anywhere. In the UI, use a JavaScript icon package (lucide-react), not
  emoji characters.
- Dark and light modes everywhere using the Signal palette.

## 2. High-Level Architecture

```
+--------------------------------------------------------------+
|                      System Trace (Tauri 2)                  |
|                                                              |
|  Rust side (core)                    Web side (UI, React/TS) |
|  ----------------                    ---------------------- |
|  Collector (bg thread)   <events>    Dashboard, Reports,     |
|   - OS Watcher (trait)               Settings, Focus, etc.   |
|   - Sampler + Idle logic             - reads via commands    |
|   - Session builder                  - live updates via      |
|        |                               Tauri events          |
|        v                                     ^               |
|  Storage (SQLite, WAL)  ---summaries---------+               |
|   - raw events                                               |
|   - rollups (daily/weekly)                                   |
|        ^                                                     |
|        |                                                     |
|  Aggregator (rollup + retention)                             |
|  Services (Phase 2-4): Limits, Blocker, Wellbeing, Focus     |
+--------------------------------------------------------------+
```

The Rust side owns truth (tracking + database). The web side is a thin, beautiful
client that reads summaries and renders them. They communicate only through Tauri
commands (request/response) and Tauri events (push updates).

## 3. Process and Threading Model

- One process, single instance enforced (a second launch focuses the existing
  window instead of starting again).
- Main thread: Tauri app, window, system tray.
- Collector thread: a dedicated background worker (Tokio task or std thread) that
  runs the sampling loop independently of the UI. The UI window can be closed; the
  collector keeps running in the tray.
- Storage access is serialized through a single writer (one connection in WAL
  mode, or an async pool with a single writer) to avoid lock contention.
- Services (limits/blocker/wellbeing) run as their own scheduled tasks that read
  from storage and act; they never block the collector.

## 4. The Collector (heart of the app)

Loop, once per tick (default tick = 1 second; configurable):

1. Ask the OS Watcher for: active app id, active window title (only if the user
   enabled title capture), and seconds-since-last-input (idle).
2. Determine state: ACTIVE, IDLE, or LOCKED/ASLEEP.
   - Idle: `idle_seconds >= idle_threshold` (default 120s) AND no media playing.
   - Media-aware exception: if audio is playing or a full-screen video is active,
     stay ACTIVE even with no input (fixes the "watching a video counts as idle"
     problem).
   - Locked/asleep: OS reports session locked or the machine slept; treat as not
     using the computer.
3. Maintain a current in-memory session: { app, title?, category, start, last_seen }.
   - If the active app/title is unchanged and state is ACTIVE, extend the session
     (update last_seen).
   - If it changed, or state left ACTIVE, close the open session and queue it as a
     finished event, then open a new session if appropriate.
4. Flush the event queue to SQLite in batches (every flush_interval, default 15s,
   or immediately on app switch). Never write on every tick.

Edge cases the collector must handle:
- Sleep/hibernate/resume: detect the time jump; do not credit the gap as usage.
- Lock/unlock and screensaver: treat locked as inactive.
- Shutdown: flush the open session and queue on exit.
- Crash safety: SQLite WAL mode plus frequent flush means at most ~flush_interval
  of unsaved time is lost.

Sampling cost: reading the foreground window and idle time once per second is
cheap on all three OSes. The expensive part (writing) is batched.

## 5. OS Abstraction (the only platform-specific code)

A single Rust trait keeps the core OS-agnostic:

```rust
pub struct ActiveWindow { pub app_id: String, pub app_name: String, pub title: Option<String> }

pub trait Watcher: Send {
    fn active_window(&mut self) -> Option<ActiveWindow>;
    fn idle_seconds(&mut self) -> u64;
    fn is_media_playing(&mut self) -> bool; // best-effort; false if unknown
    fn session_locked(&mut self) -> bool;   // best-effort
}
```

Implementations:

| OS | Active window / app | Idle | Media | Locked |
|----|--------------------|------|-------|--------|
| Windows | `GetForegroundWindow`, `GetWindowThreadProcessId` -> process exe name; `GetWindowText` for title | `GetLastInputInfo` | audio session via WASAPI/IAudioMeterInformation (best-effort) | `WTSGetActiveConsoleSessionId` / session notifications |
| macOS | `NSWorkspace.frontmostApplication`; title via Accessibility API (needs permission) | `CGEventSource secondsSinceLastEventType` | `now playing` / audio (best-effort) | `CGSessionCopyCurrentDictionary` |
| Linux (X11) | `_NET_ACTIVE_WINDOW` (EWMH), `WM_CLASS`, `_NET_WM_NAME` | `XScreenSaverQueryInfo` | PipeWire/PulseAudio sink check (best-effort) | screensaver/DBus |
| Linux (Wayland) | KDE (KWin Script callback via D-Bus) / GNOME (Focused Window extension or Eval) | GNOME (Mutter IdleMonitor); KDE (defaults to 0 when active window known) | ALSA procfs check (best-effort) | `org.freedesktop.ScreenSaver` D-Bus |

Rule: the trait is implemented per OS behind `#[cfg(target_os = ...)]`. The core
never calls OS APIs directly. Title and media are best-effort; if an OS cannot
provide them, return `None`/`false` and the app still works.

Crates to use: `windows` (Win32), `core-graphics`/`cocoa`/`objc` (macOS),
`x11rb` and `zbus` (Linux).

### 5.1 Linux Wayland Watcher Design & Fallbacks

Wayland sessions enforce strict application isolation. Standard apps cannot query global window focus or titles directly. To implement tracking on Wayland, System Trace uses a D-Bus first strategy.

#### Chosen Approach and Trade-off Analysis

To track the active window on Wayland, two main approaches were evaluated:
- **Option A (D-Bus Interfaces)**: Query GNOME Shell (via Focused Window D-Bus extension or `Eval`) and KDE Plasma (via temporary KWin scripting calling a D-Bus receiver).
- **Option B (Wayland Protocol Extensions)**: Use protocols like `ext-foreground-toplevel-v1` or `wlr-foreign-toplevel-management`.

**Trade-offs & Decision**:
- **Compatibility**: Option A (D-Bus-first) has maximum compatibility with standard Linux distributions (Fedora, Ubuntu, etc.) since they ship GNOME and KDE Plasma by default. Option B lacks widespread adoption on mainstream environments; notably, GNOME (Mutter) does not implement the `wlr-foreign-toplevel-management` or foreground toplevel protocols.
- **Complexity**: Option A requires writing different logic paths for different desktop environments (GNOME Shell extensions/D-Bus vs. KWin Scripting). Option B is theoretically cleaner and compositor-agnostic where supported (e.g., wlroots/sway/Hyprland), but offers no path on standard Ubuntu/Fedora setups.
- **Conclusion**: We selected Option A (D-Bus-first) to ensure that the core target audience of standard Linux desktop users (Ubuntu/Fedora/openSUSE) is supported, while falling back gracefully to X11 or degraded tracking on unsupported compositors.

1. **GNOME Shell**:
   - First checks the `org.gnome.Shell.extensions.FocusedWindow` D-Bus extension (a popular user extension exposing the focused window metadata).
   - If not installed, falls back to `org.gnome.Shell.Eval` D-Bus method to execute a private JavaScript snippet to query the active window class and title. Note that `Eval` is restricted on GNOME 41+ unless "unsafe mode" is explicitly enabled.
   - **Known Limitations**: On a stock, out-of-the-box GNOME Wayland session (e.g. default Fedora or Ubuntu setups) without the `FocusedWindow` extension, `active_window()` will return `None` (resulting in no active window tracking). Tracking works when the extension is active, or on KDE Plasma.
   
2. **KDE Plasma (KWin)**:
   - On watcher startup, the application registers a local D-Bus receiver (`org.system_trace.Receiver`).
   - It writes and registers a temporary KWin Script that listens to window activation signals (`workspace.windowActivated` / `workspace.clientActivated`) and calls back to the receiver whenever the active window changes.

3. **Runtime Routing Matrix**:
   - On startup, the application reads the `XDG_SESSION_TYPE` environment variable (falling back to checking `WAYLAND_DISPLAY` and `DISPLAY`).
   - If `wayland` is detected, it runs the `WaylandWatcher`.
   - If `x11` is detected, it falls back to the `X11Watcher`.
   - If neither, it degrades gracefully.

4. **Idle & Lock Handling**:
   - GNOME Wayland idle time is queried via `org.gnome.Mutter.IdleMonitor` D-Bus interface.
   - KDE Wayland defaults to `0` (active) when a foreground window is known to prevent false-idle conditions since there is no stable public idle D-Bus API.
   - Screen locking is detected by checking the `org.freedesktop.ScreenSaver` D-Bus interface.

## 6. Data Model (SQLite)

Identifiers are integers; timestamps are UTC unix-millis. Times shown in the UI
are converted to local time there.

**Encryption at rest (since 0.4.0):** the live database runs **in memory**
(`sqlite3_deserialize` from disk on launch). Only **encrypted** snapshots
(XChaCha20-Poly1305 over `sqlite3_serialize` output) are written to disk -
periodically (~2 min), on a data wipe, and on exit - so no plaintext database
file exists at rest. The 32-byte key is stored in the OS credential store
(Windows Credential Manager / macOS Keychain / Linux Secret Service) via the
`keyring` crate, with a permission-restricted key-file fallback. A pre-0.4.0
plaintext database is migrated on first launch and the old file removed. (Test
mode uses a plaintext file DB to keep the E2E harness free of the keyring.)

```sql
-- Apps seen on this machine (deduplicated)
CREATE TABLE app (
  id            INTEGER PRIMARY KEY,
  app_key       TEXT NOT NULL UNIQUE,   -- stable id, e.g. exe name / bundle id
  display_name  TEXT NOT NULL,
  category_id   INTEGER REFERENCES category(id)
);

-- User-editable categories (neutral: Work, Social, Entertainment, Dev, ...)
CREATE TABLE category (
  id            INTEGER PRIMARY KEY,
  name          TEXT NOT NULL UNIQUE,
  color         TEXT,                   -- optional hex for charts
  productive    INTEGER                 -- nullable; only used if scoring enabled
);

-- Raw usage events (one finished active session each)
CREATE TABLE event (
  id            INTEGER PRIMARY KEY,
  app_id        INTEGER NOT NULL REFERENCES app(id),
  title         TEXT,                   -- null unless title capture enabled
  start_ms      INTEGER NOT NULL,
  end_ms        INTEGER NOT NULL,
  duration_ms   INTEGER NOT NULL
);
CREATE INDEX idx_event_start ON event(start_ms);
CREATE INDEX idx_event_app   ON event(app_id);

-- Rolled-up daily totals per app (fast dashboard reads, survives raw trimming)
CREATE TABLE daily_app_usage (
  day           TEXT NOT NULL,          -- 'YYYY-MM-DD' in local time
  app_id        INTEGER NOT NULL REFERENCES app(id),
  total_ms      INTEGER NOT NULL,
  PRIMARY KEY (day, app_id)
);

-- Key-value settings (theme, idle_threshold, capture_titles, retention_days, ...)
CREATE TABLE setting ( key TEXT PRIMARY KEY, value TEXT NOT NULL );

-- Apps/windows the user excludes from tracking
CREATE TABLE exclusion ( id INTEGER PRIMARY KEY, match_type TEXT, pattern TEXT );

-- Phase 2: per-app daily limits
CREATE TABLE app_limit (
  app_id        INTEGER PRIMARY KEY REFERENCES app(id),
  daily_ms      INTEGER NOT NULL,
  strictness    TEXT NOT NULL           -- 'soft' | 'medium' | 'strict'
);

-- Phase 2: blocklists for focus mode
CREATE TABLE block_rule (
  id            INTEGER PRIMARY KEY,
  kind          TEXT NOT NULL,          -- 'app' | 'website'
  pattern       TEXT NOT NULL,
  schedule      TEXT                    -- optional cron-like window
);
```

Schema is created and migrated by a small versioned migration runner (a
`schema_version` setting + ordered migration steps).

## 7. Aggregation and Retention

- After each flush, increment `daily_app_usage` for the affected day(s) and app(s)
  so the dashboard reads summaries, not raw rows.
- A daily maintenance task: finalize yesterday's rollups and trim `event` rows
  older than `retention_days` (default 90). Summaries are kept forever.
- All dashboard queries read `daily_app_usage` (and `event` only for the
  drill-down "today, by hour" view inside the retention window).

## 8. IPC: Commands and Events

Commands (UI calls Rust; all return typed results, never raw DB rows):

- `get_today_overview()` -> { total_ms, top_apps[], by_category[], by_hour[] }
- `get_day_overview(day)` -> same shape as today, for any past day (Reports Day mode)
- `get_range_overview(from, to)` -> aggregates for any range (Reports Week / Month mode)
- `get_apps()` / `set_app_category(app_id, category_id)`
- `get_categories()` / `upsert_category(...)` / `delete_category(id)`
- `get_settings()` / `set_setting(key, value)`
- `get_exclusions()` / `add_exclusion(...)` / `remove_exclusion(id)`
- `export_data(format)` -> writes CSV/JSON to a user-chosen path
- `import_data(path)` -> merge another machine's export
- `wipe_all_data()` -> delete everything (with confirm in UI)
- Phase 2+: `get_limits()/set_limit(...)`, `get_block_rules()/set_block_rule(...)`,
  `start_focus_session(...)`, `get_focus_state()`

Events (Rust pushes to UI):
- `usage_tick` -> lightweight live update (current app, today total) for a live
  dashboard, throttled (e.g. every 5s), so the UI never polls.
- `limit_reached`, `break_due`, `focus_ended` (later phases).

## 9. Frontend Architecture (React + TypeScript)

- Vite + React + TypeScript inside the Tauri webview.
- Routing: a simple client router (Dashboard, Apps, Reports, Focus, Settings).
- Data layer: a typed `api.ts` wrapping `invoke`; React Query (or a small custom
  hook) for caching and refetch; subscribe to `usage_tick` for live numbers.
- Charts: uPlot for the time-series (fast with many points); simple bar/donut for
  top apps and categories.
- Theming: a ThemeProvider reading the `theme` setting (system | dark | light),
  exposing the Signal palette as CSS variables / Tailwind tokens.
- Icons: lucide-react ONLY. No emoji characters anywhere in the UI or copy.
- State that lives in the UI is minimal (filters, selected range, theme); all real
  data comes from the Rust side.

Screen inventory (MVP): Dashboard (hero "Screen Time Today", stat cards, daily
time-series, top apps, category split), Apps (list + re-categorize), Reports
(Day / Week / Month history explorer with prev / next stepper, reset-to-present
chip, and keyboard arrow navigation), Settings (theme, idle threshold, title
capture toggle, retention, exclusions, export/import, wipe), plus Onboarding
(permissions + intro).

## 10. Phase 2-4 Architecture Hooks (designed now, built later)

- Limits engine: a task that compares today's `daily_app_usage` against
  `app_limit`; when exceeded, emits `limit_reached` and applies the chosen
  strictness (notification, dismissible overlay, or a strict lockable block).
- Blocker: focus mode. Leaning to the system hosts-file method for websites (works
  across all browsers) plus app-level blocking by process. 
  
  **Linux Elevation Flow**:
  - Updating `/etc/hosts` requires root privileges.
  - The app stages the managed hosts content to a private, unpredictably-named
    temp file (mode `0600`, created with `O_EXCL`, under `$XDG_RUNTIME_DIR` when
    available) so a local user cannot pre-create, read, or swap it in before the
    privileged copy runs. It then uses `pkexec` (part of Polkit) to spawn a GUI
    password prompt and `cp` that file over `/etc/hosts`; the temp file is removed
    afterwards.
  - If `pkexec` is missing (exit code 127) or cancelled (exit code 126), the app
    returns a clear error to the UI.
  - **SELinux (Fedora/RHEL)**: If writes fail despite a successful prompt, check
    `journalctl -t setroubleshoot`. The hosts path is standard, but policy
    enforcement may vary.
  - **Manual fallback**: Users without `pkexec` can edit `/etc/hosts` directly with
    `sudo` and add/remove the `# >>> System Trace block (managed) >>>` … `<<<` section.

  Designed as a separate service with a clear, auditable interface; off by default.
- Wellbeing scheduler: timers for eye/posture breaks (a dedicated always-on-top
  overlay window), and a quiet-hours/bedtime mode (reduced nudges; best-effort
  OS grayscale where allowed).
- Focus sessions: a manual timer that can turn on the blocker and breaks for a set
  duration.

These read/write the same SQLite and use the same event channel; they must not
slow the collector.

## 11. Privacy, Security, Reliability

- Data location: OS app-data dir (e.g. `%APPDATA%/SystemTrace`, `~/Library/
  Application Support/SystemTrace`, `~/.local/share/system-trace`).
- No network calls for activity data. No telemetry. Optional crash reporting would
  be opt-in only and is not in MVP.
- Optional at-rest encryption (SQLCipher) is a later option, not MVP.
- Export (CSV/JSON), import/merge, and one-click wipe are first-class.
- Exclusions remove matching apps/titles before they are ever written.
- Single instance, autostart toggle, WAL + frequent flush for crash safety, and
  correct sleep/lock handling.

## 12. Project Structure (app/ repo)

```
app/
  package.json, pnpm-lock.yaml, vite.config.ts, tsconfig.json
  tailwind.config.ts, postcss.config.js
  index.html
  src/                      # React + TS
    main.tsx, App.tsx
    theme/                  # ThemeProvider, palette tokens
    lib/api.ts, lib/types.ts
    components/             # Sidebar, Topbar, StatCard, Charts, ...
    pages/                  # Dashboard, Apps, Reports, Focus, Settings, Onboarding
  src-tauri/                # Rust
    Cargo.toml, tauri.conf.json, build.rs
    icons/                  # generated by `tauri icon`
    src/
      main.rs               # app setup, tray, single instance, spawn collector
      commands.rs           # #[tauri::command] handlers
      collector/            # sampler, session builder, idle logic
      platform/             # mod.rs (trait) + windows.rs, macos.rs, linux.rs
      db/                   # connection, migrations, queries
      aggregate.rs          # rollups + retention
      models.rs             # shared structs (serde)
      services/             # limits, blocker, wellbeing, focus (later phases)
```

## 13. Testing Strategy

- Rust unit tests (`cargo test`): idle/state machine, session builder, aggregation
  math, retention trimming, migrations. These are pure and fast.
- Playwright (via tauri-driver): drive the real app window; assert the dashboard
  renders, theme toggles, settings persist, and each MVP flow works.
- A test seam: the collector takes a `Watcher` trait object, so tests inject a fake
  watcher with scripted windows/idle to verify sessions and rollups deterministically.

## 14. Build and Release (reference)

- Dev: `pnpm tauri dev`. Build: `pnpm tauri build` per OS.
- Icons: `pnpm tauri icon assets/logo/system-trace-dark.svg`.
- CI: GitHub Actions matrix (Windows/macOS/Linux) runs lint, `cargo test`, build,
  and Playwright. Release attaches installers to GitHub Releases (see
  `DISTRIBUTION.md`). All three OS ship together.

## 15. Data Flow (one cycle)

```
OS -> Watcher.active_window()/idle -> Sampler decides state
   -> Session builder extends or closes a session
   -> on close: event queued
   -> flush (batch) -> SQLite event table
   -> aggregator updates daily_app_usage
   -> command get_today_overview() reads summaries
   -> UI renders charts; usage_tick keeps the hero number live
```

## 16. Open Implementation Notes

- Wayland active-window is the hardest; X11 first, Wayland via portals before the
  Linux release. Degrade gracefully (track app even if title is unavailable).
- macOS title capture needs Accessibility permission; the onboarding must request
  it and the app must work (app-level) even if denied.
- Keep the collector's per-tick work allocation-free where practical.
```
