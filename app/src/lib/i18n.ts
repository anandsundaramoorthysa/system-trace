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
