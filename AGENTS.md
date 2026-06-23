# System Trace - Agent Workflows and Contributor Guidance

Role: this document guides Claude Code agents and human contributors through System Trace's architecture, code gates, conventions, and the principles that guard privacy and multi-platform integrity. Read together with `SYSTEM_DESIGN.md` and `FEATURES.md`.

## 1. Project Overview

System Trace is a Tauri 2 desktop application (Windows, macOS, Linux) that tracks active window time with privacy-first principles: all data local, no telemetry, no phone-home.

**Stack:**
- **Rust backend** (app/src-tauri/): collector state machine, SQLite storage, OS integration
- **React + TypeScript frontend** (app/src/): dashboard, reports, settings
- **E2E tests** (app/e2e/): WDIO + Tauri driver

**Layout:**
```
app/
  src/              React + TS (pages, components, theme, lib/)
  src-tauri/        Rust (collector, db, commands, platform/)
  e2e/              WDIO tests
  tauri.conf.json   App config (devUrl, bundle, security)
extension/          Browser extension (Manifest V3, standalone)
```

## 2. Code Gates (Required Before Commit or PR)

Every change must pass these gates **in order**. This is not optional.

```bash
cd app

# 1. Frontend checks
pnpm lint            # ESLint — catches style + logic errors
pnpm build           # tsc --noEmit + vite build — typecheck + bundle

# 2. Rust checks (from app/src-tauri)
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test           # lib crate only (pure core, no OS integration)
```

**Why this order:** lint catches cheap style issues first; build ensures types are sound; rustfmt ensures consistency; clippy finds performance/idiom misses; tests verify core logic.

**When to run:** after every meaningful change, before staging, before pushing.

## 3. Local-First and Privacy Principles

System Trace's core invariant: **activity data never leaves the machine. Ever.**

- **No network calls** for tracking or telemetry. (HTTP plugin use is confined to Phase 2+ features like checking updates.)
- **No cloud account or login** for tracking. (User data is not identifiable or valuable in the cloud.)
- **Database is in-memory + encrypted snapshots**: the live database runs in memory; only encrypted snapshots (XChaCha20-Poly1305) are written to disk (periodic, on data wipe, on exit). The decryption key lives in the OS credential store (Windows Credential Manager, macOS Keychain, Linux Secret Service). Test mode uses a plaintext file DB for speed.
- **Exclusions and titles are optional**: users can exclude apps or disable window title capture at any time.

When designing new features (Phase 2+: limits, blocker, wellbeing):
1. Data flows only through SQLite and Tauri commands/events on the same machine.
2. Do not add network calls unless it is opt-in and explicit (e.g., a future crash reporting feature).
3. If you touch the collector or storage layer, confirm the privacy boundary holds.

## 4. No Emoji, Ever

**Rule:** No emoji in code, comments, UI, commit messages, or documentation.

**Why:** emoji renders inconsistently across platforms and is inaccessible to screen readers.

**Use instead:** 
- UI: lucide-react icons (e.g., `<Clock />`, `<Settings />`)
- Text: plain language descriptors (e.g., "Clock icon" instead of "🕐")

Failing to follow this rule will block review.

## 5. Rust and IPC Contract Synchronization

The Rust and TypeScript layers communicate via Tauri commands and events. The contract is **split across two files** and must stay in sync **always**.

**Rust side:** `app/src-tauri/src/models.rs`  
**TypeScript side:** `app/src/lib/types.ts`

**Rule:**
- Every command return type in Rust has a corresponding TypeScript interface.
- Every event payload in Rust has a corresponding TypeScript type.
- Field names must match exactly (snake_case in Rust, automatically camelCase in JS via serde).

**When you change one file, change the other immediately in the same commit.**

Example:
```rust
// models.rs
#[derive(Serialize)]
pub struct AppUsage {
    app_id: u64,
    total_ms: u64,
}
```

```typescript
// types.ts
interface AppUsage {
  appId: number;
  totalMs: number;
}
```

Mismatch symptoms: "command returns undefined", "field is always null", or runtime panics. **Check both files first** when debugging IPC.

## 6. Rust Architecture: Lib vs Bin, Watcher Trait, Platform Code

### Lib vs Bin

- **`src-tauri/src/lib.rs`** owns all application logic: collector, database, commands, state, services. It exports `pub fn run()`.
- **`src-tauri/src/main.rs`** is a thin passthrough:
  ```rust
  fn main() {
      app_lib::run();
  }
  ```
  Why split: Tauri replaces `main()` with a mobile entry point on mobile builds. All portable logic lives in `lib.rs`.

- **`cargo test`** exercises the `lib` crate only, with no OS integration. Platform code is hidden behind trait abstraction.

### Watcher Trait (OS Abstraction)

All OS-specific code lives behind a single trait:
```rust
pub trait Watcher: Send {
    fn active_window(&mut self) -> Option<ActiveWindow>;
    fn idle_seconds(&mut self) -> u64;
    fn is_media_playing(&mut self) -> bool;
    fn session_locked(&mut self) -> bool;
}
```

**Rule:** The core collector never calls OS APIs directly. All OS calls go through the trait.

**Platform implementations** live in `src-tauri/src/platform/`:
- `windows.rs` — Win32 APIs (GetForegroundWindow, GetLastInputInfo, WASAPI)
- `macos.rs` — CoreGraphics, Accessibility, NSWorkspace
- `linux_x11.rs` — X11 (EWMH, XScreenSaver)
- `linux_wayland.rs` — D-Bus (GNOME Shell, KDE Plasma), Mutter IdleMonitor
- `linux.rs` — runtime routing (detects X11 vs Wayland, selects implementation)

In tests, inject a fake `Watcher` to control active windows and idle time deterministically without touching real OS APIs.

### When Porting to a New OS or Changing Platform Code

1. Implement the `Watcher` trait for the new platform.
2. Add runtime routing in `linux.rs` or a similar router if needed.
3. Write unit tests with a mock watcher.
4. Add platform-specific E2E tests in CI.
5. **Do not** call OS APIs from the core; the trait is the boundary.

## 7. Frontend Architecture and Design Principles

- **React + TypeScript** inside a Tauri webview.
- **Vite** for bundling and dev server (fixed port 1420, matches `tauri.conf.json` `build.devUrl`).
- **TypeScript strict mode** is enforced: `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`.
- **Routing:** simple client-side router (Dashboard, Apps, Reports, Focus, Settings, Onboarding).
- **Data layer:** `app/src/lib/api.ts` wraps Tauri `invoke()` and types all responses against `lib/types.ts`.
- **State:** UI state lives in React hooks; persistent data (theme, settings) comes from Rust via commands, and is cached locally only when safe.
- **Icons:** lucide-react ONLY. No emoji.
- **Theme:** Signal palette, CSS variables, dark + light modes. Theming is controlled by a `theme` setting (system, dark, light) persisted in Rust.

Constraints that protect this:
- **ESLint ignores `src-tauri/`**: `src/eslint.cjs` has `ignorePatterns: ['src-tauri']`.
- **Prettier formats TypeScript and CSS**, but not Rust.
- **Vite dev port 1420** is hardcoded in `tauri.conf.json`. Do not change it without updating the config.

## 8. Testing Strategy

### Rust Unit Tests (`cargo test`)

Fast, deterministic tests of pure logic:
- Collector state machine (session transitions, idle logic)
- Aggregation math (rollup totals, date boundaries)
- Retention trimming (old events deleted, summaries kept)
- Migrations (schema versioning, data integrity)

**Inject a fake `Watcher`** to control input without OS calls:
```rust
struct FakeWatcher { windows: Vec<...>, idle: u64 }
impl Watcher for FakeWatcher { ... }

#[test]
fn collector_closes_session_on_app_switch() {
    let mut watcher = FakeWatcher::new()
        .add_window("notepad", 0)
        .set_idle(0);
    // ... assert session behavior
}
```

### E2E Tests (`pnpm test:e2e`)

Playwright via tauri-driver; tests the real app window:
- Dashboard renders today's data and charts.
- Settings persist across app restart.
- Theme toggle works.
- Export and import flow.
- Each MVP flow end-to-end.

**Prerequisite:** `pnpm tauri build --debug` first. E2E tests require a debug build.

**Test mode:** The `SYSTEM_TRACE_TEST_MODE` env var auto-wipes the DB on launch, marks onboarding complete, and boots straight to dashboard. This keeps E2E tests fast and stateless.

### Coverage

- Unit tests verify **logic is correct**.
- E2E tests verify **the feature works in the UI**.
- Together, they form the safety net for refactors and new features.

## 9. Platform-Specific Code and CI

**All three OS are tested in CI** (ubuntu-22.04, windows-latest, macos-latest). This is where macOS and Linux watchers are compiled and verified.

- **Windows**: runs native (Win32 APIs).
- **macOS**: requires Xcode and can only be run on macOS CI or a Mac device.
- **Linux**: runs on ubuntu-22.04 with both X11 (via Xvfb) and Wayland (via new-session-dbus-service).

When you touch `src-tauri/src/platform/`, **CI must pass on all three OS** before merge. If you cannot test locally on a platform, the CI result is your verification.

## 10. Development Workflow

### Setting Up Locally

```bash
# Install dependencies
cd app
pnpm install

# Optional: install tauri-driver for E2E
pnpm add -D @tauri-apps/cli @wdio/tauri-service

# Start dev server
pnpm tauri dev

# In another terminal, run tests
cargo test --manifest-path app/src-tauri/Cargo.toml

# Or full E2E
pnpm tauri build --debug
pnpm test:e2e
```

### Commit and PR Workflow

1. **Create a branch** for your feature.
2. **Make changes** to the code.
3. **Run code gates** (lint, build, fmt, clipy, test) — see Section 2.
4. **Verify in the app** — run `pnpm tauri dev`, click through the feature, watch the logs.
5. **Write tests** if the change touches core logic or IPC.
6. **Commit with a conventional message** (feat, fix, docs, refactor, test, chore).
7. **Push and open a PR.**
8. **CI must pass on all three OS.** If a platform-specific issue appears, investigate and fix it (or document the limitation).

### Verifying a Change Works

Use the `/verify` skill to run the app and confirm the change works:
```
/verify "Run the app and confirm the feature works"
```

This will start `pnpm tauri dev`, launch the app, and let you test manually. Always do this for UI changes before claiming the feature is done.

## 11. Available Skills and When to Use Them

| Skill | When to use |
|-------|------------|
| **conventional-commit** | Generating a standardized commit message (feat, fix, docs, refactor, test, chore). |
| **rust-best-practices** | Writing Rust code — borrowing vs cloning, error handling, generics, performance. |
| **tauri-v2** | Questions about Tauri commands, IPC, permissions, capabilities, configuration, or plugin setup. |

## 12. Common Pitfalls and How to Avoid Them

| Pitfall | Why it happens | How to avoid |
|---------|---------------|-------------|
| IPC contract mismatch | models.rs and types.ts out of sync | Change both files in the same commit. Run tests to catch field mismatches. |
| Collector doesn't collect | Command not in `generate_handler![]` | Always add new commands to the macro. Test with `invoke()` on the frontend. |
| Rust test passes, but app panics | Logic works in isolation but breaks with real OS data | Inject realistic fake `Watcher` data in tests. E2E test the feature in the real app. |
| E2E test fails on CI but passes locally | Platform-specific behavior (X11 vs Wayland, Windows GDI timing) | Check CI logs. May need to run E2E on that OS or loosen timing assertions. |
| Emoji appears in UI or docs | Careless copy-paste or unclear rule | Grep for emoji. Use lucide-react icons. Plain text for descriptions. |
| Database bloats or schema breaks | Migrations skipped or incorrectly applied | Run `cargo test` in migrations module. Verify retention logic. Test migrations on a copy of a real DB. |
| Window titles or idle time unavailable on Linux | Wayland or missing dependencies | This is expected on some configs. Fallback gracefully (track app, skip title). Update docs. |

## 13. Asking for Help

When you get stuck, provide:
1. **What you changed** (files, lines).
2. **What went wrong** (error message, expected vs actual behavior).
3. **What you tried** (gates you ran, tests you wrote, E2E you attempted).
4. **Context** (which OS, which phase of development).

The core team will help unblock you within these principles:
- **Privacy and local-first** are non-negotiable.
- **All three OS must work equally.** Linux is not a second-class citizen.
- **Code gates are not suggestions.** They catch bugs and enforce consistency.
- **Simplicity over cleverness.** If it's not clear, it's not done.
