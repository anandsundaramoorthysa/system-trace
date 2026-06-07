import { useEffect, useState, type ReactNode } from "react";
import { Plus, Trash2, Download, Upload, AlertTriangle } from "lucide-react";
import { save, open } from "@tauri-apps/plugin-dialog";
import {
  addExclusion,
  exportData,
  getExclusions,
  getSettings,
  importData,
  isTauri,
  removeExclusion,
  setSetting,
  wipeAllData,
} from "../lib/api";
import type {
  ExclusionMatchType,
  ExportFormat,
  Exclusion,
  Settings as SettingsModel,
  SettingKey,
  SummaryCadence,
  ThemePreference,
} from "../lib/types";
import { useTheme } from "../theme/ThemeProvider";
import { Card, CardTitle, Segmented, Toggle, cx } from "../components/ui";

function Row({
  title,
  description,
  control,
}: {
  title: string;
  description?: string;
  control: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-6 px-5 py-3.5">
      <div className="min-w-0">
        <div className="text-body-strong text-text">{title}</div>
        {description ? (
          <div className="text-label text-text-muted">{description}</div>
        ) : null}
      </div>
      <div className="shrink-0">{control}</div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="space-y-2">
      <CardTitle>{title}</CardTitle>
      <Card className="divide-y divide-border">{children}</Card>
    </div>
  );
}

export function Settings() {
  const { setTheme } = useTheme();
  const [s, setS] = useState<SettingsModel | null>(null);
  const [exclusions, setExclusions] = useState<Exclusion[]>([]);
  const [pattern, setPattern] = useState("");
  const [matchType, setMatchType] = useState<ExclusionMatchType>("app");
  const [status, setStatus] = useState<string | null>(null);

  useEffect(() => {
    getSettings().then(setS).catch(() => {});
    getExclusions().then(setExclusions).catch(() => {});
  }, []);

  function flash(msg: string) {
    setStatus(msg);
    window.setTimeout(() => setStatus(null), 3000);
  }

  function setBool(key: SettingKey & keyof SettingsModel, val: boolean) {
    setS((prev) => (prev ? ({ ...prev, [key]: val } as SettingsModel) : prev));
    setSetting(key, val ? "true" : "false").catch(() => {});
  }

  function setNum(key: SettingKey & keyof SettingsModel, val: number) {
    setS((prev) => (prev ? ({ ...prev, [key]: val } as SettingsModel) : prev));
    setSetting(key, String(val)).catch(() => {});
  }

  function setStr(key: SettingKey & keyof SettingsModel, val: string) {
    setS((prev) => (prev ? ({ ...prev, [key]: val } as SettingsModel) : prev));
    setSetting(key, val).catch(() => {});
  }

  function chooseTheme(t: ThemePreference) {
    setS((prev) => (prev ? { ...prev, theme: t } : prev));
    setTheme(t);
  }

  async function addEx() {
    const p = pattern.trim();
    if (!p) return;
    const created = await addExclusion({ match_type: matchType, pattern: p });
    setExclusions((list) => [...list, created]);
    setPattern("");
  }

  async function removeEx(id: number) {
    await removeExclusion(id);
    setExclusions((list) => list.filter((e) => e.id !== id));
  }

  async function doExport(format: ExportFormat) {
    let path = `system-trace-export.${format}`;
    if (isTauri) {
      const picked = await save({
        defaultPath: path,
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });
      if (!picked) return;
      path = picked;
    }
    const res = await exportData(format, path);
    flash(`Exported ${res.rows_written} rows to ${res.format.toUpperCase()}.`);
  }

  async function doImport() {
    if (!isTauri) {
      flash("Import is available in the desktop app.");
      return;
    }
    const picked = await open({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!picked || Array.isArray(picked)) return;
    const res = await importData(picked);
    flash(`Merged ${res.events_merged} events across ${res.days_affected} days.`);
  }

  async function doWipe() {
    const ok = window.confirm(
      "Delete ALL local data (events, apps, exclusions)? This cannot be undone.",
    );
    if (!ok) return;
    await wipeAllData();
    flash("All local data deleted.");
  }

  if (!s) return null;

  return (
    <div className="mx-auto max-w-3xl space-y-6 pb-10">
      {status ? (
        <div className="rounded-md border border-accent/40 bg-accent/10 px-4 py-2 text-body text-text">
          {status}
        </div>
      ) : null}

      <Section title="Appearance">
        <Row
          title="Theme"
          description="System follows your OS setting."
          control={
            <Segmented<ThemePreference>
              value={s.theme}
              onChange={chooseTheme}
              options={[
                { value: "system", label: "System" },
                { value: "light", label: "Light" },
                { value: "dark", label: "Dark" },
              ]}
            />
          }
        />
      </Section>

      <Section title="Tracking">
        <Row
          title="Idle threshold"
          description="Seconds of no input before time stops counting."
          control={
            <input
              type="number"
              min={15}
              max={3600}
              value={s.idle_threshold_secs}
              onChange={(e) => setNum("idle_threshold_secs", Number(e.target.value))}
              className="w-24 rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
            />
          }
        />
        <Row
          title="Capture window titles"
          description="Off by default for privacy. Titles can contain private text."
          control={
            <Toggle checked={s.capture_titles} onChange={(v) => setBool("capture_titles", v)} />
          }
        />
        <Row
          title="Productivity scoring"
          description="Optional. Adds a Focus Score and productive/distracting labels."
          control={
            <Toggle checked={s.scoring_enabled} onChange={(v) => setBool("scoring_enabled", v)} />
          }
        />
        <Row
          title="Summary notifications"
          description="A recap of your screen time: off, daily, weekly, or both. Catches up if the app was closed."
          control={
            <Segmented<SummaryCadence>
              options={[
                { value: "off", label: "Off" },
                { value: "daily", label: "Daily" },
                { value: "weekly", label: "Weekly" },
                { value: "both", label: "Both" },
              ]}
              value={s.summary_cadence}
              onChange={(v) => setStr("summary_cadence", v)}
            />
          }
        />
      </Section>

      <Section title="Startup">
        <Row
          title="Launch at login"
          control={
            <Toggle checked={s.launch_at_login} onChange={(v) => setBool("launch_at_login", v)} />
          }
        />
        <Row
          title="Start minimized to tray"
          control={
            <Toggle checked={s.start_minimized} onChange={(v) => setBool("start_minimized", v)} />
          }
        />
      </Section>

      <Section title="Privacy and data">
        <Row
          title="Keep raw events for"
          description="Daily summaries are kept forever; raw events older than this are trimmed."
          control={
            <div className="flex items-center gap-2">
              <input
                type="number"
                min={7}
                max={3650}
                value={s.retention_days}
                onChange={(e) => setNum("retention_days", Number(e.target.value))}
                className="w-24 rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
              />
              <span className="text-body text-text-muted">days</span>
            </div>
          }
        />

        <div className="px-5 py-3.5">
          <div className="text-body-strong text-text">Exclusions</div>
          <div className="text-label text-text-muted">
            Apps or window titles that are never tracked.
          </div>
          <div className="mt-3 flex gap-2">
            <select
              value={matchType}
              onChange={(e) => setMatchType(e.target.value as ExclusionMatchType)}
              className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
            >
              <option value="app">App is</option>
              <option value="title_contains">Title contains</option>
            </select>
            <input
              value={pattern}
              onChange={(e) => setPattern(e.target.value)}
              placeholder={matchType === "app" ? "e.g. 1password.exe" : "e.g. Incognito"}
              className="flex-1 rounded-md border border-border bg-bg px-3 py-1.5 text-body text-text placeholder:text-text-muted"
            />
            <button
              type="button"
              onClick={addEx}
              className="flex items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-body-strong text-white"
            >
              <Plus className="h-4 w-4" aria-hidden /> Add
            </button>
          </div>
          {exclusions.length > 0 ? (
            <ul className="mt-3 space-y-1.5">
              {exclusions.map((e) => (
                <li
                  key={e.id}
                  className="flex items-center justify-between rounded-md border border-border bg-bg px-3 py-2 text-body"
                >
                  <span className="text-text">
                    <span className="text-text-muted">
                      {e.match_type === "app" ? "App is " : "Title contains "}
                    </span>
                    {e.pattern}
                  </span>
                  <button
                    type="button"
                    onClick={() => removeEx(e.id)}
                    className="text-text-muted hover:text-negative"
                    aria-label={`Remove exclusion ${e.pattern}`}
                  >
                    <Trash2 className="h-4 w-4" aria-hidden />
                  </button>
                </li>
              ))}
            </ul>
          ) : null}
        </div>

        <Row
          title="Export your data"
          description="A full copy of your events. Stays on your machine."
          control={
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => doExport("csv")}
                className="flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2"
              >
                <Download className="h-4 w-4" aria-hidden /> CSV
              </button>
              <button
                type="button"
                onClick={() => doExport("json")}
                className="flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2"
              >
                <Download className="h-4 w-4" aria-hidden /> JSON
              </button>
            </div>
          }
        />
        <Row
          title="Import data"
          description="Merge a JSON export from another computer."
          control={
            <button
              type="button"
              onClick={doImport}
              className="flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2"
            >
              <Upload className="h-4 w-4" aria-hidden /> Import
            </button>
          }
        />
      </Section>

      <Section title="Danger zone">
        <Row
          title="Delete all data"
          description="Permanently remove every event, app, and exclusion."
          control={
            <button
              type="button"
              onClick={doWipe}
              className={cx(
                "flex items-center gap-1.5 rounded-md border border-negative/50 px-3 py-1.5 text-body-strong text-negative",
                "hover:bg-negative/10",
              )}
            >
              <AlertTriangle className="h-4 w-4" aria-hidden /> Delete everything
            </button>
          }
        />
      </Section>
    </div>
  );
}
