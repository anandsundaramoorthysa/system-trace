import { useEffect, useMemo, useState } from "react";
import { CalendarDays, Clock, TrendingUp, Flame, Repeat, AppWindow, Timer } from "lucide-react";
import { getDayOverview, getRangeOverview } from "../lib/api";
import { useAsync } from "../lib/useAsync";
import {
  addDays,
  dayKeyOffset,
  formatDayLabel,
  formatDelta,
  formatDuration,
  formatLongDay,
  formatMonthLabel,
  formatWeekLabel,
  parseDayKey,
  startOfMonth,
  startOfNextMonth,
  startOfWeek,
  toDayKey,
} from "../lib/format";
import { Card, CardTitle, EmptyState, Segmented, Spinner } from "../components/ui";
import { StatCard } from "../components/StatCard";
import { TopApps } from "../components/TopApps";
import { CategoryDonut } from "../components/CategoryDonut";
import { HourChart } from "../components/HourChart";
import { DateStepper } from "../components/DateStepper";
import type { DayTotal } from "../lib/types";

type Mode = "day" | "week" | "month";

function DayBars({ data }: { data: DayTotal[] }) {
  const max = Math.max(1, ...data.map((d) => d.total_ms));
  // For wide ranges (Month view, ~28-31 days), full "Mon 1" labels collide.
  // Switch to the day number alone so every date stays visible and readable.
  const compact = data.length > 14;
  return (
    <div className="flex h-44 gap-[2px]">
      {data.map((d) => (
        <div key={d.day} className="flex min-w-0 flex-1 flex-col items-center gap-2">
          <div className="flex w-full flex-1 items-end">
            <div
              className="w-full rounded-t bg-accent/80"
              style={{
                height: `${(d.total_ms / max) * 100}%`,
                minHeight: d.total_ms > 0 ? "2px" : 0,
              }}
              title={`${d.day}: ${formatDuration(d.total_ms)}`}
            />
          </div>
          <span
            className={
              compact
                ? "text-[10px] leading-none text-text-muted tabular-nums"
                : "truncate text-label text-text-muted"
            }
          >
            {compact ? parseDayKey(d.day).getDate() : formatDayLabel(d.day)}
          </span>
        </div>
      ))}
    </div>
  );
}

/* ----------------------------- Day view ---------------------------------- */

function DayView({ anchor }: { anchor: string }) {
  const { data, loading, error } = useAsync(() => getDayOverview(anchor), [anchor]);

  if (loading && !data) return <Spinner label="Loading day" />;
  if (error && !data)
    return <p className="text-body text-negative">Could not load: {error}</p>;
  if (!data) return null;

  if (data.total_ms === 0) {
    return (
      <Card className="p-8">
        <EmptyState
          icon={<CalendarDays className="h-7 w-7" />}
          title="No activity on this day"
          description="Either nothing was tracked, or the day is older than the raw-event retention window."
        />
      </Card>
    );
  }

  const mostUsed = data.top_apps[0];

  return (
    <div className="space-y-5">
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-4">
        <Card className="p-5 lg:col-span-2">
          <div className="flex items-center gap-2 text-text-muted">
            <Clock className="h-4 w-4" aria-hidden />
            <span className="text-label uppercase tracking-wide">Screen Time</span>
          </div>
          <div className="mt-2 text-display text-text">{formatDuration(data.total_ms)}</div>
          <div className="mt-1 text-body text-text-muted">
            {formatDelta(data.delta_vs_yesterday_ms)} vs the day before
          </div>
        </Card>
        <StatCard
          icon={<AppWindow className="h-4 w-4" />}
          label="Most Used"
          value={mostUsed ? mostUsed.display_name : "-"}
          hint={mostUsed ? formatDuration(mostUsed.total_ms) : "No usage"}
        />
        <StatCard
          icon={<Timer className="h-4 w-4" />}
          label="Longest Session"
          value={formatDuration(data.longest_session_ms)}
          hint={data.longest_session_app ?? "No usage"}
        />
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        <Card className="p-5 lg:col-span-2">
          <div className="mb-3 flex items-center justify-between">
            <CardTitle>By hour</CardTitle>
            <span className="flex items-center gap-1.5 text-label text-text-muted">
              <Repeat className="h-3.5 w-3.5" aria-hidden /> {data.app_switches} switches
            </span>
          </div>
          <HourChart data={data.by_hour} />
        </Card>
        <Card className="p-5">
          <div className="mb-3">
            <CardTitle>Categories</CardTitle>
          </div>
          <CategoryDonut data={data.by_category} />
        </Card>
      </div>

      <Card className="p-5">
        <div className="mb-4">
          <CardTitle>Top apps</CardTitle>
        </div>
        <TopApps data={data.top_apps} />
      </Card>
    </div>
  );
}

/* --------------------------- Range view (week/month) --------------------- */

function RangeView({ from, to }: { from: string; to: string }) {
  const { data, loading, error } = useAsync(() => getRangeOverview(from, to), [from, to]);

  if (loading && !data) return <Spinner label="Loading range" />;
  if (error && !data)
    return <p className="text-body text-negative">Could not load: {error}</p>;
  if (!data) return null;

  return (
    <div className="space-y-5">
      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        <StatCard
          icon={<Clock className="h-4 w-4" />}
          label="Total"
          value={formatDuration(data.total_ms)}
          hint={`${formatDelta(data.total_ms - data.prev_total_ms)} vs previous`}
          hintTone="muted"
        />
        <StatCard
          icon={<TrendingUp className="h-4 w-4" />}
          label="Daily Average"
          value={formatDuration(data.daily_average_ms)}
        />
        <StatCard
          icon={<Flame className="h-4 w-4" />}
          label="Busiest Day"
          value={data.busiest_day ? formatDayLabel(data.busiest_day) : "-"}
        />
        <StatCard
          icon={<CalendarDays className="h-4 w-4" />}
          label="Days Tracked"
          value={String(data.by_day.filter((d) => d.total_ms > 0).length)}
        />
      </div>

      <Card className="p-5">
        <div className="mb-4">
          <CardTitle>Daily usage</CardTitle>
        </div>
        <DayBars data={data.by_day} />
      </Card>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <Card className="p-5">
          <div className="mb-4">
            <CardTitle>Top apps</CardTitle>
          </div>
          <TopApps data={data.top_apps} />
        </Card>
        <Card className="p-5">
          <div className="mb-4">
            <CardTitle>Categories</CardTitle>
          </div>
          <CategoryDonut data={data.by_category} />
        </Card>
      </div>
    </div>
  );
}

/* ----------------------------- page shell -------------------------------- */

export function Reports() {
  const today = dayKeyOffset(0);
  const [mode, setMode] = useState<Mode>("week");
  const [anchor, setAnchor] = useState<string>(today);

  // Snap the anchor back into the current period whenever the mode changes,
  // so switching modes never lands on a confusing partial window.
  useEffect(() => {
    setAnchor(today);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode]);

  const { rangeFrom, rangeTo, atPresent, label, resetLabel } = useMemo(() => {
    if (mode === "day") {
      return {
        rangeFrom: anchor,
        rangeTo: anchor,
        atPresent: anchor === today,
        label: formatLongDay(anchor),
        resetLabel: "Today",
      };
    }
    if (mode === "week") {
      const start = startOfWeek(anchor);
      const end = addDays(start, 6);
      return {
        rangeFrom: start,
        rangeTo: end,
        atPresent: start === startOfWeek(today),
        label: formatWeekLabel(start),
        resetLabel: "This week",
      };
    }
    const start = startOfMonth(anchor);
    const next = startOfNextMonth(anchor);
    const end = addDays(next, -1);
    return {
      rangeFrom: start,
      rangeTo: end,
      atPresent: start === startOfMonth(today),
      label: formatMonthLabel(start),
      resetLabel: "This month",
    };
  }, [mode, anchor, today]);

  const step = (dir: 1 | -1) => {
    if (mode === "day") {
      setAnchor(addDays(anchor, dir));
      return;
    }
    if (mode === "week") {
      setAnchor(addDays(startOfWeek(anchor), dir * 7));
      return;
    }
    // month
    const start = startOfMonth(anchor);
    if (dir > 0) {
      setAnchor(startOfNextMonth(start));
    } else {
      // Subtract one day from start of month to land in the previous month, then snap.
      setAnchor(startOfMonth(addDays(start, -1)));
    }
  };

  // Keyboard nav: left / right arrow steps the anchor (don't hijack when
  // typing in inputs).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName)) return;
      if (e.key === "ArrowLeft") step(-1);
      else if (e.key === "ArrowRight" && !atPresent) step(1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode, anchor, atPresent]);

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <CardTitle>Overview</CardTitle>
          <Segmented<Mode>
            value={mode}
            onChange={setMode}
            options={[
              { value: "day", label: "Day" },
              { value: "week", label: "Week" },
              { value: "month", label: "Month" },
            ]}
          />
        </div>
        <DateStepper
          label={label}
          onPrev={() => step(-1)}
          onNext={() => step(1)}
          onReset={() => setAnchor(today)}
          atPresent={atPresent}
          resetLabel={resetLabel}
        />
      </div>

      {mode === "day" ? (
        <DayView anchor={rangeFrom} />
      ) : (
        <RangeView from={rangeFrom} to={rangeTo} />
      )}
    </div>
  );
}

// Avoid unused-import warning for toDayKey (kept for future calendar picker).
void toDayKey;
