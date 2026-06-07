import type { CategoryUsage } from "../lib/types";
import { formatDuration } from "../lib/format";
import { EmptyState } from "./ui";
import { PieChart } from "lucide-react";

const FALLBACK = ["#2DD4BF", "#0EA5A0", "#34D399", "#F59E0B", "#F87171", "#8B949E", "#656D76"];

/** Category split donut + legend. Dependency-free SVG. */
export function CategoryDonut({ data }: { data: CategoryUsage[] }) {
  const total = data.reduce((s, d) => s + d.total_ms, 0);
  if (total <= 0) {
    return (
      <EmptyState
        icon={<PieChart className="h-7 w-7" />}
        title="No category data yet"
        description="As you use your computer, time is grouped by category here."
      />
    );
  }

  const r = 52;
  const circumference = 2 * Math.PI * r;
  let acc = 0;
  const segments = data.map((d, i) => {
    const frac = d.total_ms / total;
    const len = frac * circumference;
    const seg = {
      color: d.color ?? FALLBACK[i % FALLBACK.length],
      dash: len,
      gap: circumference - len,
      offset: -acc,
      name: d.name,
      total: d.total_ms,
      pct: Math.round(frac * 100),
    };
    acc += len;
    return seg;
  });

  return (
    <div className="flex items-center gap-6">
      <svg viewBox="0 0 140 140" className="h-36 w-36 shrink-0">
        <g transform="rotate(-90 70 70)">
          <circle cx="70" cy="70" r={r} fill="none" stroke="var(--border)" strokeWidth="16" />
          {segments.map((s, i) => (
            <circle
              key={i}
              cx="70"
              cy="70"
              r={r}
              fill="none"
              stroke={s.color}
              strokeWidth="16"
              strokeDasharray={`${s.dash} ${s.gap}`}
              strokeDashoffset={s.offset}
            />
          ))}
        </g>
        <text
          x="70"
          y="66"
          textAnchor="middle"
          fontSize="11"
          fill="var(--text-muted)"
        >
          Total
        </text>
        <text
          x="70"
          y="82"
          textAnchor="middle"
          fontSize="14"
          fontWeight="600"
          fill="var(--text)"
        >
          {formatDuration(total)}
        </text>
      </svg>

      <ul className="min-w-0 flex-1 space-y-2">
        {segments.map((s, i) => (
          <li key={i} className="flex items-center gap-2 text-body">
            <span
              className="h-2.5 w-2.5 shrink-0 rounded-full"
              style={{ backgroundColor: s.color }}
              aria-hidden
            />
            <span className="min-w-0 flex-1 truncate text-text">{s.name}</span>
            <span className="text-text-muted">{s.pct}%</span>
            <span className="w-16 text-right font-medium text-text">
              {formatDuration(s.total)}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
