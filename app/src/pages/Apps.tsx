import { useMemo, useState } from "react";
import { Search, AppWindow } from "lucide-react";
import { getApps, getCategories, setAppCategory } from "../lib/api";
import { useAsync } from "../lib/useAsync";
import { Card, EmptyState, Spinner } from "../components/ui";

export function Apps() {
  const appsState = useAsync(getApps, []);
  const catsState = useAsync(getCategories, []);
  const [query, setQuery] = useState("");

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

  if (appsState.loading && !appsState.data) return <Spinner label="Loading apps" />;

  return (
    <div className="space-y-4">
      <div className="relative max-w-sm">
        <Search
          className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-muted"
          aria-hidden
        />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search apps"
          className="w-full rounded-md border border-border bg-surface py-2 pl-9 pr-3 text-body text-text placeholder:text-text-muted"
        />
      </div>

      <Card className="overflow-hidden">
        {filtered.length === 0 ? (
          <EmptyState
            icon={<AppWindow className="h-7 w-7" />}
            title="No apps to show"
            description="Apps you use are listed here once tracking has data."
          />
        ) : (
          <ul className="divide-y divide-border">
            {filtered.map((a) => (
              <li key={a.id} className="flex items-center justify-between gap-4 px-4 py-3">
                <div className="min-w-0">
                  <div className="truncate text-body-strong text-text">{a.display_name}</div>
                  <div className="truncate text-label text-text-muted">{a.app_key}</div>
                </div>
                <select
                  value={a.category_id ?? ""}
                  onChange={(e) => onChangeCategory(a.id, e.target.value)}
                  className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
                >
                  <option value="">Uncategorized</option>
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
  );
}
