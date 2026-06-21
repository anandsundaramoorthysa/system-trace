import { useMemo, useState } from "react";
import { Search, AppWindow, History } from "lucide-react";
import { t } from "../lib/i18n";
import { getApps, getCategories, searchUsage, setAppCategory } from "../lib/api";
import { useAsync } from "../lib/useAsync";
import { Card, CardTitle, EmptyState, Spinner } from "../components/ui";
import { AppAvatar } from "../components/AppAvatar";
import { formatDayLabel, formatDuration } from "../lib/format";
import type { SearchHit } from "../lib/types";

export function Apps() {
  const appsState = useAsync(getApps, []);
  const catsState = useAsync(getCategories, []);
  const [query, setQuery] = useState("");

  // History search (events, not just app names). Runs on demand.
  const [histQuery, setHistQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[] | null>(null);
  const [searching, setSearching] = useState(false);

  const cats = catsState.data ?? [];

  const filtered = useMemo(
    () =>
      (appsState.data ?? []).filter((a) =>
        a.display_name.toLowerCase().includes(query.trim().toLowerCase()),
      ),
    [appsState.data, query],
  );

  async function onChangeCategory(appId: number, value: string) {
    const categoryId = value === "" ? null : Number(value);
    await setAppCategory(appId, categoryId);
    appsState.reload();
  }

  async function runHistorySearch(e: React.FormEvent) {
    e.preventDefault();
    const q = histQuery.trim();
    if (!q) {
      setHits(null);
      return;
    }
    setSearching(true);
    try {
      setHits(await searchUsage(q, null, null));
    } finally {
      setSearching(false);
    }
  }

  if (appsState.loading && !appsState.data) return <Spinner label="Loading apps" />;

  return (
    <div className="space-y-6">
      {/* History search across all tracked time */}
      <div className="space-y-2">
        <CardTitle>{t("apps.search_history", "Search history")}</CardTitle>
        <Card className="p-5">
          <form onSubmit={runHistorySearch} className="flex gap-2">
            <div className="relative flex-1">
              <History
                className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-muted"
                aria-hidden
              />
              <input
                value={histQuery}
                onChange={(e) => setHistQuery(e.target.value)}
                placeholder={t("apps.search_placeholder", "Find when you used an app, e.g. Slack")}
                aria-label={t("apps.search_aria", "Search your usage history")}
                className="w-full rounded-md border border-border bg-bg py-2 pl-9 pr-3 text-body text-text placeholder:text-text-muted"
              />
            </div>
            <button
              type="submit"
              className="rounded-md bg-accent px-4 py-2 text-body-strong text-white transition-colors hover:opacity-90"
            >
              {t("apps.search_button", "Search")}
            </button>
          </form>

          {searching ? (
            <div className="mt-4">
              <Spinner label={t("apps.searching", "Searching")} />
            </div>
          ) : hits === null ? null : hits.length === 0 ? (
            <p className="mt-4 text-body text-text-muted">{t("apps.no_results", "No matching usage found.")}</p>
          ) : (
            <ul className="mt-4 divide-y divide-border">
              {hits.map((h) => (
                <li
                  key={`${h.day}-${h.app_key}`}
                  className="flex items-center justify-between gap-3 py-2.5"
                >
                  <span className="flex min-w-0 items-center gap-2">
                    <AppAvatar name={h.display_name} appKey={h.app_key} size={22} />
                    <span className="min-w-0">
                      <span className="block truncate text-body-strong text-text">
                        {h.display_name}
                      </span>
                      <span className="block truncate text-label text-text-muted">
                        {formatDayLabel(h.day)}
                        {h.sample_title ? ` - ${h.sample_title}` : ""}
                      </span>
                    </span>
                  </span>
                  <span className="shrink-0 font-medium text-text">
                    {formatDuration(h.total_ms)}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>

      {/* App list + re-categorize */}
      <div className="space-y-2">
        <CardTitle>{t("apps.list_title", "Apps")}</CardTitle>
        <div className="relative max-w-sm">
          <Search
            className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-muted"
            aria-hidden
          />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("apps.filter_placeholder", "Filter apps")}
            aria-label={t("apps.filter_aria", "Filter the app list")}
            className="w-full rounded-md border border-border bg-surface py-2 pl-9 pr-3 text-body text-text placeholder:text-text-muted"
          />
        </div>

        <Card className="overflow-hidden">
          {filtered.length === 0 ? (
            <EmptyState
              icon={<AppWindow className="h-7 w-7" />}
              title={t("apps.no_apps_title", "No apps to show")}
              description={t("apps.no_apps_desc", "Apps you use are listed here once tracking has data.")}
            />
          ) : (
            <ul className="divide-y divide-border">
              {filtered.map((a) => (
                <li key={a.id} className="flex items-center justify-between gap-4 px-4 py-3">
                  <div className="flex min-w-0 items-center gap-3">
                    <AppAvatar name={a.display_name} appKey={a.app_key} size={28} />
                    <div className="min-w-0">
                      <div className="truncate text-body-strong text-text">{a.display_name}</div>
                      <div className="truncate text-label text-text-muted">{a.app_key}</div>
                    </div>
                  </div>
                  <select
                    value={a.category_id ?? ""}
                    onChange={(e) => onChangeCategory(a.id, e.target.value)}
                    aria-label={`${t("apps.category_aria", "Category for")} ${a.display_name}`}
                    className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
                  >
                    <option value="">{t("apps.category_uncategorized", "Uncategorized")}</option>
                    {cats.map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.name}
                      </option>
                    ))}
                  </select>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>
    </div>
  );
}
