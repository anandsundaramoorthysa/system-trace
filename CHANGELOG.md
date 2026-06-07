# Changelog

All notable changes to System Trace are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing yet. New work in progress lives here until the next tagged release.

## [0.1.0] - Initial open-source release

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
