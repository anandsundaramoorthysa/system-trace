/**
 * Lightweight i18n scaffolding.
 *
 * This is intentionally small: a flat key->string catalog per language and a
 * `t(key, fallback)` lookup. The app ships English today; other languages are
 * community-contributed by adding a catalog below and a `LANGUAGES` entry.
 *
 * Components call `t("some.key", "English fallback")`. If the active language
 * has no entry for the key, the English fallback is shown - so the UI is never
 * blank even when a translation is incomplete. Full migration of every hard
 * coded string into keys is tracked as a follow-up issue.
 */

export interface Language {
  code: string;
  label: string;
}

/** Languages the user can pick in Settings. Add entries as catalogs land. */
export const LANGUAGES: Language[] = [{ code: "en", label: "English" }];

type Catalog = Record<string, string>;

// English is the source of truth; its catalog is the set of migrated keys.
const en: Catalog = {
  "nav.dashboard": "Dashboard",
  "nav.apps": "Apps",
  "nav.reports": "Reports",
  "nav.focus": "Focus",
  "nav.wellbeing": "Wellbeing",
  "nav.settings": "Settings",
  "settings.appearance": "Appearance",
  "settings.language": "Language",
  "settings.palette": "Accent palette",
  "common.pause": "Pause",
  "common.resume": "Resume",
  "state.active": "Active",
  "state.idle": "Idle",
  "state.locked": "Locked",
  "state.paused": "Paused",
  "dashboard.title": "Dashboard",
  "dashboard.screen_time_today": "Screen Time Today",
  "dashboard.vs_yesterday": "vs yesterday",
  "dashboard.most_used": "Most Used",
  "dashboard.no_usage": "No usage yet",
  "dashboard.longest_session": "Longest Session",
  "dashboard.focus_score": "Focus Score",
  "dashboard.categorize_apps": "Categorize apps to score",
  "dashboard.productive": "productive",
  "dashboard.distracting": "distracting",
  "dashboard.by_hour": "Today, by hour",
  "dashboard.switches": "switches",
  "dashboard.categories": "Categories",
  "dashboard.top_apps": "Top apps",
  "apps.search_history": "Search history",
  "apps.search_placeholder": "Find when you used an app, e.g. Slack",
  "apps.search_aria": "Search your usage history",
  "apps.search_button": "Search",
  "apps.loading": "Loading apps",
  "apps.searching": "Searching",
  "apps.no_results": "No matching usage found.",
  "apps.list_title": "Apps",
  "apps.filter_placeholder": "Filter apps",
  "apps.filter_aria": "Filter the app list",
  "apps.no_apps_title": "No apps to show",
  "apps.no_apps_desc": "Apps you use are listed here once tracking has data.",
  "apps.category_uncategorized": "Uncategorized",
  "apps.category_aria": "Category for",
  "focus.title": "Focus mode",
  "focus.active_desc": "Blocked apps will nudge you.",
  "focus.off_desc": "Off. {count} block rule(s) ready.",
  "focus.stop": "Stop",
  "focus.start": "Start focus",
  "focus.timer_aria": "Time remaining in focus session",
  "focus.note_label": "What are you working on? (saved when the session ends)",
  "focus.note_placeholder": "e.g. deep work on the auth refactor",
  "focus.recent_title": "Recent sessions",
  "focus.session_default": "Focus session",
  "focus.duration_min": "{mins}m",
  "focus.limits_title": "Daily app limits",
  "focus.limit_choose": "Choose an app",
  "focus.limit_min_day": "min/day",
  "focus.limit_strict_soft": "Soft (track only)",
  "focus.limit_strict_med": "Medium (nudge)",
  "focus.limit_strict_str": "Strict (strong nudge)",
  "focus.limit_add": "Add limit",
  "focus.no_limits_title": "No limits yet",
  "focus.no_limits_desc": "Set a daily cap on an app to get a gentle nudge when you reach it.",
  "focus.limit_remove_aria": "Remove limit for {name}",
  "focus.block_list_title": "Block list (focus mode)",
  "focus.rule_kind_app": "App",
  "focus.rule_kind_web": "Website",
  "focus.rule_pattern_placeholder": "e.g. {example}",
  "focus.rule_add": "Add rule",
  "focus.no_rules_title": "No block rules",
  "focus.no_rules_desc": "Add apps or websites to block while focus mode is on.",
  "focus.rule_remove_aria": "Remove rule {pattern}",
  "focus.rule_active_between": "Active only between",
  "focus.rule_and": "and",
  "focus.apply_now": "Apply now",
  "focus.clear_now": "Clear now",
  "focus.block_desc": "App rules nudge you when a blocked app is in front during focus mode. System-wide website blocking edits the hosts file (requires running System Trace as administrator) and now follows each rule's schedule automatically - blocks apply when a window opens and clear when it ends. The buttons above just force an immediate sync; a \"Clear now\" will re-apply within seconds if a rule is still enabled and in its active window.",
  "onboarding.step_indicator": "Step {current} of {total}",
  "onboarding.step1_title": "Welcome to System Trace",
  "onboarding.step1_body": "A calm screen-time tracker for your desktop. It records the app and window you are using, detects idle time, and turns it into clear dashboards and reports - so you can understand where your time goes without guesswork.",
  "onboarding.step2_title": "Private by default",
  "onboarding.step2_body": "Everything stays on this device. There is no cloud, no account, and no telemetry. Your data lives in a local SQLite database you can export, wipe, or exclude apps from at any time. Window-title capture is off by default - you can turn it on in Settings if you want.",
  "onboarding.step3_title": "Made for grown-ups",
  "onboarding.step3_body": "Limits, focus mode, and break reminders are here when you want them - quiet when you don't. Categories are neutral by default; turn on productivity scoring in Settings if you want a Focus Score. You can pause tracking at any moment from the top bar.",
  "onboarding.step4_title": "Always on, quietly",
  "onboarding.step4_body": "System Trace works best when it runs in the background. With your permission, it will start when you sign in to your computer and live in the system tray. Closing the window keeps it tracking; only quitting from the tray menu stops it. You can change this any time in Settings.",
  "onboarding.run_at_login": "Run System Trace when I sign in to my computer",
  "onboarding.recommended": "(recommended)",
  "onboarding.skip": "Skip",
  "onboarding.next": "Next",
  "onboarding.get_started": "Get started",
};

const CATALOGS: Record<string, Catalog> = { en };

let activeLang = "en";

/** Set the active language code. Unknown codes fall back to English. */
export function setLanguage(code: string) {
  activeLang = CATALOGS[code] ? code : "en";
}

export function getLanguage(): string {
  return activeLang;
}

/** Translate a key, returning `fallback` when the active language lacks it. */
export function t(key: string, fallback: string): string {
  const cat = CATALOGS[activeLang] ?? en;
  return cat[key] ?? en[key] ?? fallback;
}
