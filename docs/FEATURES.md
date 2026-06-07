# System Trace - Full Feature Plan (Gap-Filling, Decided)

This is the complete, decided product plan. Each feature exists to close a
specific gap found in market research. The goal is one polished, cross-platform,
private app that does what today's tools only do in fragments.

## The Gaps We Are Solving

- G1: No tool is cross-platform on Windows, macOS, and Linux equally (Linux is
  the common blind spot).
- G2: No all-in-one. Users stitch together a tracker, a blocker, and a break app.
- G3: Private tools look rough; polished tools are cloud-based. No private AND
  polished option.
- G4: No Android-style per-app daily limits on desktop.
- G5: Built-in OS blockers only cover their own browser; no cross-browser/app
  blocking.
- G6: Weak wellbeing angle (bedtime/wind-down, grayscale, eye/posture breaks,
  doomscroll nudges).
- G7: Outdated UX in the market leaders.
- G8: No open-source option that is also well designed.

## Feature Set Mapped to Gaps

| Feature | What it does | Gaps it fills |
|---------|--------------|---------------|
| Cross-platform core | One app, identical on Windows, macOS, Linux | G1, G2 |
| Automatic time tracking | Records active app (and optional window title) and duration | G2, G3 |
| Smart idle detection | Excludes time away; still counts media/meeting time | G3 |
| Local-first storage | All data in local SQLite, never uploaded | G3, G8 |
| Polished dashboard | Clean charts: daily/weekly time, top apps, trends | G3, G7 |
| Neutral categories | Work, Social, Entertainment, Dev, etc.; optional scoring | G7 |
| Dark and light themes | Signal palette, both modes | G7 |
| Per-app daily limits | Set a daily cap; choose how strict the stop is | G4 |
| App and website blocking | Cross-browser, cross-app focus mode / blocklists | G5, G2 |
| Goals | Optional targets (e.g. keep social under 1h) | G4 |
| Break reminders | Eye-strain and posture breaks (overlay, skippable/strict) | G6, G2 |
| Wind-down / bedtime mode | Quiet hours, reduced nudges, best-effort grayscale | G6 |
| Distraction nudges | Gentle "you have been scrolling X" prompts | G6 |
| Focus session timer | Manual focus/Pomodoro session with optional blocking | G2, G6 |
| Data export and import | CSV/JSON export, one-click wipe, manual import to merge | G3, G8 |
| Exclusions | Skip chosen apps or private/incognito windows | G3 |
| Open source (MIT) | Fully open, inspectable, free | G8 |

## Decided Feature Specifications (locked)

These answers are final unless explicitly revisited.

1. Tracking unit: record the active app always; record window/tab titles too, but
   this is OPTIONAL and OFF by default, behind a clear toggle.
2. Smart idle: only mark idle after a longer no-input gap; detect audio playback
   and full-screen video so passive media and meetings still count as usage.
3. Browser depth: app-level in Phase 1 (e.g. "Chrome - 2h"). Optional browser
   extension for per-website detail comes later (Phase 4). No online lookups.
4. Judgment: neutral categories by default (no "distracting" labels). Productivity
   scoring (including a Focus Score) is OPTIONAL and user-enabled in settings.
5. Categorization source: a built-in LOCAL preset list plus easy manual
   re-tagging. Never send app names off the machine.
6. Goals: none in MVP; optional goals arrive in Phase 2 alongside limits.
7. Limit behavior: when a per-app daily limit is reached, the user chooses the
   strictness per limit. Default is a medium, dismissible full-screen nudge;
   strict (hard block, lockable) is opt-in.
8. Website blocking method: leaning to the system hosts-file method first (works
   across all browsers); browser-extension method later. Final call is made with
   the system design.
9. Break reminders: full-screen "look away" overlay, with skippable and strict
   modes, intervals user-configurable.
10. Bedtime/wind-down: starts as quiet hours plus reduced nudges; OS-level
    grayscale is best-effort per platform (applied where the OS allows it).
11. Hero metric: "Screen Time Today" is the dashboard hero number. An optional
    Focus Score shows only if scoring is enabled.
12. Notifications: a daily summary by default; real-time nudges are opt-in.
13. Multi-computer: each machine keeps its own local data. Manual export/import to
    merge in MVP. No automatic sync (protects the privacy promise).
14. Data retention: keep rolled-up summaries forever; trim raw events after a
    user-configurable period, default 90 days, to keep the database small.
15. Focus session timer: included, but in Phase 3 (pairs with blocking and breaks),
    not the MVP.

## Phased Roadmap

Phases are about sequencing, not separate products. All ship on Windows, macOS,
and Linux.

### Phase 0 - Setup and planning (current)
Brand, palette, logo, tech stack, license, plan. System design next. No app code.

### Phase 1 - Tracking core (MVP)
The trustworthy foundation. Fills G1, G3, G7, G8.
- Background collector with per-OS active-window and smart idle watchers
  (audio/video aware).
- Active app tracking; optional, off-by-default window-title tracking.
- Local SQLite storage, daily/weekly aggregation, configurable raw retention
  (default 90 days).
- Dashboard: "Screen Time Today" hero, top apps, daily/weekly trends.
- Neutral categories from a local preset list, with manual re-tagging.
- Dark and light themes.
- Privacy basics: export (CSV/JSON), import/merge, delete/reset, exclusions
  (apps and private/incognito windows).
- System tray, autostart toggle, onboarding with permission flow.
- Daily summary notification (opt-in real-time nudges come later).

### Phase 2 - Control (limits and blocking)
Turns insight into action. Fills G2, G4, G5.
- Per-app daily limits with per-limit strictness (default medium nudge, opt-in
  strict lockable block).
- Optional goals (e.g. keep a category under a target).
- Focus mode: block chosen apps and websites across all browsers (hosts-file
  method leaning; finalized in system design).
- Blocklists and schedules (e.g. work hours).

### Phase 3 - Wellbeing
The underserved angle. Fills G6.
- Break reminders for eyes and posture: full-screen overlay, skippable or strict.
- Wind-down / bedtime mode: quiet hours, reduced nudges, best-effort grayscale.
- Distraction nudges based on usage patterns (opt-in real-time).
- Manual focus session / Pomodoro timer, with optional blocking during sessions.

### Phase 4 - Polish and distribution
- Installers per OS, code signing / notarization (cost to decide), auto-update.
- Optional browser extension for per-website detail (instead of app-level only).
- Performance passes to keep the background footprint minimal.

## Out of Scope (for now)

- Cloud sync and accounts (conflicts with local-first; only ever as explicit
  opt-in, much later, if at all).
- Team/employer monitoring and screenshots (not our audience).
- Mobile apps.
- Parental-control account hierarchies (could be revisited far later).
- Online category lookups or any feature that sends activity data off-device.

## Guardrails

- Never let a new feature break the privacy or cross-platform promises.
- Keep the tracking core lean; new features must not bloat the always-on agent.
- Every feature ships with dark and light styling and no emoji in its copy.
