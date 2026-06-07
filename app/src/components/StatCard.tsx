import type { ReactNode } from "react";
import { Card, cx } from "./ui";

type Tone = "muted" | "accent" | "positive" | "negative" | "warning";

const TONE: Record<Tone, string> = {
  muted: "text-text-muted",
  accent: "text-accent",
  positive: "text-positive",
  negative: "text-negative",
  warning: "text-warning",
};

export function StatCard({
  icon,
  label,
  value,
  hint,
  hintTone = "muted",
}: {
  icon: ReactNode;
  label: string;
  value: string;
  hint?: string;
  hintTone?: Tone;
}) {
  return (
    <Card className="p-4">
      <div className="flex items-center gap-2 text-text-muted">
        <span aria-hidden>{icon}</span>
        <span className="text-label uppercase tracking-wide">{label}</span>
      </div>
      <div className="mt-2 text-stat font-semibold text-text">{value}</div>
      {hint ? <div className={cx("mt-1 text-label", TONE[hintTone])}>{hint}</div> : null}
    </Card>
  );
}
