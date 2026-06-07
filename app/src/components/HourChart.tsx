import type { HourBucket } from "../lib/types";
import { formatDurationShort } from "../lib/format";

/**
 * "Today, by hour" area chart, 24 buckets. Dependency-free SVG so it always
 * renders; the accent color comes from the Signal CSS variable, so it is
 * theme-correct. (uPlot is reserved for very large multi-day series later.)
 */
export function HourChart({ data }: { data: HourBucket[] }) {
  const W = 720;
  const H = 180;
  const padX = 10;
  const padT = 12;
  const padB = 22;
  const n = Math.max(data.length, 2);
  const innerW = W - padX * 2;
  const innerH = H - padT - padB;
  const max = Math.max(1, ...data.map((d) => d.active_ms));

  const x = (i: number) => padX + (i / (n - 1)) * innerW;
  const y = (v: number) => padT + innerH - (v / max) * innerH;

  const line = data
    .map((d, i) => `${i === 0 ? "M" : "L"}${x(i).toFixed(1)},${y(d.active_ms).toFixed(1)}`)
    .join(" ");
  const area = `${line} L${x(n - 1).toFixed(1)},${padT + innerH} L${x(0).toFixed(1)},${padT + innerH} Z`;

  const ticks = [0, 6, 12, 18, 23];

  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      className="h-44 w-full"
      role="img"
      aria-label="Active time by hour of day"
      preserveAspectRatio="none"
    >
      <defs>
        <linearGradient id="hour-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="var(--accent)" stopOpacity="0.30" />
          <stop offset="100%" stopColor="var(--accent)" stopOpacity="0.02" />
        </linearGradient>
      </defs>

      {/* baseline */}
      <line
        x1={padX}
        y1={padT + innerH}
        x2={W - padX}
        y2={padT + innerH}
        stroke="var(--border)"
        strokeWidth="1"
      />

      <path d={area} fill="url(#hour-fill)" />
      <path
        d={line}
        fill="none"
        stroke="var(--accent)"
        strokeWidth="2.5"
        strokeLinejoin="round"
        strokeLinecap="round"
        vectorEffect="non-scaling-stroke"
      />

      {ticks.map((h) => (
        <text
          key={h}
          x={x(h)}
          y={H - 6}
          textAnchor="middle"
          fontSize="11"
          fill="var(--text-muted)"
        >
          {h === 0 ? "12a" : h === 12 ? "12p" : h < 12 ? `${h}a` : `${h - 12}p`}
        </text>
      ))}

      <text x={padX} y={padT + 2} fontSize="11" fill="var(--text-muted)">
        {formatDurationShort(max)}
      </text>
    </svg>
  );
}
