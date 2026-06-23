# Tauri v2+ Development Skill

> Build cross-platform desktop and mobile apps with web frontends and Rust backends.

| | |
|---|---|
| **Status** | Active |
| **Version** | 1.0.1 |
| **Last Updated** | 2026-04-02 |
| **Confidence** | 4/5 |
| **Production Tested** | https://v2.tauri.app/ |

## What This Skill Does

Provides expert assistance for Tauri v2 application development, covering the full development lifecycle from project setup to cross-platform deployment. Specializes in Rust backend commands, IPC patterns, security configuration, and frontend-backend communication.

### Core Capabilities

- Implement Rust commands with `#[tauri::command]` and proper error handling
- Configure IPC patterns (invoke, events, channels) for frontend-backend communication
- Set up security capabilities and permissions for plugins and APIs
- Access exhaustive reference docs for plugins (fs, dialog, shell, store, etc.), updater/distribution signing, and advanced runtime (tray, sidecars, deep links)
- Build and deploy for desktop (macOS, Windows, Linux) and mobile (iOS, Android)
- Integrate Vite + TanStack Router frontends with Tauri backends
- Configure tauri.conf.json and Cargo.toml for cross-platform builds

## Auto-Trigger Keywords

### Primary Keywords
Exact terms that strongly trigger this skill:
- tauri
- tauri v2
- tauri.conf.json
- src-tauri
- #[tauri::command]
- tauri::invoke
- capabilities.json

### Secondary Keywords
Related terms that may trigger in combination:
- rust backend
- desktop app
- cross-platform app
- webview
- invoke command
- emit event
- app permissions
- bundle desktop

### Error-Based Keywords
Common error messages that should trigger this skill:
- "Command not found"
- "Permission denied" (in Tauri context)
- "Failed to invoke command"
- "Missing capability"
- "Cannot read property of undefined" (invoke result)
- "tauri build failed"
- "Missing Rust target"

## Known Issues Prevention

| Issue | Root Cause | Solution |
|-------|-----------|----------|
| Command not found | Missing from `generate_handler![]` | Register all commands in the macro |
| Permission denied | Missing capability configuration | Add required permissions to capabilities file |
| State access panic | Type mismatch in `State<T>` | Use exact type matching `.manage()` call |
| White screen | Frontend not building | Verify `beforeDevCommand` and `devUrl` |
| Mobile build fails | Missing Rust targets | Run `rustup target add <platform-targets>` |
| IPC timeout | Blocking in async command | Use non-blocking async or spawn threads |

## When to Use

### Use This Skill For
- Creating new Tauri v2 projects or commands
- Configuring permissions and capabilities
- Setting up IPC (invoke, events, channels)
- Debugging command invocation issues
- Cross-platform build configuration
- Plugin integration and configuration
- Mobile (iOS/Android) deployment setup

### Don't Use This Skill For
- Tauri v1 development (use migration guide then this skill)
- Pure frontend development without Tauri integration
- Native mobile development (Swift/Kotlin directly)
- Backend API development without Tauri

## Version Policy

> [!NOTE]
> This skill targets **Tauri v2+**. Feature availability may vary across minor versions. When exact version timing matters, check the [official Tauri changelog](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri/CHANGELOG.md) and release notes for `tauri`, `@tauri-apps/api`, `@tauri-apps/cli`, and relevant plugins.

## Quick Usage

```bash
# Create new Tauri project
npm create tauri-app@latest

# Add Tauri to existing project
npm install -D @tauri-apps/cli@latest
npx tauri init

# Development
npm run tauri dev

# Production build
npm run tauri build

# Add a plugin (e.g., fs, dialog, store)
cargo tauri add fs          # Adds tauri-plugin-fs to Cargo.toml + JS package
cargo tauri add dialog
cargo tauri add store

# Mobile development
cargo tauri android init    # One-time setup
cargo tauri android dev     # Run on Android
cargo tauri android build   # Release build

cargo tauri ios init        # One-time setup (macOS only)
cargo tauri ios dev         # Run on iOS simulator
cargo tauri ios build       # Release build
```

## Token Efficiency

| Approach | Estimated Tokens | Time |
|----------|-----------------|------|
| Manual Implementation | ~15,000 | 2+ hours |
| With This Skill | ~6,000 | 30 min |
| **Savings** | **60%** | **~1.5 hours** |

## Reference Documentation

For deep-dive guidance on specific topics, see the upstream Tauri v2 documentation:

| Topic | Link |
|-------|------|
| **Security & Permissions** | [Capabilities & Permissions](https://v2.tauri.app/security/capabilities/) |
| **IPC Patterns** | [Calling Rust](https://v2.tauri.app/develop/calling-rust/) |
| **Official Plugins** | [Plugin Reference](https://v2.tauri.app/plugin/) |
| **Updater & Distribution** | [Updater Plugin](https://v2.tauri.app/plugin/updater/) |
| **Advanced Runtime** | [System Tray](https://v2.tauri.app/develop/system-tray/) |

## File Structure

```
tauri-v2/
├── SKILL.md        # Quick-start patterns, core rules, critical guidance
└── README.md       # This file - discovery and quick reference
```

## Dependencies

| Package | Version | Verified |
|---------|---------|----------|
| `@tauri-apps/cli` | ^2 (v2+) | 2026-04-02* |
| `@tauri-apps/api` | ^2 (v2+) | 2026-04-02* |
| `tauri` (Rust) | ^2 (v2+) | 2026-04-02* |
| `tauri-build` (Rust) | ^2 (v2+) | 2026-04-02* |

*\*Last verified: 2026-04-02. Always check [official changelog](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri/CHANGELOG.md) for feature timing.*

## Official Documentation

- [Tauri v2+ Documentation](https://v2.tauri.app/)
- [Commands Reference](https://v2.tauri.app/develop/calling-rust/)
- [IPC Concepts](https://v2.tauri.app/concept/inter-process-communication/)
- [Capabilities & Permissions](https://v2.tauri.app/security/capabilities/)
- [Configuration Reference](https://v2.tauri.app/reference/config/)
- [Plugin Directory](https://v2.tauri.app/plugin/)

## Related Skills

- `tanstack-start-expert` - TanStack Router patterns for type-safe frontend routing
- `react-component-architect` - React component patterns for Tauri frontends
- `go-google-style-expert` - Alternative backend patterns (if using Go instead)

## Companion Agent (Deprecated)

The `tauri-v2-expert` agent at `.claude/agents/specialized/tauri/tauri-v2-expert.md` is **deprecated/legacy**. This skill is the preferred and actively maintained interface. Use this skill over the agent for all new Tauri v2 work.

---

**License:** MIT
