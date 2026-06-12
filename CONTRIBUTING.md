# Contributing to System Trace

Thanks for your interest in improving System Trace. Contributions are welcome -
bug fixes, the macOS and Linux watchers, new features, tests, and docs.

System Trace is a free, open-source, privacy-first, cross-platform screen-time
tracker built with Tauri 2 (Rust core + React/TypeScript UI) and SQLite.

## Principles (please respect these)

- **Local-first and private.** No telemetry, no accounts, nothing leaves the
  user's machine. Never add a feature that sends activity data off-device without
  an explicit, separate opt-in.
- **Cross-platform.** Every feature must work on Windows, macOS, and Linux. The
  only platform-specific code lives behind the `Watcher` trait in
  `app/src-tauri/src/platform/`.
- **No emoji** anywhere - code, comments, UI copy, commits, or docs. The UI uses
  the lucide-react icon set.
- **Calm, clean design** in both dark and light themes, using the "Signal" palette.

## Before you start

For anything non-trivial, please open an
[issue](https://github.com/anandsundaramoorthysa/System-Trace/issues) to discuss
the approach before writing code. For small fixes, a direct pull request is fine.

Architecture and the feature plan are documented in:

- [`docs/SYSTEM_DESIGN.md`](docs/SYSTEM_DESIGN.md) - how the collector, storage,
  aggregation, IPC, and per-OS watchers fit together.
- [`docs/FEATURES.md`](docs/FEATURES.md) - the full feature plan and roadmap.

## Development setup

Requirements:

- **Rust** (stable) via [rustup](https://rustup.rs/)
- **Node 20+** and **pnpm** (`npm install -g pnpm`)
- **Tauri CLI** (`pnpm add -g @tauri-apps/cli`) plus your OS build dependencies:
  - Windows: Microsoft C++ Build Tools and the WebView2 runtime
  - macOS: Xcode Command Line Tools
  - Linux: `webkit2gtk`, `libgtk-3-dev`, `librsvg2-dev`, and related packages

```bash
cd app
pnpm install
pnpm tauri dev      # run the app
pnpm tauri build    # build the production app + installer
```

## The shared IPC contract

`app/src-tauri/src/models.rs` (Rust) and `app/src/lib/types.ts` (TypeScript) are
two halves of one contract: every command result and event payload has a matching
struct/interface in both files with identical field names. **When you change one
side, change the other.**

## Code style and checks

All of these must pass before a pull request is merged (CI enforces them):

```bash
# Frontend (from app/)
pnpm lint           # ESLint
pnpm build          # tsc typecheck + vite build

# Rust (from app/src-tauri)
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

- TypeScript/React: ESLint + Prettier.
- Rust: rustfmt for formatting, clippy for lints.
- Tests: `cargo test` for the core (state machine, aggregation, retention), and
  WebdriverIO for E2E UI flows. Add tests when you add behavior; the collector
  takes a `Watcher` trait object so you can inject a fake watcher in tests.

### E2E Testing Setup

To run E2E tests locally, you need `tauri-driver` and the platform's WebDriver:

1. **Install tauri-driver**: `cargo install tauri-driver`
2. **Install WebDriver**:
   - **Linux**: `sudo apt install webkit2gtk-driver`
   - **Windows**: [Edge WebDriver](https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/)
   - **macOS**: Comes with Safari (`safaridriver --enable`)
3. **Run tests**:
   ```bash
   cd app
   pnpm tauri build --debug
   pnpm test:e2e
   ```

## Pull request process

1. **Fork** the repository.
2. Create a branch: `git checkout -b feature/your-change`.
3. Make your changes and run all the checks above.
4. **Commit** with a clear, plain-English message (no emoji).
5. **Open a pull request** describing what changed and why, and link any related
   issue. The CI matrix builds on Windows, macOS, and Linux - this is where the
   macOS and Linux watchers get verified.

## Reporting bugs and requesting features

Use the issue templates:
[bug report](https://github.com/anandsundaramoorthysa/System-Trace/issues/new/choose)
or feature request. For security issues, see [SECURITY.md](SECURITY.md) instead of
opening a public issue.

By contributing, you agree that your contributions are licensed under the project's
[MIT License](LICENSE).
