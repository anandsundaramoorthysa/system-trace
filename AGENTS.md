# AGENTS.md - System Trace

## What this is

Tauri 2 desktop app: Rust core (tracking, SQLite, encryption) + React/TypeScript UI.
Also contains a standalone browser extension in `extension/` (Manifest V3, load-unpacked).

## Project layout

```
app/
  src/              React + TS UI (pages, components, theme, lib/)
  src-tauri/        Rust core (collector, db, commands, platform watchers)
  e2e/              WDIO E2E tests
extension/          Browser extension (plain JS, no build step)
```

## Commands

All frontend commands run from `app/`. All Rust commands run from `app/src-tauri/`.

```bash
# Frontend
cd app
pnpm install
pnpm lint            # ESLint
pnpm build           # tsc --noEmit + vite build (typecheck + build)
pnpm format          # Prettier write
pnpm format:check    # Prettier check

# Rust
cd app/src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test           # runs the lib crate (pure core, no OS watchers)

# Full dev
cd app
pnpm tauri dev       # starts Vite + Rust in dev mode

# E2E (requires tauri-driver + platform WebDriver)
cd app
pnpm tauri build --debug
pnpm test:e2e
```

**Required check order**: `pnpm lint` -> `pnpm build` -> `cargo fmt` -> `cargo clippy` -> `cargo test`

## Critical: IPC contract sync

`app/src-tauri/src/models.rs` (Rust) and `app/src/lib/types.ts` (TypeScript) are
two halves of one contract. Every command result and event payload must have matching
structs/interfaces with identical field names (snake_case). **When you change one,
change the other.**

## Rust lib vs bin

The lib crate (`system_trace_lib`) is what `cargo test` exercises. It contains the
pure core logic (collector state machine, aggregation, migrations) without OS watchers.
`main.rs` is a thin entry point that calls `run()` from the lib. Tests inject a fake
`Watcher` trait object to avoid touching real OS APIs.

## Platform-specific code

All OS-specific code lives behind `#[cfg(target_os = ...)]` in `app/src-tauri/src/platform/`:
- `windows.rs` - Win32 APIs
- `macos.rs` - CoreGraphics / Accessibility
- `linux_x11.rs` - X11 via x11rb
- `linux_wayland.rs` - D-Bus (GNOME/KDE)
- `linux.rs` - runtime routing

The core never calls OS APIs directly. The `Watcher` trait is the abstraction.

## Key gotchas

- **pnpm only** - not npm, not yarn. Lockfile is committed.
- **No emoji anywhere** - code, comments, UI, commits. Use lucide-react icons.
- **Database is in-memory + encrypted snapshots** - no plaintext DB on disk at rest.
  Key stored in OS keyring. Test mode uses a plaintext file DB.
- **Collector runs in background** - closing the window hides it; tray Quit exits.
- **`SYSTEM_TRACE_TEST_MODE`** env var gates E2E mode (WDIO plugin, fresh DB, skips onboarding).
- **Cargo.lock is committed** - this is an application, not a library.
- **TypeScript strict mode** is on (`noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`).
- **Vite dev port 1420** (fixed, matches `tauri.conf.json` `build.devUrl`).
- **ESLint ignores `src-tauri/`** - the `.eslintrc.cjs` `ignorePatterns` excludes it.
- **CI runs on all 3 OS** (ubuntu-22.04, windows-latest, macos-latest) - this is where macOS/Linux watchers get compiled and tested.

## Testing

- `cargo test` for core logic (state machine, aggregation, migrations)
- `pnpm test:e2e` for UI flows via WDIO + tauri-driver (requires debug build first)
- E2E test mode auto-wipes DB, marks onboarding complete, and boots to dashboard
- The collector takes a `Watcher` trait object - inject fakes in unit tests

## Available Skills

| Skill | Purpose |
|-------|---------|
| **conventional-commit** | Generate standardized commit messages following Conventional Commits spec (feat, fix, docs, etc.) |
| **rust-best-practices** | Guide for writing idiomatic Rust code — borrowing patterns, error handling, performance, testing, generics |
| **tauri-v2** | Build cross-platform desktop apps with Rust backend and React/TypeScript frontend — commands, IPC, permissions, configuration |
