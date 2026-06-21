import { Clock, AppWindow, Timer, Repeat, Gauge } from "lucide-react";
import { t } from "../lib/i18n";
import { getFocusScore, getSettings, getTodayOverview } from "../lib/api";
import { useAsync } from "../lib/useAsync";
import { formatDelta, formatDuration } from "../lib/format";
import { Card, CardTitle, Spinner } from "../components/ui";
import { StatCard } from "../components/StatCard";
import { HourChart } from "../components/HourChart";
import { CategoryDonut } from "../components/CategoryDonut";
import { TopApps } from "../components/TopApps";

export function Dashboard({ liveTotalMs }: { liveTotalMs: number | null }) {
  const { data, loading, error } = useAsync(getTodayOverview, []);
  const { data: settings } = useAsync(getSettings, []);
  const scoringOn = settings?.scoring_enabled ?? false;
  const { data: focusScore } = useAsync(
    () => (scoringOn ? getFocusScore() : Promise.resolve(null)),
    [scoringOn],
  );

  if (loading && !data) return <Spinner label="Loading today" />;
  if (error && !data)
    return <p className="text-body text-negative">Could not load: {error}</p>;
  if (!data) return null;

  const total = liveTotalMs ?? data.total_ms;
  const mostUsed = data.top_apps[0];

  return (
    <div className="space-y-5">
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-4">
        <Card className="p-5 lg:col-span-2">
          <div className="flex items-center gap-2 text-text-muted">
            <Clock className="h-4 w-4" aria-hidden />
            <span className="text-label uppercase tracking-wide">Screen Time Today</span>
          </div>
          <div className="mt-2 text-display text-text">{formatDuration(total)}</div>
          <div className="mt-1 text-body text-text-muted">
            {formatDelta(data.delta_vs_yesterday_ms)} vs yesterday
          </div>
        </Card>

        <StatCard
          icon={<AppWindow className="h-4 w-4" />}
          label={t("dashboard.most_used", "Most Used")}
          value={mostUsed ? mostUsed.display_name : "-"}
          hint={mostUsed ? formatDuration(mostUsed.total_ms) : "No usage yet"}
        />
        <StatCard
          icon={<Timer className="h-4 w-4" />}
          label={t("dashboard.longest_session", "Longest Session")}
          value={formatDuration(data.longest_session_ms)}
          hint={data.longest_session_app ?? "No usage yet"}
        />
        {scoringOn && focusScore ? (
          <StatCard
            icon={<Gauge className="h-4 w-4" />}
            label={t("dashboard.focus_score", "Focus Score")}
            value={`${focusScore.score}`}
            hint={
              focusScore.productive_ms + focusScore.distracting_ms === 0
                ? t("dashboard.categorize_apps", "Categorize apps to score")
                : `${formatDuration(focusScore.productive_ms)} productive / ${formatDuration(focusScore.distracting_ms)} distracting`
            }
          />
        ) : null}
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        <Card className="p-5 lg:col-span-2">
          <div className="mb-3 flex items-center justify-between">
            <CardTitle>{t("dashboard.by_hour", "Today, by hour")}</CardTitle>
            <span className="flex items-center gap-1.5 text-label text-text-muted">
              <Repeat className="h-3.5 w-3.5" aria-hidden /> {data.app_switches} switches
            </span>
          </div>
          <HourChart data={data.by_hour} />
        </Card>

        <Card className="p-5">
          <div className="mb-3">
            <CardTitle>{t("dashboard.categories", "Categories")}</CardTitle>
          </div>
          <CategoryDonut data={data.by_category} />
        </Card>
      </div>

      <Card className="p-5">
        <div className="mb-4">
          <CardTitle>{t("dashboard.top_apps", "Top apps")}</CardTitle>
        </div>
        <TopApps data={data.top_apps} />
      </Card>
    </div>
  );
}
