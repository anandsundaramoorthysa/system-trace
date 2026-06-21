import { useEffect, useState } from "react";
import { Coffee, Moon, Play, AlertCircle } from "lucide-react";
import { getSettings, setSetting } from "../lib/api";
import { t } from "../lib/i18n";
import type { Settings, SettingKey } from "../lib/types";
import { Card, CardTitle, Spinner, Toggle } from "../components/ui";
import { Goals } from "../components/Goals";
import { AppGoals } from "../components/AppGoals";

function hhmmToMins(s: string): number {
  const [h, m] = s.split(":");
  return (Number(h) || 0) * 60 + (Number(m) || 0);
}

function inQuietHours(start: string, end: string): boolean {
  const now = new Date();
  const mins = now.getHours() * 60 + now.getMinutes();
  const a = hhmmToMins(start);
  const b = hhmmToMins(end);
  if (a === b) return false;
  return a < b ? mins >= a && mins < b : mins >= a || mins < b;
}

export function Wellbeing() {
  const [s, setS] = useState<Settings | null>(null);

  useEffect(() => {
    getSettings().then(setS).catch(() => {});
  }, []);

  function save(key: SettingKey, raw: string, local: Partial<Settings>) {
    setS((prev) => (prev ? { ...prev, ...local } : prev));
    void setSetting(key, raw);
  }

  function previewBreak() {
    window.dispatchEvent(
      new CustomEvent("preview-break", {
        detail: { duration_secs: s?.break_duration_secs ?? 20, strict: false },
      }),
    );
  }

  if (!s) return <Spinner label={t("wellbeing.loading", "Loading wellbeing settings")} />;

  const quiet = s.bedtime_enabled && inQuietHours(s.bedtime_start, s.bedtime_end);

  return (
    <div className="space-y-6">
      {/* Eye / posture breaks */}
      <div className="space-y-2">
        <CardTitle>{t("wellbeing.break_reminders", "Break reminders")}</CardTitle>
        <Card className="p-5">
          <div className="flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <span
                className="flex h-10 w-10 items-center justify-center rounded-lg bg-accent/15 text-accent"
                aria-hidden
              >
                <Coffee className="h-5 w-5" />
              </span>
              <div>
                <p className="text-body-strong text-text">{t("wellbeing.remind_me", "Remind me to take breaks")}</p>
                <p className="text-body text-text-muted">
                { t("wellbeing.break_desc", "A short overlay after a stretch of continuous use, to rest your eyes.")}
                </p>
              </div>
            </div>
            <Toggle
              checked={s.breaks_enabled}
              onChange={(v) => save("breaks_enabled", v ? "true" : "false", { breaks_enabled: v })}
            />
          </div>

          <div className="mt-5 grid gap-4 sm:grid-cols-3">
            <label className="flex flex-col gap-1">
              <span className="text-label text-text-muted">{t("wellbeing.every_mins", "Every (minutes)")}</span>
              <input
                type="number"
                min={5}
                max={240}
                value={s.break_interval_mins}
                disabled={!s.breaks_enabled}
                onChange={(e) =>
                  save("break_interval_mins", String(Number(e.target.value)), {
                    break_interval_mins: Number(e.target.value),
                  })
                }
                className="rounded-md border border-border bg-bg px-3 py-1.5 text-body text-text disabled:opacity-50"
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-label text-text-muted">{t("wellbeing.break_length", "Break length (seconds)")} </span>
              <input
                type="number"
                min={5}
                max={600}
                value={s.break_duration_secs}
                disabled={!s.breaks_enabled}
                onChange={(e) =>
                  save("break_duration_secs", String(Number(e.target.value)), {
                    break_duration_secs: Number(e.target.value),
                  })
                }
                className="rounded-md border border-border bg-bg px-3 py-1.5 text-body text-text disabled:opacity-50"
              />
            </label>
            <div className="flex flex-col gap-1">
              <span className="text-label text-text-muted">{t("wellbeing.strict", "Strict (cannot skip)")}</span>
              <div className="flex h-9 items-center">
                <Toggle
                  checked={s.break_strict}
                  disabled={!s.breaks_enabled}
                  onChange={(v) => save("break_strict", v ? "true" : "false", { break_strict: v })}
                />
              </div>
            </div>
          </div>

          <button
            type="button"
            onClick={previewBreak}
            className="mt-5 flex items-center gap-2 rounded-md border border-border bg-surface px-3 py-2 text-body-strong text-text hover:bg-surface-2"
          >
            <Play className="h-4 w-4" aria-hidden />{ t("wellbeing.preview_break", "Preview a break")}
          </button>
        </Card>
      </div>

      {/* Distraction nudges */}
      <div className="space-y-2">
        <CardTitle>{t("wellbeing.distraction_nudges", "Distraction nudges")}</CardTitle>
        <Card className="p-5">
          <div className="flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <span
                className="flex h-10 w-10 items-center justify-center rounded-lg bg-warning/15 text-warning"
                aria-hidden
              >
                <AlertCircle className="h-5 w-5" />
              </span>
              <div>
                <p className="text-body-strong text-text">{t("wellbeing.nudge_me", "Nudge me on sustained distracting use")}</p>
                <p className="text-body text-text-muted">
                 { t("wellbeing.nudge_desc", "After this many continuous minutes on an app whose category is marked distracting, a quiet toast appears. Quiet hours suppress these.")}
                </p>
              </div>
            </div>
            <Toggle
              checked={s.distraction_nudges_enabled}
              onChange={(v) =>
                save("distraction_nudges_enabled", v ? "true" : "false", {
                  distraction_nudges_enabled: v,
                })
              }
            />
          </div>
          <div className="mt-5 grid gap-4 sm:grid-cols-2">
            <label className="flex flex-col gap-1">
              <span className="text-label text-text-muted">{t("wellbeing.threshold", "Threshold (minutes)")}</span>
              <input
                type="number"
                min={5}
                max={240}
                value={s.distraction_threshold_mins}
                disabled={!s.distraction_nudges_enabled}
                onChange={(e) =>
                  save("distraction_threshold_mins", e.target.value, {
                    distraction_threshold_mins: Number(e.target.value) || 0,
                  })
                }
                className="rounded-md border border-border bg-bg px-3 py-1.5 text-body text-text disabled:opacity-50"
              />
            </label>
          </div>
        </Card>
      </div>

      {/* Bedtime / wind-down */}
      <div className="space-y-2">
        <CardTitle>{t("wellbeing.wind_down", "Wind-down (quiet hours)")}</CardTitle>
        <Card className="p-5">
          <div className="flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <span
                className="flex h-10 w-10 items-center justify-center rounded-lg bg-accent/15 text-accent"
                aria-hidden
              >
                <Moon className="h-5 w-5" />
              </span>
              <div>
                <p className="text-body-strong text-text">{t("wellbeing.quiet_hours", "Quiet hours")}</p>
                <p className="text-body text-text-muted">
                 { t("wellbeing.quiet_desc", "During these hours, limit and break nudges stay quiet.")}
                  {quiet ? " Quiet hours are active now." : ""}
                </p>
              </div>
            </div>
            <Toggle
              checked={s.bedtime_enabled}
              onChange={(v) => save("bedtime_enabled", v ? "true" : "false", { bedtime_enabled: v })}
            />
          </div>

          <div className="mt-5 grid gap-4 sm:grid-cols-2">
            <label className="flex flex-col gap-1">
              <span className="text-label text-text-muted">{t("wellbeing.start", "Start")}</span>
              <input
                type="time"
                value={s.bedtime_start}
                disabled={!s.bedtime_enabled}
                onChange={(e) =>
                  save("bedtime_start", e.target.value, { bedtime_start: e.target.value })
                }
                className="rounded-md border border-border bg-bg px-3 py-1.5 text-body text-text disabled:opacity-50"
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-label text-text-muted">{t("wellbeing.end", "End")}</span>
              <input
                type="time"
                value={s.bedtime_end}
                disabled={!s.bedtime_enabled}
                onChange={(e) =>
                  save("bedtime_end", e.target.value, { bedtime_end: e.target.value })
                }
                className="rounded-md border border-border bg-bg px-3 py-1.5 text-body text-text disabled:opacity-50"
              />
            </label>
          </div>

          <div className="mt-5 flex items-start justify-between gap-4 border-t border-border pt-5">
            <div>
              <p className="text-body-strong text-text">{t("wellbeing.grayscale", "Apply OS grayscale during quiet hours")}</p>
              <p className="text-body text-text-muted">
                {t("wellbeing.grayscale_desc", "Turns the screen monochrome on Windows, macOS, and (best-effort) GNOME during quiet hours. May need a sign-out / sign-in on macOS to take full effect.")}
              </p>
            </div>
            <Toggle
              checked={s.bedtime_grayscale_enabled}
              onChange={(v) =>
                save("bedtime_grayscale_enabled", v ? "true" : "false", {
                  bedtime_grayscale_enabled: v,
                })
              }
            />
          </div>
        </Card>
      </div>

      <Goals />
      <AppGoals />
    </div>
  );
}
