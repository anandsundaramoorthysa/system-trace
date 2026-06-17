import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  Plus,
  Trash2,
  Download,
  Upload,
  AlertTriangle,
  Save,
  RotateCcw,
  RefreshCw,
  Keyboard,
} from "lucide-react";
import { t } from "../lib/i18n";
import { save, open } from "@tauri-apps/plugin-dialog";
import {
  addExclusion,
  backupDatabase,
  exportData,
  getExclusions,
  getHotkeyStatus,
  getSettings,
  importData,
  isTauri,
  removeExclusion,
  restoreDatabase,
  setSetting,
  wipeAllData,
} from "../lib/api";
import { checkForUpdate } from "../lib/update";
import { LANGUAGES, setLanguage } from "../lib/i18n";
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

const PALETTES: { value: string; label: string; swatch: string }[] = [
  { value: "signal", label: "Signal", swatch: "#2DD4BF" },
  { value: "slate", label: "Slate", swatch: "#94A3B8" },
  { value: "solar", label: "Solar", swatch: "#FBBF24" },
  { value: "cocoa", label: "Cocoa", swatch: "#C79A6F" },
];

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
  const { setTheme, palette, setPalette } = useTheme();
  const [s, setS] = useState<SettingsModel | null>(null);
  const [exclusions, setExclusions] = useState<Exclusion[]>([]);
  const [pattern, setPattern] = useState("");
  const [matchType, setMatchType] = useState<ExclusionMatchType>("app");
  const [status, setStatus] = useState<string | null>(null);
  const [exportFrom, setExportFrom] = useState("");
  const [exportTo, setExportTo] = useState("");
  const [hotkeyOk, setHotkeyOk] = useState(true);
  const [checking, setChecking] = useState(false);
  const statusTimer = useRef<number | null>(null);

  useEffect(() => {
    getSettings().then(setS).catch(() => {});
    getExclusions().then(setExclusions).catch(() => {});
    getHotkeyStatus().then(setHotkeyOk).catch(() => {});
    return () => {
      if (statusTimer.current !== null) window.clearTimeout(statusTimer.current);
    };
  }, []);

  // Show a toast. `sticky` keeps it up until the next flash (used for the
  // in-progress message of a slow request); otherwise it auto-dismisses. Each
  // call cancels the previous timer so a stale one can never wipe a fresh
  // message - the bug that made "Check for updates" look like it did nothing.
  function flash(msg: string, sticky = false) {
    if (statusTimer.current !== null) {
      window.clearTimeout(statusTimer.current);
      statusTimer.current = null;
    }
    setStatus(msg);
    if (!sticky) {
      statusTimer.current = window.setTimeout(() => {
        setStatus(null);
        statusTimer.current = null;
      }, 4000);
    }
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
    const from = exportFrom || null;
    const to = exportTo || null;
    const res = await exportData(format, path, from, to);
    const scope = from || to ? " for the chosen range" : "";
    flash(`Exported ${res.rows_written} rows to ${res.format.toUpperCase()}${scope}.`);
  }

  async function doBackup() {
    if (!isTauri) {
      flash("Backup is available in the desktop app.");
      return;
    }
    const picked = await save({
      defaultPath: "system-trace-backup.sqlite",
      filters: [{ name: "SQLite", extensions: ["sqlite"] }],
    });
    if (!picked) return;
    const res = await backupDatabase(picked);
    flash(`Backed up ${(res.bytes / 1024).toFixed(0)} KB.`);
  }

  async function doRestore() {
    if (!isTauri) {
      flash("Restore is available in the desktop app.");
      return;
    }
    const picked = await open({
      multiple: false,
      filters: [{ name: "SQLite", extensions: ["sqlite"] }],
    });
    if (!picked || Array.isArray(picked)) return;
    const ok = window.confirm(
      "Restore will replace your current data with the backup. Continue?",
    );
    if (!ok) return;
    try {
      await restoreDatabase(picked);
      flash("Restored. Your backup data is loaded and live now.");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      flash(`Restore failed: ${msg}`);
    }
  }

  async function doCheckUpdate() {
    if (checking) return;
    setChecking(true);
    // Sticky: this stays up for the whole request (which can take seconds) so
    // it is never cleared before the result arrives.
    flash("Checking for updates...", true);
    try {
      const info = await checkForUpdate();
      if (info.update_available) {
        flash(`Update available: v${info.latest} (you have v${info.current}).`);
      } else {
        flash(`You are on the latest version (v${info.current}).`);
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      flash(`Could not check for updates: ${msg}`);
    } finally {
      setChecking(false);
    }
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
        <div
          className="fixed bottom-6 left-1/2 z-40 -translate-x-1/2 rounded-md border border-accent/40 bg-surface px-4 py-2 text-body text-text shadow-e2 dark:shadow-e2-dark"
          role="status"
          aria-live="polite"
        >
          {status}
        </div>
      ) : null}

      <Section title={t("settings.appearance", "Appearance")}>
        <Row
          title={t("settings.theme", "Theme")}
          description={t("settings.theme_desc", "System follows your OS setting.")}
          control={
            <Segmented<ThemePreference>
              value={s.theme}
              onChange={chooseTheme}
              options={[
               { value: "system", label: t("settings.theme_system", "System") },
               { value: "light", label: t("settings.theme_light", "Light") },
               { value: "dark", label: t("settings.theme_dark", "Dark") },
              ]}
            />
          }
        />
        <Row
          title={t("settings.palette", "Accent palette")}
          description={t("settings.palette_desc", "The highlight color used across charts and buttons.")}
          control={
            <div className="flex gap-1.5" role="group" aria-label="Accent palette">
              {PALETTES.map((p) => (
                <button
                  key={p.value}
                  type="button"
                  onClick={() => setPalette(p.value)}
                  aria-pressed={palette === p.value}
                  title={p.label}
                  className={cx(
                    "flex h-8 w-8 items-center justify-center rounded-full border-2",
                    palette === p.value ? "border-text" : "border-transparent",
                  )}
                >
                  <span
                    className="h-5 w-5 rounded-full"
                    style={{ backgroundColor: p.swatch }}
                    aria-hidden
                  />
                </button>
              ))}
            </div>
          }
        />
        <Row
          title={t("settings.language", "Language")}
          description={t("settings.language_desc", "More languages are community-contributed.")}
          control={
            <select
              value={s.language}
              onChange={(e) => {
                setStr("language", e.target.value);
                setLanguage(e.target.value);
              }}
              aria-label="Language"
              className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
            >
              {LANGUAGES.map((l) => (
                <option key={l.code} value={l.code}>
                  {l.label}
                </option>
              ))}
            </select>
          }
        />
      </Section>

      <Section title={t("settings.tracking", "Tracking")}>
        <Row
          title={t("settings.idle_threshold", "Idle threshold")}
          description={t("settings.idle_desc", "Seconds of no input before time stops counting.")}
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
          title={t("settings.capture_titles", "Capture window titles")}
          description={t("settings.capture_desc", "Off by default for privacy. Titles can contain private text.")}
          control={
            <Toggle checked={s.capture_titles} onChange={(v) => setBool("capture_titles", v)} />
          }
        />
        <Row
          title={t("settings.productivity", "Productivity scoring")}
          description={t("settings.productivity_desc", "Optional. Adds a Focus Score and productive/distracting labels.")}
          control={
            <Toggle checked={s.scoring_enabled} onChange={(v) => setBool("scoring_enabled", v)} />
          }
        />
        <Row
          title={t("settings.notifications", "Summary notifications")}
          description={t("settings.notifications_desc", "A recap of your screen time: off, daily, weekly, or both. Catches up if the app was closed.")}
          control={
            <Segmented<SummaryCadence>
              options={[
                { value: "off", label:t("settings.off", "Off") },
                { value: "daily", label: t("settings.daily", "Daily") },
                { value: "weekly", label: t("settings.weekly", "Weekly") },
                { value: "both", label: t("settings.both", "Both") },
              ]}
              value={s.summary_cadence}
              onChange={(v) => setStr("summary_cadence", v)}
            />
          }
        />
      </Section>

      <Section title={t("settings.startup", "Startup")}>
        <Row
          title={t("settings.launch_at_login", "Launch at login")}
          control={
            <Toggle checked={s.launch_at_login} onChange={(v) => setBool("launch_at_login", v)} />
          }
        />
        <Row
          title={t("settings.start_minimized", "Start minimized to tray")}
          control={
            <Toggle checked={s.start_minimized} onChange={(v) => setBool("start_minimized", v)} />
          }
        />
      </Section>

      <Section title={t("settings.privacy","Privacy and data")}>
        <Row
          title={t("settings.retention", "Keep raw events for")}
          description={t("settings.retention_desc", "Daily summaries are kept forever; raw events older than this are trimmed.")}
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
              <span className="text-body text-text-muted">{t("settings.days", "days")}</span>
            </div>
          }
        />

        <div className="px-5 py-3.5">
          <div className="text-body-strong text-text">{t("settings.exclusions", "Exclusions")}</div>
          <div className="text-label text-text-muted">
            {t("settings.exclusions_desc", "Apps or window titles that are never tracked.")}
          </div>
          <div className="mt-3 flex gap-2">
            <select
              value={matchType}
              onChange={(e) => setMatchType(e.target.value as ExclusionMatchType)}
              className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
            >
              <option value="app">{t("settings.app_is", "App is")}</option>
              <option value="title_contains">{t("settings.title_contains", "Title contains")}</option>
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
              <Plus className="h-4 w-4" aria-hidden /> {t("settings.add", "Add")}
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

        <div className="px-5 py-3.5">
          <div className="text-body-strong text-text">{t("settings.export", "Export your data")}</div>
          <div className="text-label text-text-muted">
           {t("settings.export_desc", "A copy of your events. Leave the dates empty to export everything.")}
          </div>
          <div className="mt-3 flex flex-wrap items-end gap-3">
            <label className="flex flex-col gap-1">
              <span className="text-label text-text-muted">{t("settings.from", "From")}</span>
              <input
                type="date"
                value={exportFrom}
                onChange={(e) => setExportFrom(e.target.value)}
                className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-label text-text-muted">{t("settings.to", "To")}</span>
              <input
                type="date"
                value={exportTo}
                onChange={(e) => setExportTo(e.target.value)}
                className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
              />
            </label>
            <button
              type="button"
              onClick={() => doExport("csv")}
              className="flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2"
            >
              <Download className="h-4 w-4" aria-hidden /> {t("settings.csv", "CSV")}
            </button>
            <button
              type="button"
              onClick={() => doExport("json")}
              className="flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2"
            >
              <Download className="h-4 w-4" aria-hidden /> {t("settings.json", "JSON")}
            </button>
          </div>
        </div>
        <Row
          title={t("settings.import", "Import data")}
          description={t("settings.import_desc", "Merge a JSON export from another computer.")}
          control={
            <button
              type="button"
              onClick={doImport}
              className="flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2"
            >
              <Upload className="h-4 w-4" aria-hidden /> {t("settings.import_btn", "Import")}
            </button>
          }
        />
        <Row
          title={t("settings.backup_restore", "Backup and restore")}
          description={t("settings.backup_restore_desc", "A full snapshot of your local database. Restore replaces current data.")}
          control={
            <div className="flex gap-2">
              <button
                type="button"
                onClick={doBackup}
                className="flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2"
              >
                <Save className="h-4 w-4" aria-hidden />{t("settings.backup", "Backup")}
              </button>
              <button
                type="button"
                onClick={doRestore}
                className="flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2"
              >
                <RotateCcw className="h-4 w-4" aria-hidden /> {t("settings.restore", "Restore")}
              </button>
            </div>
          }
        />
      </Section>

      <Section title={t("settings.updates","Updates and shortcuts")}>
        <Row
          title={t("settings.hotkey", "Pause / resume hotkey")}
          description={
            hotkeyOk
              ? t("settings.hotkey_desc_ok","Toggle tracking from anywhere with this global shortcut.")
              : t("settings.hotkey_desc_fail","Another app is already using this shortcut, so it is unavailable. You can still pause from the top bar.")
          }
          control={
            <span
              className={cx(
                "inline-flex items-center gap-1.5 rounded-md border px-2.5 py-1.5 text-label",
                hotkeyOk
                  ? "border-border bg-bg text-text-muted"
                  : "border-negative/50 bg-negative/10 text-negative line-through",
              )}
            >
              <Keyboard className="h-4 w-4" aria-hidden /> {t("settings.hotkey_label", "Ctrl + Alt + P")}
            </span>
          }
        />
        <Row
          title={t("settings.check_update", "Check for updates")}
          description={t("settings.check_desc","Asks GitHub for the latest version. Nothing is sent.")}
          control={
            <button
              type="button"
              onClick={doCheckUpdate}
              disabled={checking}
              className={cx(
                "flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2",
                checking && "cursor-not-allowed opacity-60",
              )}
            >
              <RefreshCw
                className={cx("h-4 w-4", checking && "animate-spin")}
                aria-hidden
              />{" "}
              {checking ? t("settings.checking", "Checking...") : t("settings.check", "Check")}
            </button>
          }
        />
      </Section>

      <Section title={t("settings.danger", "Danger zone")}>
        <Row
          title={t("settings.delete_all", "Delete all data")}
          description={t("settings.delete_desc","Permanently remove every event, app, and exclusion.")}
          control={
            <button
              type="button"
              onClick={doWipe}
              className={cx(
                "flex items-center gap-1.5 rounded-md border border-negative/50 px-3 py-1.5 text-body-strong text-negative",
                "hover:bg-negative/10",
              )}
            >
              <AlertTriangle className="h-4 w-4" aria-hidden />{t("settings.delete_btn", "Delete everything")}
            </button>
          }
        />
      </Section>
    </div>
  );
}
