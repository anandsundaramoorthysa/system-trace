# Changelog

All notable changes to System Trace are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-08-12 - Cross-platform parity: Wayland, hard-kill, custom categories

A large release built with the community during Social Summer of Code 2026.
Every platform gets closer to parity, per-app limits can now truly enforce, and
users get custom categories and a configurable idle timeout.

### Added

- **Linux Wayland active-window and idle tracking.** GNOME/KDE via a D-Bus-first
  strategy with a runtime X11 / Wayland router; stock GNOME Wayland without the
  focused-window extension degrades safely.
- **Hard-kill per-app limit enforcement on all three OSes.** A new "Hard"
  strictness terminates the offending process (Windows via `TerminateProcess`;
  Linux and macOS via `SIGTERM` -> `SIGKILL`), behind a shared
  `ProcessTerminator` trait, with graceful notifications on failure.
- **Real OS app-icon extraction on Linux.** Resolves the app's `.desktop` entry
  and the active icon theme (with `hicolor` / `pixmaps` fallbacks).
- **macOS window-title capture** via the Accessibility API, with a permission
  prompt and graceful degradation when permission is denied.
- **Custom app categories.** Create, rename, recolor, and delete your own
  categories (app groups) directly from the Apps screen.
- **Configurable idle timeout.** Choose how many minutes of inactivity count as
  idle, in Settings (default 2 minutes).
- **Multi-monitor break overlay.** On Windows, the break/limit overlay now
  appears on the monitor holding the focused window.
- **Scroll-to-top button** for long pages.
- **Linux system-wide website blocking without running the whole app as root** —
  uses `pkexec` (Polkit) to edit `/etc/hosts`, staged via a private temp file.
- **Intel (`x86_64`) macOS installer** in the release matrix, alongside Apple
  Silicon, with architecture-suffixed asset names.
- **CodeQL security scanning** (Rust + TypeScript) on every pull request.
- **`AGENTS.md`** and agent-skill manifests for contributors.

### Fixed

- **Call / meeting audio no longer counts as idle.** Media detection now also
  checks the WASAPI communications endpoint, so headset calls keep the session
  active.
- **UWP apps** (Calculator, Settings, Store, ...) now resolve to the real app
  instead of collapsing into the generic `ApplicationFrameHost`.
- **App icons resolve for apps first seen this session.** A stale `"none"` path
  no longer sticks, so apps like Chrome show their real icon.
- **Bedtime grayscale on Linux** now saves and restores the user's real GTK
  theme instead of clobbering it.
- **Orphaned `.enc.corrupt` quarantine files** are pruned after a successful
  startup, and the Linux hosts file is staged to a private `0600` / `O_EXCL`
  temp file before the privileged copy.

### Changed

- **Full i18n string migration** across the core pages onto a `t()` translation
  catalog, keeping English fallbacks.
- The GitHub release workflow now builds an Intel `x86_64-apple-darwin` macOS
  installer alongside the Apple Silicon build.

### Verified (by the community, on real hardware)

- Windows: real app-icon extraction across app types; packaged-installer
  autostart and close-to-tray; sleep/suspend idle handling.
- Windows and Linux: data-at-rest encryption and key persistence, including the
  headless `db.key` fallback.
- Linux: the `pkexec` hosts-file website blocker; autostart and close-to-tray.

## [0.4.2] - 2026-06-14 - Critical: the app could not launch after a restart

### Fixed

- **CRITICAL: the app failed to launch after a reboot/restart.** The `keyring`
  crate (3.x) ships **no credential-store backend by default** - it silently
  falls back to an **in-memory mock store** that is lost when the process
  exits. So the database encryption key only ever lived in memory: the app
  worked while it kept running, but after a restart the key was gone, and the
  (correct) "don't create a new key when encrypted data exists" safeguard then
  made startup fail - the window never appeared.
  - **Fix:** enable the real, persistent OS credential stores
    (`windows-native` → Windows Credential Manager, `apple-native` → macOS
    Keychain, `sync-secret-service` → Linux Secret Service). Verified
    end-to-end: the key now survives separate process runs, and the app opens
    cleanly across restarts with data intact.
  - **Recovery:** if the key is genuinely missing (e.g. upgrading from the
    broken build), the app no longer refuses to start - it quarantines the
    now-undecryptable snapshot and starts fresh, while a *transient* key-store
    error still fails safe so a retry can recover. (Data created by the broken
    build is unrecoverable, since its key was never persisted.)

## [0.4.1] - 2026-06-14 - Critical tracking fix + hardening

A deep code review caught a critical regression and a batch of robustness bugs.

### Fixed

- **CRITICAL: activity tracking was disabled since v0.3.0.** The call that
  starts the background collector thread (`collector::spawn`) was accidentally
  removed in the v0.3.0 commit, so since then **nothing was tracked** - no
  events, no live state, and (in 0.4.0) none of the collector-loop features
  (media/lock detection, periodic encrypted snapshots). Restored. (Unit tests
  exercise the session-builder directly, so they didn't catch the missing
  spawn.)
- **Encryption key is never silently replaced.** If the OS key store errors
  transiently (locked / not ready) while an encrypted database exists, the app
  no longer mints a fresh key (which would make the data permanently
  undecryptable); it fails the launch safely and recovers on a later start.
- **A corrupt/undecryptable snapshot no longer bricks the app.** Instead of
  panicking on every launch, the unreadable file is quarantined and the app
  starts fresh.
- **Migration is safer.** The legacy plaintext database is only removed after
  the first encrypted snapshot succeeds, so a failed snapshot can't leave you
  with neither a readable plaintext file nor a good encrypted one.
- **Restore now persists immediately.** `restore_database` writes an encrypted
  snapshot right after restoring, so a crash can't lose the restore.
- **App icons no longer render invisible.** Icons whose source had no alpha
  channel were drawn fully transparent on Windows; they're now treated as
  opaque. (Plus a macOS bitmap-rep leak and a dead `ReleaseDC` cleaned up.)
- **Bedtime grayscale is reliably reverted on quit** even if the apply worker
  hadn't finished (intent is recorded immediately).
- **macOS window-title lookup only runs when title capture is on**, avoiding a
  per-second Accessibility round-trip (and permission spam) otherwise.
- Smaller fixes: distraction nudge stops counting an app the moment it's
  re-categorized as non-distracting; Windows desktop-name parsing reads only up
  to the NUL terminator; the strict-limit overlay unsubscribes cleanly; per-app
  goal input rejects a blank/zero value; per-app icon RGBA is clamped to the
  canvas buffer.

## [0.4.0] - 2026-06-14 - Gap-closure features

A broad feature release closing the largest remaining gaps: real activity
signals (media/lock detection), encryption at rest, real app icons, per-app
goals, strict-limit enforcement, PDF export, a notification-click fix, and a
companion browser extension.

### Added

**Data-at-rest encryption**
- The live database now runs **in memory**; only **encrypted** snapshots
  (XChaCha20-Poly1305) are written to disk - periodically and on exit - so no
  plaintext database exists at rest. The key lives in the OS credential store
  (Windows Credential Manager / macOS Keychain / Linux Secret Service) with a
  restricted key-file fallback. Pure-Rust crypto, so it builds everywhere with
  no extra toolchain. Existing plaintext databases are migrated on first launch
  and the old plaintext file is removed. "Delete all data" rewrites the
  encrypted snapshot immediately.

**Media-aware idle + lock detection (all OS)**
- Audio actually playing now keeps you "active" even with no input: Windows via
  the WASAPI peak meter, macOS via CoreAudio, Linux via ALSA `/proc/asound`.
- A locked/secure session now counts as inactive: Windows via the input desktop,
  macOS via the window-server session dictionary.

**Real app icons**
- Top Apps / Apps / search now show the app's **real OS icon** (Windows exe
  icon via GDI; macOS bundle icon), falling back to the letter avatar when none
  is available. App executable paths are remembered for extraction.

**Per-app goals**
- Daily "stay under / reach at least" targets on individual apps (alongside
  category goals), with progress bars and consecutive-day streaks.

**Strict-limit lockout**
- A **strict** daily limit now shows a blocking full-screen lockout overlay and
  brings the window forward, rather than only nudging. (No process is killed -
  consistent with the app's philosophy.) Medium/soft limits are unchanged.

**PDF report export**
- The Reports view can export a one-page PDF summary of the current day or
  week/month (totals, top apps, categories). Local only, no email. jsPDF is
  lazy-loaded so it isn't in the startup bundle.

**Browser extension (companion, separate artifact)**
- A Manifest-V3 extension under `extension/` tracks **per-site** time in the
  browser (domain only, stored locally) and exports JSON matching System
  Trace's per-app-per-day shape. Load-unpacked, no store or fee required.

**macOS window titles**
- Frontmost window title capture via the Accessibility API (when the user
  grants Accessibility permission).

### Fixed
- **Clicking a notification now opens the app.** A global notification-action
  handler focuses the window when any reminder (break / limit / distraction) is
  clicked, on every platform.

### Notes
- Linux per-site media detection and the new Linux/macOS native code paths are
  verified by the CI build matrix. Linux **Wayland** active-window capture
  remains compositor-specific and is not yet implemented (XWayland apps are
  still tracked via the X11 path); pure-Wayland windows degrade gracefully.
- Internationalization: the app chrome (navigation, top bar, sidebar, settings
  sections, live-state labels) is migrated to the `t()` catalog; full
  body-string coverage is ongoing mechanical work and renders English via
  fallback meanwhile.

## [0.3.1] - 2026-06-14 - Bug fixes

A patch release that fixes a data-loss risk, makes backup/restore safe and
instant, and resolves a batch of cross-platform correctness bugs found in a
deep code and security review.

### Fixed
- **Resume did not visibly work.** `set_tracking_paused(false)` returned the
  stale `Paused` collector state instead of a running one, so clicking
  "Resume" in the top bar snapped straight back to "Resume" and looked
  broken (the collector did resume in the background, but the UI never
  reflected it until the next tick). Resuming now reports `Idle` immediately
  so the button flips to "Pause" at once.
- **Restore could lose data and required a restart.** Restore overwrote the
  live database file under an open connection, which risked corruption and
  meant the app had to be relaunched to see the data. Backup and restore now
  use SQLite's online backup/restore API: backup takes a consistent snapshot
  while holding the connection lock (no race with the collector mid-copy), and
  restore copies the backup page-by-page into the live database - safe, and
  effective immediately with no restart.
- **Restored backups from older versions are migrated.** After a restore the
  schema is brought up to date, so a backup taken on an earlier release gains
  any newer columns, tables, and settings instead of leaving the app on a
  stale schema.
- **Global hotkey now persists and reports failure.** Toggling tracking with
  Ctrl + Alt + P now saves the paused state (so it survives a restart, like
  the Settings toggle does), and if another app already owns the shortcut,
  Settings shows the chord as unavailable instead of letting it look dead.
- **Bedtime grayscale no longer stalls tracking or lingers after quit.**
  Applying the OS color filter (which shells out to `reg.exe` / `defaults` /
  `gsettings`) now runs off the collector loop so a tick is never blocked on a
  child process. On Windows it broadcasts `WM_SETTINGCHANGE` so the running
  session picks up the change, and on quit any applied filter is undone so the
  Linux theme swap does not outlive the app.
- **Distraction nudges stop re-querying the database every tick.** Whether the
  foreground app is distracting is now resolved once per app change and cached,
  instead of hitting the database on every loop while the same app stays in
  front.
- **"Check for updates" now gives feedback and can't hang.** The button shows
  a spinning, disabled "Checking..." state while it works, so feedback no
  longer depends on a toast that could scroll off-screen or clear itself
  mid-request - the original reason it looked like nothing happened. The whole
  request (headers and body) is bounded by a 10-second timeout, and rate-limit
  and connection failures are reported in plain language.
- **Tracking can no longer be silently killed by a poisoned lock.** If any
  thread panicked while holding the shared collector state, the next tick used
  to panic the collector too and stop tracking for good; it now recovers and
  keeps running.

### Fixed (cross-platform correctness)

A deeper review surfaced a batch of pre-existing bugs that affected specific
operating systems or edge cases:

- **Website blocking now honors its schedule (all OS).** Scheduled hosts-file
  blocks were only ever written or removed by the manual Apply / Clear
  buttons, so an overnight or timed window never switched on at its start or
  off at its end. The collector now reconciles the system hosts file with the
  in-force website rules continuously - blocks apply when a window opens and
  clear when it ends. It only writes when the set changes, so it never spams
  the hosts file or permission prompts for users who don't use website
  blocking. The Apply / Clear buttons are now "force an immediate sync".
- **The hosts blocker can no longer eat your hosts file (all OS).** If a
  previous block was left unterminated (interrupted mid-write), the cleanup
  pass used to delete every line after it - potentially your own hosts
  entries. It now stops at the first line that isn't ours and preserves the
  rest. Covered by a new test.
- **Linux idle detection no longer counts you active forever.** Under Wayland
  or an X server without the SCREENSAVER extension there is no idle signal;
  the watcher used to report "0 ms since input" (always active), inflating
  usage around the clock. It now reports idle in that case so time is not
  counted while you're away.
- **Windows: unreadable-process windows are no longer merged into one bogus
  app.** A foreground window whose process couldn't be resolved (a protected
  or elevated system surrogate) used to be attributed to a single synthetic
  "Unknown" app, lumping unrelated windows together; such samples are now
  skipped instead.
- **Day drill-downs survive a daylight-saving spring-forward.** In time zones
  where the clocks jump at midnight, local midnight doesn't exist, and the
  per-day view used to fall back to 1970 and mix in unrelated events. It now
  resolves the first real instant of the day.
- **A zero / brand-new limit no longer fires immediately.** A daily limit of
  0 ms counted as "exceeded" at 0 usage and fired a "limit reached" nudge at
  once; a non-positive limit is now never treated as exceeded. Covered by a
  new test.
- **Bedtime grayscale can no longer get stuck on after rapid toggles.** The OS
  color-filter change is now applied on a single serialized worker so the most
  recent request always wins, instead of detached threads that could finish
  out of order and leave the display in the wrong state.
- **Settings status messages could be invisible.** They rendered at the top
  of the Settings page, so any message triggered while scrolled down -
  export, backup and restore, and the update check - appeared off-screen.
  They now show as a fixed bottom-center toast that is always visible and
  linger a little longer.

### Internal
- Added a regression test proving an in-place upgrade from an old (v0.1.0-era)
  database preserves all existing data while gaining the new columns, tables,
  and settings - so updating the app never loses your history.

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
