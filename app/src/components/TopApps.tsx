import type { UsageEntry } from "../lib/types";
import { formatDuration } from "../lib/format";
import { EmptyState } from "./ui";
import { AppWindow } from "lucide-react";

/** Horizontal bar list of the most-used apps. */
export function TopApps({ data }: { data: UsageEntry[] }) {
  if (data.length === 0) {
    return (
      <EmptyState
        icon={<AppWindow className="h-7 w-7" />}
        title="No app usage yet"
        description="Apps you use will appear here, ranked by time."
      />
    );
  }
  const max = Math.max(1, ...data.map((d) => d.total_ms));

  return (
    <ul className="space-y-3">
      {data.map((d) => (
        <li key={d.app_id}>
          <div className="mb-1 flex items-center justify-between gap-3 text-body">
            <span className="flex min-w-0 items-center gap-2">
              <span
                className="h-2.5 w-2.5 shrink-0 rounded-full"
                style={{ backgroundColor: d.color ?? "var(--accent)" }}
                aria-hidden
              />
              <span className="truncate font-medium text-text">{d.display_name}</span>
              {d.category_name ? (
                <span className="truncate text-label text-text-muted">
                  {d.category_name}
                </span>
              ) : null}
            </span>
            <span className="shrink-0 font-medium text-text">
              {formatDuration(d.total_ms)}
            </span>
          </div>
          <div className="h-2 w-full overflow-hidden rounded-full bg-bg">
            <div
              className="h-full rounded-full"
              style={{
                width: `${(d.total_ms / max) * 100}%`,
                backgroundColor: d.color ?? "var(--accent)",
              }}
            />
          </div>
        </li>
      ))}
    </ul>
  );
}
