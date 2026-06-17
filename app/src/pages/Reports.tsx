import { useEffect, useMemo, useState } from "react";
import {
  CalendarDays,
  Clock,
  TrendingUp,
  Flame,
  Repeat,
  AppWindow,
  Timer,
  FileDown,
} from "lucide-react";
import { t } from "../lib/i18n";
import { getDayOverview, getRangeOverview } from "../lib/api";
import { downloadReportPdf } from "../lib/pdf";
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

  if (loading && !data) return <Spinner label={t("reports.loading_day", "Loading day")} />;
  if (error && !data)
    return <p className="text-body text-negative">{t("reports.error_load", `Could not load: ${error}`)}</p>;
  if (!data) return null;

  if (data.total_ms === 0) {
    return (
      <Card className="p-8">
        <EmptyState
          icon={<CalendarDays className="h-7 w-7" />}
          title={t("reports.no_activity_title", "No activity on this day")}
          description={t("reports.no_activity_desc", "Either nothing was tracked, or the day is older than the raw-event retention window.")}
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
            <span className="text-label uppercase tracking-wide">{t("reports.screen_time", "Screen Time")}</span>
          </div>
          <div className="mt-2 text-display text-text">{formatDuration(data.total_ms)}</div>
          <div className="mt-1 text-body text-text-muted">
            {formatDelta(data.delta_vs_yesterday_ms)} {t("reports.vs_prev", "${formatDelta(data.delta_vs_yesterday_ms)} vs the day before")}
          </div>
        </Card>
        <StatCard
          icon={<AppWindow className="h-4 w-4" />}
          label={t("reports.most_used", "Most Used")}
          value={mostUsed ? mostUsed.display_name : "-"}
          hint={mostUsed ? formatDuration(mostUsed.total_ms) : t("reports.no_usage", "No usage")}
        />
        <StatCard
          icon={<Timer className="h-4 w-4" />}
          label={t("reports.longest_session", "Longest Session")}
          value={formatDuration(data.longest_session_ms)}
          hint={data.longest_session_app ?? t("reports.no_usage", "No usage")}
        />
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        <Card className="p-5 lg:col-span-2">
          <div className="mb-3 flex items-center justify-between">
            <CardTitle>{t("reports.by_hour", "By hour")}</CardTitle>
            <span className="flex items-center gap-1.5 text-label text-text-muted">
              <Repeat className="h-3.5 w-3.5" aria-hidden />{t("reports.switches", "${data.app_switches} switches")}
            </span>
          </div>
          <HourChart data={data.by_hour} />
        </Card>
        <Card className="p-5">
          <div className="mb-3">
            <CardTitle>{t("reports.categories", "Categories")}</CardTitle>
          </div>
          <CategoryDonut data={data.by_category} />
        </Card>
      </div>

      <Card className="p-5">
        <div className="mb-4">
          <CardTitle>{t("reports.top_apps", "Top apps")}</CardTitle>
        </div>
        <TopApps data={data.top_apps} />
      </Card>
    </div>
  );
}

/* --------------------------- Range view (week/month) --------------------- */

function RangeView({ from, to }: { from: string; to: string }) {
  const { data, loading, error } = useAsync(() => getRangeOverview(from, to), [from, to]);

  if (loading && !data) return <Spinner label={t("reports.loading_range", "Loading range")} />;
  if (error && !data)
    return <p className="text-body text-negative">{t("reports.error_load", `Could not load: ${error}`)}</p>;
  if (!data) return null;

  return (
    <div className="space-y-5">
      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        <StatCard
          icon={<Clock className="h-4 w-4" />}
          label={t("reports.total", "Total")}
          value={formatDuration(data.total_ms)}
          hint={`${formatDelta(data.total_ms - data.prev_total_ms)} vs previous`}
          hintTone="muted"
        />
        <StatCard
          icon={<TrendingUp className="h-4 w-4" />}
          label={t("reports.daily_avg", "Daily Average")}
          value={formatDuration(data.daily_average_ms)}
        />
        <StatCard
          icon={<Flame className="h-4 w-4" />}
          label={t("reports.busiest_day", "Busiest Day")}
          value={data.busiest_day ? formatDayLabel(data.busiest_day) : "-"}
        />
        <StatCard
          icon={<CalendarDays className="h-4 w-4" />}
          label={t("reports.days_tracked", "Days tracked")}
          value={String(data.by_day.filter((d) => d.total_ms > 0).length)}
        />
      </div>

      <Card className="p-5">
        <div className="mb-4">
          <CardTitle>{t("reports.daily_usage", "Daily usage")}</CardTitle>
        </div>
        <DayBars data={data.by_day} />
      </Card>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <Card className="p-5">
          <div className="mb-4">
            <CardTitle>{t("reports.top_apps", "Top apps")}</CardTitle>
          </div>
          <TopApps data={data.top_apps} />
        </Card>
        <Card className="p-5">
          <div className="mb-4">
            <CardTitle>{t("reports.categories", "Categories")}</CardTitle>
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
  const [pdfMsg, setPdfMsg] = useState<string | null>(null);
  const [pdfBusy, setPdfBusy] = useState(false);

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
        resetLabel: t("reports.today", "Today"),
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
        resetLabel: t("reports.this_week", "This week"),
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
      resetLabel: t("reports.this_month", "This month"),
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

  async function downloadPdf() {
    if (pdfBusy) return;
    setPdfBusy(true);
    setPdfMsg(t("reports.building_pdf", "Building PDF..."));
    try {
      const msg = await downloadReportPdf({ mode, from: rangeFrom, to: rangeTo, label });
      setPdfMsg(msg || null);
    } catch (e) {
      setPdfMsg(t("reports.export_error",`Could not export: ${e instanceof Error ? e.message : String(e)}`));
    } finally {
      setPdfBusy(false);
      window.setTimeout(() => setPdfMsg(null), 4000);
    }
  }

  return (
    <div className="space-y-5">
      {pdfMsg ? (
        <div
          className="fixed bottom-6 left-1/2 z-40 -translate-x-1/2 rounded-md border border-accent/40 bg-surface px-4 py-2 text-body text-text shadow-e2 dark:shadow-e2-dark"
          role="status"
          aria-live="polite"
        >
          {pdfMsg}
        </div>
      ) : null}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <CardTitle>{t("reports.overview", "Overview")}</CardTitle>
          <Segmented<Mode>
            value={mode}
            onChange={setMode}
            options={[
              { value: "day", label: t("reports.day", "Day") },
              { value: "week", label: t("reports.week", "Week") },
              { value: "month", label: t("reports.month", "Month") },
            ]}
          />
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={downloadPdf}
            disabled={pdfBusy}
            className="flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2 disabled:opacity-60"
          >
            <FileDown className="h-4 w-4" aria-hidden /> {pdfBusy ? t("reports.exporting", "Exporting...") : t("reports.pdf", "PDF")}
          </button>
          <DateStepper
            label={label}
            onPrev={() => step(-1)}
            onNext={() => step(1)}
            onReset={() => setAnchor(today)}
            atPresent={atPresent}
            resetLabel={resetLabel}
          />
        </div>
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
