import { useMemo, useState } from "react";
import { CalendarDays, Clock, TrendingUp, Flame } from "lucide-react";
import { getRangeOverview } from "../lib/api";
import { useAsync } from "../lib/useAsync";
import { dayKeyOffset, formatDayLabel, formatDelta, formatDuration } from "../lib/format";
import { Card, CardTitle, Segmented, Spinner } from "../components/ui";
import { StatCard } from "../components/StatCard";
import { TopApps } from "../components/TopApps";
import { CategoryDonut } from "../components/CategoryDonut";
import type { DayTotal } from "../lib/types";

type Range = "week" | "month";

function DayBars({ data }: { data: DayTotal[] }) {
  const max = Math.max(1, ...data.map((d) => d.total_ms));
  return (
    <div className="flex h-44 items-end gap-1.5">
      {data.map((d) => (
        <div key={d.day} className="flex min-w-0 flex-1 flex-col items-center gap-2">
          <div className="flex w-full flex-1 items-end">
            <div
              className="w-full rounded-t bg-accent/80"
              style={{ height: `${(d.total_ms / max) * 100}%` }}
              title={`${d.day}: ${formatDuration(d.total_ms)}`}
            />
          </div>
          <span className="truncate text-label text-text-muted">
            {formatDayLabel(d.day)}
          </span>
        </div>
      ))}
    </div>
  );
}

export function Reports() {
  const [range, setRange] = useState<Range>("week");
  const { from, to } = useMemo(
    () => ({ from: dayKeyOffset(range === "week" ? 6 : 29), to: dayKeyOffset(0) }),
    [range],
  );
  const { data, loading } = useAsync(() => getRangeOverview(from, to), [from, to]);

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between">
        <CardTitle>Overview</CardTitle>
        <Segmented<Range>
          value={range}
          onChange={setRange}
          options={[
            { value: "week", label: "Last 7 days" },
            { value: "month", label: "Last 30 days" },
          ]}
        />
      </div>

      {loading && !data ? (
        <Spinner label="Loading report" />
      ) : !data ? null : (
        <>
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
        </>
      )}
    </div>
  );
}
