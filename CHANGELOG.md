# Changelog

All notable changes to System Trace are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing yet. New work in progress lives here until the next tagged release.

## [0.4.0] - 2026-06-14 - Always-visible status messages

A small follow-up release.

### Fixed
- Settings status messages (export, backup and restore, and the update
  check) used to render at the top of the Settings page, so any message
  triggered while scrolled down appeared off-screen and looked like nothing
  had happened. They now show as a fixed bottom-center toast that is always
  visible, and linger a little longer so they are easy to read. This also
  confirms the "Check for updates" result is shown reliably.

## [0.3.0] - 2026-06-13 - Productivity tools, personalization, and data safety

A broad quality release: search, real insight tools, personalization, data
safety, and an important CI fix.

### Added

**Search across history**
- A "Search history" box on the Apps page finds when you used an app across
  all tracked time (matches app name, key, and window title), grouped by day
  and sorted by total time. Backed by a new `search_usage` command.

**App icons**
- Top Apps, the Apps list, and search results now show a per-app icon: a
  deterministic colored letter-avatar derived from the app key. (Real OS icon
  extraction is tracked as a follow-up.)

**Productivity Focus Score** (shipped in 0.2.0) now sits alongside new tools:

**Category goal streaks**
- Each category goal shows a flame badge with its consecutive-days-met streak,
  counted only from the first day with tracked data. New `get_goal_streaks`.

**Manual focus-session annotations**
- While a focus session runs, a note field appears; on stop the session is
  saved with its note. A "Recent sessions" list shows recent sessions with
  their notes and durations. New `focus_session` table and
  `save_focus_session` / `list_focus_sessions` commands.

**Date-range export**
- The data export now takes optional From / To dates; leave them empty to
  export everything. `export_data` gained range parameters.

**In-app backup and restore**
- Settings -> Privacy can snapshot the local database to a file and restore it
  from a backup (with a confirm and a header sanity check). New
  `backup_database` / `restore_database` commands.

**Global pause / resume hotkey**
- A system-wide Ctrl + Alt + P shortcut toggles tracking from anywhere, via
  `tauri-plugin-global-shortcut`. Shown in Settings.

**Custom accent palettes**
- Settings -> Appearance offers four accent palettes (Signal, Slate, Solar,
  Cocoa) applied live via CSS-variable overrides and persisted.

**Update check**
- Settings -> Updates and shortcuts has a "Check for updates" button that
  reads the public GitHub releases API and reports whether a newer version
  exists. Nothing is sent; silent auto-install is a tracked follow-up.

**Internationalization scaffolding**
- A lightweight `t(key, fallback)` helper, an English catalog, and a language
  picker in Settings. The Sidebar is migrated as a proof; full string
  migration is a tracked follow-up.

**Accessibility pass**
- Aria labels on icon-only controls and form inputs, a named main nav
  landmark, and dialog semantics on the break overlay.

### Fixed
- **E2E regression**: the 0.2.0 onboarding flow showed the first-run welcome
  screen in the WebdriverIO test harness (which starts from a fresh database),
  hiding the dashboard the smoke tests assert on and turning the CI E2E job
  red. Test mode now marks onboarding complete so the app boots straight to
  the dashboard. The three real-OS build jobs were always green; this restores
  the E2E job too.

## [0.2.0] - 2026-06-13 - Onboarding, history, wellbeing engines, and always-on tracing

The second public release of System Trace. v0.1.0 shipped the tracking
core and the focus / wellbeing scaffolding; this release fills in the
missing engines, adds the history explorer, and fixes the biggest
real-world bug - that closing the window used to kill tracing.

### Added

**History explorer in Reports**
- Day / Week / Month mode toggle with a prev / next stepper, a
  reset-to-present chip, and left / right arrow key navigation.
- Day mode renders the same drill-down as the Dashboard (hourly chart,
  top apps, categories, longest session, app switches) for any past
  day.
- Week and Month modes show the daily-usage bars and aggregates for
  the picked calendar period. Month mode adapts the bar labels to keep
  all 28-31 dates visible.

**Onboarding flow**
- Four-step welcome (Welcome -> Privacy -> Made for grown-ups ->
  Always on, quietly) that runs once on first launch and is gated on
  the existing `settings.onboarding_complete` flag.
- The last step asks the user to enable autostart-at-login with a
  default-on checkbox.

**Pomodoro countdown for focus sessions**
- The Focus page now live-ticks the remaining session time as a
  monospaced MM:SS readout, refreshed every second.

**Productivity scoring engine**
- New `get_focus_score` command computes a 0-100 Focus Score from
  productive / distracting / neutral time, using the `category.productive`
  flag.
- Dashboard shows the score in a StatCard when
  `settings.scoring_enabled` is on.

**Category goals**
- New `category_goal` table and CRUD commands. Goals can be `under`
  (stay below) or `over` (reach at least), with per-day progress bars
  in a new Wellbeing section.

**Distraction nudges**
- Collector tracks continuous-time-on-distracting-app and fires a
  `distraction_nudge` event with a Tauri notification once per app per
  occurrence.
- New `DistractionToast` overlays calm dismissible toasts (Pause /
  Dismiss) in the bottom-right corner.
- Opt-in via `distraction_nudges_enabled` and
  `distraction_threshold_mins` settings.

**Schedules on block-list rules**
- Additive `block_rule` migration adds `schedule_enabled`,
  `schedule_start`, `schedule_end`.
- The hosts-file blocker and the in-process app block now both filter
  by the schedule. Same-day and overnight windows are both handled.
- Focus page exposes per-rule schedule UI inline with each rule.

**Bedtime grayscale (best effort)**
- New `grayscale.rs` with per-OS implementations: Windows uses HKCU
  `ColorFiltering` registry keys; macOS uses
  `defaults write com.apple.universalaccess`; Linux falls back to a
  GNOME theme swap via `gsettings`.
- Collector applies it on transition into / out of quiet hours when
  `bedtime_grayscale_enabled` is set.

**Always-on background tracing**
- The window's X button now hides the window instead of exiting the
  process, so the collector keeps tracking.
- Startup honors the autostart plugin's `--minimized` flag (and the
  `start_minimized` setting) so the boot-time launch is invisible.
- Onboarding wires the user into the loop with a recommended
  default-on autostart checkbox.

### Fixed
- The `bg-color/<alpha>` Tailwind opacity modifier broke against the
  Signal palette's hex CSS variables, which made the Reports daily-bar
  chart render with zero-height bars. Palette tokens now use RGB
  triplet companions; the modifier resolves correctly across the app.
- The macOS unexpected_cfgs lint inside the `objc` 0.2 crate's
  `msg_send!` and `class!` macros was tripping `cargo clippy -D warnings`
  in CI. Allowed at the crate root.
- The Linux `LinuxWatcher` lacked a `Default` impl, which clippy
  enforces.
- `tauri-plugin-wdio` (the E2E test bridge introduced in PR #8) is now
  gated behind the `SYSTEM_TRACE_TEST_MODE` env var, so production
  installs do not load test-harness code.

### Infrastructure
- E2E test suite landed in PR #8 via WebdriverIO + tauri-driver, with a
  dedicated `test-e2e` CI job running on Ubuntu under `xvfb-run`.

## [0.1.0] - 2026-06-11 - Initial open-source release

The first public release of System Trace - a free, local-first, cross-platform
desktop screen-time tracker built with Tauri 2, Rust, React, and SQLite.

### Added

**Phase 1 - Tracking core**

- Per-OS `Watcher` trait with real Windows implementation (Win32) and
  CI-verified macOS (NSWorkspace + CGEventSource) and Linux X11 (EWMH +
  screensaver) implementations.
- Background collector thread with a session-builder state machine
  (ACTIVE / IDLE / LOCKED) and smart, media-aware idle detection.
- SQLite storage via bundled `rusqlite`, with migrations, batched writes,
  daily rollups, and 90-day raw retention.
- Shared IPC contract between the Rust core (`models.rs`) and the TypeScript
  UI (`types.ts`).
- Dashboard UI with daily total, top apps, categories, and uPlot charts.

**Phase 2 - Limits and focus**

- Per-app daily limits with strict and soft strictness levels, and live
  enforcement from the collector.
- Focus mode that blocks distracting apps and (optionally) websites via
  bounded edits to the system `hosts` file.
- Tauri events for `usage_tick`, `limit_reached`, `focus_blocked`, and
  `focus_ended`.

**Phase 3 - Wellbeing**

- Break reminders with configurable interval, duration, and strictness.
- Bedtime quiet hours.
- `break_due` event and "break ended" handling.

**Phase 4 - Reports, polish, and platform integration**

- Daily and weekly summary delivery with catch-up markers so missed
  summaries are not silently skipped.
- Per-day and per-range totals; weekly screen-time report view.
- **History explorer** in the Reports view: walk back through any past
  **Day**, **Week**, or **Month** with a prev / next date stepper, a
  reset-to-present chip, and left / right arrow key navigation. Day mode
  shows the same drill-down as the Dashboard (hourly chart, top apps,
  categories, longest session, app switches) for any past day.
- System tray, single-instance enforcement, and the dialog, autostart, and
  notification Tauri plugins.
- "Launch at login" setting applied live through the autolaunch plugin.
- Dark and light themes using the "Signal" palette, Inter font, and the
  lucide-react icon set. No emoji anywhere in the product.

### Infrastructure

- GitHub Actions CI matrix for Ubuntu, Windows, and macOS, running
  `pnpm lint`, `pnpm build`, `cargo fmt --check`, `cargo clippy -D warnings`,
  and `cargo test`.
- `tauri-action` release workflow for producing installers for all three
  platforms.
- 11 unit tests covering the database layer, plus collector tests that use
  an injectable fake `Watcher`.

### Known limitations

- Linux Wayland window-title capture is deferred (X11 only for now).
- macOS window-title capture beyond the frontmost app is limited.
- The website blocker requires admin / root privileges to edit `hosts`.
- "Hard kill" on limit-reached is not enforced - the app currently nudges
  rather than terminating processes.

[Unreleased]: https://github.com/anandsundaramoorthysa/System-Trace/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/anandsundaramoorthysa/System-Trace/releases/tag/v0.1.0
