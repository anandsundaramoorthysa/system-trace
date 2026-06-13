import { useEffect, useState } from "react";
import { Target, Plus, Trash2 } from "lucide-react";
import {
  getCategories,
  getCategoryGoals,
  removeCategoryGoal,
  setCategoryGoal,
} from "../lib/api";
import type { Category, CategoryGoal, GoalKind } from "../lib/types";
import { Card, CardTitle, EmptyState, cx } from "./ui";
import { formatDuration } from "../lib/format";

function ratio(g: CategoryGoal): { pct: number; met: boolean; color: string } {
  const pct = g.daily_ms > 0 ? Math.min(100, (g.today_ms / g.daily_ms) * 100) : 0;
  if (g.kind === "under") {
    const met = g.today_ms <= g.daily_ms;
    return { pct, met, color: pct < 80 ? "bg-accent" : pct < 100 ? "bg-warning" : "bg-negative" };
  }
  const met = g.today_ms >= g.daily_ms;
  return { pct, met, color: met ? "bg-positive" : "bg-accent" };
}

export function Goals() {
  const [goals, setGoals] = useState<CategoryGoal[]>([]);
  const [cats, setCats] = useState<Category[]>([]);
  const [catId, setCatId] = useState<string>("");
  const [mins, setMins] = useState(60);
  const [kind, setKind] = useState<GoalKind>("under");

  useEffect(() => {
    getCategoryGoals().then(setGoals).catch(() => {});
    getCategories().then(setCats).catch(() => {});
  }, []);

  async function addGoal() {
    if (catId === "") return;
    await setCategoryGoal({
      category_id: Number(catId),
      daily_ms: mins * 60_000,
      kind,
    });
    setGoals(await getCategoryGoals());
    setCatId("");
  }

  async function dropGoal(id: number) {
    await removeCategoryGoal(id);
    setGoals((g) => g.filter((x) => x.category_id !== id));
  }

  const taken = new Set(goals.map((g) => g.category_id));
  const available = cats.filter((c) => !taken.has(c.id));

  return (
    <div className="space-y-2">
      <CardTitle>Category goals</CardTitle>
      <Card className="p-5">
        <div className="mb-4 flex flex-wrap items-center gap-2">
          <select
            value={kind}
            onChange={(e) => setKind(e.target.value as GoalKind)}
            className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
          >
            <option value="under">Stay under</option>
            <option value="over">Reach at least</option>
          </select>
          <select
            value={catId}
            onChange={(e) => setCatId(e.target.value)}
            className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
          >
            <option value="">Choose a category</option>
            {available.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
          <div className="flex items-center gap-1.5">
            <input
              type="number"
              min={5}
              max={1440}
              value={mins}
              onChange={(e) => setMins(Number(e.target.value))}
              className="w-20 rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
            />
            <span className="text-body text-text-muted">min/day</span>
          </div>
          <button
            type="button"
            onClick={addGoal}
            className="flex items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-body-strong text-white"
          >
            <Plus className="h-4 w-4" aria-hidden /> Add goal
          </button>
        </div>

        {goals.length === 0 ? (
          <EmptyState
            icon={<Target className="h-7 w-7" />}
            title="No goals yet"
            description="Set a daily target on a category to see progress at a glance."
          />
        ) : (
          <ul className="space-y-3">
            {goals.map((g) => {
              const r = ratio(g);
              return (
                <li key={g.category_id}>
                  <div className="mb-1 flex items-center justify-between gap-3 text-body">
                    <span className="flex min-w-0 items-center gap-2">
                      <span
                        className="h-2.5 w-2.5 shrink-0 rounded-full"
                        style={{ backgroundColor: g.color ?? "var(--accent)" }}
                        aria-hidden
                      />
                      <span className="truncate font-medium text-text">{g.category_name}</span>
                      <span className="text-label text-text-muted">
                        {g.kind === "under" ? "stay under" : "reach at least"}
                      </span>
                    </span>
                    <span className="flex items-center gap-3">
                      <span className={cx("font-medium", r.met ? "text-positive" : "text-text")}>
                        {formatDuration(g.today_ms)} / {formatDuration(g.daily_ms)}
                      </span>
                      <button
                        type="button"
                        onClick={() => dropGoal(g.category_id)}
                        className="text-text-muted hover:text-negative"
                        aria-label={`Remove goal for ${g.category_name}`}
                      >
                        <Trash2 className="h-4 w-4" aria-hidden />
                      </button>
                    </span>
                  </div>
                  <div className="h-2 w-full overflow-hidden rounded-full bg-bg">
                    <div
                      className={cx("h-full rounded-full", r.color)}
                      style={{ width: `${r.pct}%` }}
                    />
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </Card>
    </div>
  );
}
