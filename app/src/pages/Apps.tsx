import { useMemo, useState } from "react";
import { Search, AppWindow, History, Settings2, Trash2, Edit2, X } from "lucide-react";
import { t } from "../lib/i18n";
import { getApps, getCategories, searchUsage, setAppCategory, upsertCategory, deleteCategory } from "../lib/api";
import { useAsync } from "../lib/useAsync";
import { Card, CardTitle, EmptyState, Spinner } from "../components/ui";
import { AppAvatar } from "../components/AppAvatar";
import { formatDayLabel, formatDuration } from "../lib/format";
import type { SearchHit } from "../lib/types";

const PALETTE = ["#2DD4BF", "#0EA5A0", "#34D399", "#F59E0B", "#F87171", "#8B949E", "#656D76"];

export function Apps() {
  // Use a state dependency nonce to re-trigger useAsync requests cleanly
  const [refreshNonce, setRefreshNonce] = useState(0);

  // --- Data Fetching Hooks passing dependency array ---
  const { data: apps, loading: appsLoading } = useAsync(getApps, [refreshNonce]);
  const { data: initialCats, loading: catsLoading } = useAsync(getCategories, [refreshNonce]);
  
  // Localized query states
  const [query, setQuery] = useState("");
  const [histQuery, setHistQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [hits, setHits] = useState<SearchHit[] | null>(null);

  // --- Category Management UI State (IDs are numbers) ---
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [catName, setCatName] = useState("");
  const [catColor, setCatColor] = useState(PALETTE[0]);
  const [catProductive, setCatProductive] = useState(true);
  const [formError, setFormError] = useState("");

  const cats = initialCats || [];

  // Trigger cache refetches seamlessly
  const triggerRefetch = () => setRefreshNonce((prev) => prev + 1);

  // --- Search Actions ---
  const runHistorySearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!histQuery.trim()) return;
    setSearching(true);
    try {
      // searchUsage expects (query, from, to)
      const res = await searchUsage(histQuery, null, null);
      setHits(res);
    } catch (err) {
      console.error(err);
    } finally {
      setSearching(false);
    }
  };

  const onChangeCategory = async (appId: number, categoryStrId: string) => {
    try {
      const parsedCatId = categoryStrId ? parseInt(categoryStrId, 10) : null;
      await setAppCategory(appId, parsedCatId);
      triggerRefetch();
    } catch (err) {
      console.error(err);
    }
  };

  // --- Category CRUD Submissions ---
  const resetForm = () => {
    setEditingId(null);
    setCatName("");
    setCatColor(PALETTE[0]);
    setCatProductive(true);
    setFormError("");
  };

  const handleSaveCategory = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmedName = catName.trim();
    
    if (!trimmedName) {
      setFormError(t("apps.cat_error_empty", "Category name cannot be empty."));
      return;
    }

    // Explicit type matching for ID checks
    const isDuplicate = cats.some(
      (c) => c.name.toLowerCase() === trimmedName.toLowerCase() && c.id !== editingId
    );
    if (isDuplicate) {
      setFormError(t("apps.cat_error_duplicate", "A category with this name already exists."));
      return;
    }

    try {
      await upsertCategory({
        id: editingId,
        name: trimmedName,
        color: catColor,
        productive: catProductive,
      });
      resetForm();
      triggerRefetch();
    } catch (err) {
      setFormError(t("apps.cat_error_generic", "Failed to save category."));
      console.error(err);
    }
  };

  const handleEditInit = (cat: typeof cats[0]) => {
    setEditingId(cat.id);
    setCatName(cat.name);
    setCatColor(cat.color || PALETTE[0]);
    setCatProductive(cat.productive ?? true);
    setFormError("");
  };

  const handleDeleteCategory = async (id: number, name: string) => {
    const confirmed = window.confirm(
      `${t("apps.cat_delete_confirm", "Are you sure you want to delete the category")} "${name}"? ${t(
        "apps.cat_delete_warn",
        "Apps inside this group will revert to Uncategorized."
      )}`
    );
    if (!confirmed) return;

    try {
      await deleteCategory(id);
      if (editingId === id) resetForm();
      triggerRefetch();
    } catch (err) {
      console.error(err);
    }
  };

  // --- Filtering Filtered Apps ---
  const filtered = useMemo(() => {
    const list = apps || [];
    if (!query.trim()) return list;
    const q = query.toLowerCase();
    return list.filter(
      (a) =>
        a.display_name.toLowerCase().includes(q) ||
        a.app_key.toLowerCase().includes(q)
    );
  }, [apps, query]);

  if (appsLoading || catsLoading) {
    return (
      <div className="flex h-48 items-center justify-center">
        <Spinner label={t("apps.loading", "Loading system data...")} />
      </div>
    );
  }

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

      {/* App list + re-categorize header */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <CardTitle>{t("apps.list_title", "Apps")}</CardTitle>
          <button
            onClick={() => setIsModalOpen(true)}
            className="flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-label font-medium text-text transition-colors hover:bg-bg"
          >
            <Settings2 className="h-3.5 w-3.5" />
            {t("apps.manage_categories", "Manage Categories")}
          </button>
        </div>
        
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

      {/* --- Overlay Category Management Modal Dialog --- */}
      {isModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 backdrop-blur-sm">
          <div className="w-full max-w-xl rounded-lg border border-border bg-surface p-6 shadow-xl space-y-4 max-h-[90vh] overflow-y-auto">
            <div className="flex items-center justify-between border-b border-border pb-3">
              <h3 className="text-body-strong font-semibold text-text">
                {t("apps.manage_categories_modal", "Custom Category Configuration")}
              </h3>
              <button
                onClick={() => { setIsModalOpen(false); resetForm(); }}
                className="text-text-muted hover:text-text p-1 transition-colors"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            {/* Layout Split: Form on Left/Top, List on Right/Bottom */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* Form Input Block */}
              <form onSubmit={handleSaveCategory} className="space-y-3">
                <h4 className="text-label font-medium text-text-muted">
                  {editingId !== null ? t("apps.edit_cat", "Modify Selection") : t("apps.add_cat", "Create Custom Group")}
                </h4>
                
                <div className="space-y-1">
                  <label className="text-label text-text-muted" htmlFor="cat-name-input">
                    {t("apps.cat_name_label", "Category Name")}
                  </label>
                  <input
                    id="cat-name-input"
                    type="text"
                    value={catName}
                    onChange={(e) => setCatName(e.target.value)}
                    placeholder="e.g., Design Tools"
                    className="w-full rounded-md border border-border bg-bg px-3 py-1.5 text-body text-text"
                  />
                </div>

                {/* Color Chooser Palette */}
                <div className="space-y-1">
                  <span className="text-label text-text-muted block">
                    {t("apps.cat_color_label", "Theme Color")}
                  </span>
                  <div className="flex flex-wrap gap-2 pt-1">
                    {PALETTE.map((hex) => (
                      <button
                        key={hex}
                        type="button"
                        onClick={() => setCatColor(hex)}
                        style={{ backgroundColor: hex }}
                        className={`h-6 w-6 rounded-full border transition-all ${
                          catColor === hex ? "border-text scale-110 ring-2 ring-accent/30" : "border-transparent opacity-80 hover:opacity-100"
                        }`}
                        aria-label={`Color ${hex}`}
                      />
                    ))}
                  </div>
                </div>

                {/* Productive Switch Checkbox Row */}
                <div className="flex items-center gap-2 pt-2">
                  <input
                    id="cat-productive-toggle"
                    type="checkbox"
                    checked={catProductive}
                    onChange={(e) => setCatProductive(e.target.checked)}
                    className="h-4 w-4 rounded border-border bg-bg text-accent focus:ring-accent"
                  />
                  <label htmlFor="cat-productive-toggle" className="text-body text-text select-none">
                    {t("apps.cat_productive_toggle", "Mark as Productive")}
                  </label>
                </div>

                {formError && <p className="text-label text-red-500">{formError}</p>}

                <div className="flex gap-2 pt-2">
                  <button
                    type="submit"
                    className="flex-1 rounded-md bg-accent py-1.5 text-body-strong text-white hover:opacity-90 transition-colors"
                  >
                    {editingId !== null ? t("apps.update_btn", "Save Changes") : t("apps.create_btn", "Add Category")}
                  </button>
                  {editingId !== null && (
                    <button
                      type="button"
                      onClick={resetForm}
                      className="rounded-md border border-border bg-bg px-3 py-1.5 text-body text-text hover:bg-surface transition-colors"
                    >
                      {t("apps.cancel_btn", "Cancel")}
                    </button>
                  )}
                </div>
              </form>

              {/* Existing Categories Collection */}
              <div className="space-y-2 border-t md:border-t-0 md:border-l border-border pt-4 md:pt-0 md:pl-6">
                <h4 className="text-label font-medium text-text-muted">
                  {t("apps.existing_cats", "Active System Categories")}
                </h4>
                <div className="space-y-1.5 max-h-[220px] overflow-y-auto pr-1">
                  {cats.map((c) => (
                    <div
                      key={c.id}
                      className="flex items-center justify-between rounded-md border border-border bg-bg px-3 py-2"
                    >
                      <div className="flex items-center gap-2 min-w-0">
                        <span
                          className="h-3 w-3 shrink-0 rounded-full"
                          style={{ backgroundColor: c.color || PALETTE[0] }}
                        />
                        <span className="truncate text-body font-medium text-text">
                          {c.name}
                        </span>
                        {c.productive && (
                          <span className="text-[10px] bg-green-500/10 text-green-500 px-1.5 py-0.5 rounded uppercase tracking-wider font-semibold">
                            Pro
                          </span>
                        )}
                      </div>
                      <div className="flex items-center gap-1 shrink-0">
                        <button
                          onClick={() => handleEditInit(c)}
                          className="p-1 text-text-muted hover:text-text transition-colors"
                          title="Edit"
                        >
                          <Edit2 className="h-3.5 w-3.5" />
                        </button>
                        <button
                          onClick={() => handleDeleteCategory(c.id, c.name)}
                          className="p-1 text-text-muted hover:text-red-500 transition-colors"
                          title="Delete"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}