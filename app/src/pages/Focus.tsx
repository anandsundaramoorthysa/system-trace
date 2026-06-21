import { useEffect, useState } from "react";
import {
  Target,
  ShieldBan,
  Hourglass,
  Plus,
  Trash2,
  Play,
  Square,
  Timer,
  Globe,
} from "lucide-react";
import { t } from "../lib/i18n";
import {
  applyWebsiteBlock,
  clearWebsiteBlock,
  getApps,
  getBlockRules,
  getFocusState,
  getLimits,
  listFocusSessions,
  removeBlockRule,
  removeLimit,
  saveFocusSession,
  setBlockRule,
  setLimit,
  startFocusSession,
  stopFocusSession,
} from "../lib/api";
import type {
  AppInfo,
  BlockKind,
  BlockRule,
  FocusSession,
  FocusState,
  LimitStrictness,
  LimitView,
} from "../lib/types";
import { Card, CardTitle, EmptyState, Toggle, cx } from "../components/ui";
import { formatDuration } from "../lib/format";

const SESSION_OPTIONS = [25, 50, 90];

/** "9:00" -> 540, defaulting to 0 on parse failure. */
function hhmmToMins(s: string): number {
  const [h, m] = s.split(":");
  return (Number(h) || 0) * 60 + (Number(m) || 0);
}
/** 540 -> "09:00". Accepts null for the empty case. */
function minsToHHMM(m: number | null | undefined): string {
  if (m == null) return "00:00";
  const h = Math.floor(m / 60)
    .toString()
    .padStart(2, "0");
  const min = (m % 60).toString().padStart(2, "0");
  return `${h}:${min}`;
}

function LimitBar({ limit }: { limit: LimitView }) {
  const ratio = limit.daily_ms > 0 ? limit.used_ms / limit.daily_ms : 0;
  const color = limit.exceeded ? "bg-negative" : ratio > 0.8 ? "bg-warning" : "bg-accent";
  return (
    <div className="h-2 w-full overflow-hidden rounded-full bg-bg">
      <div className={cx("h-full rounded-full", color)} style={{ width: `${Math.min(ratio * 100, 100)}%` }} />
    </div>
  );
}

export function Focus() {
  const [focus, setFocus] = useState<FocusState | null>(null);
  const [limits, setLimits] = useState<LimitView[]>([]);
  const [apps, setApps] = useState<AppInfo[]>([]);
  const [rules, setRules] = useState<BlockRule[]>([]);
  const [now, setNow] = useState(() => Date.now());

  const [sessionMins, setSessionMins] = useState(25);
  const [sessionNote, setSessionNote] = useState("");
  const [sessionStart, setSessionStart] = useState<number | null>(null);
  const [recentSessions, setRecentSessions] = useState<FocusSession[]>([]);
  const [limApp, setLimApp] = useState<string>("");
  const [limMins, setLimMins] = useState(60);
  const [limStrict, setLimStrict] = useState<LimitStrictness>("medium");
  const [ruleKind, setRuleKind] = useState<BlockKind>("app");
  const [rulePattern, setRulePattern] = useState("");
  const [blockMsg, setBlockMsg] = useState("");

  useEffect(() => {
    getFocusState().then(setFocus).catch(() => {});
    getLimits().then(setLimits).catch(() => {});
    getApps().then(setApps).catch(() => {});
    getBlockRules().then(setRules).catch(() => {});
    listFocusSessions(8).then(setRecentSessions).catch(() => {});
  }, []);

  // Tick the countdown every second while a focus session is running so the
  // user sees real time remaining, not just the value at session start.
  useEffect(() => {
    if (!focus?.active || !focus.ends_at_ms) return;
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [focus?.active, focus?.ends_at_ms]);

  // When the timer reaches zero, refresh the focus state in case the backend
  // has already ended the session (the focus_ended event also triggers this).
  useEffect(() => {
    if (!focus?.active || !focus.ends_at_ms) return;
    if (now >= focus.ends_at_ms) {
      getFocusState().then(setFocus).catch(() => {});
    }
  }, [now, focus?.active, focus?.ends_at_ms]);

  async function startSession() {
    setSessionStart(Date.now());
    setFocus(await startFocusSession(sessionMins));
  }
  async function stopSession() {
    // Record the completed session with its note before clearing UI state.
    const start = sessionStart;
    if (start != null) {
      try {
        await saveFocusSession(start, Date.now(), sessionNote);
        setRecentSessions(await listFocusSessions(8));
      } catch {
        // Saving the annotation is best-effort; never block ending the session.
      }
    }
    setSessionStart(null);
    setSessionNote("");
    setFocus(await stopFocusSession());
  }

  async function addLimit() {
    if (limApp === "") return;
    await setLimit({ app_id: Number(limApp), daily_ms: limMins * 60000, strictness: limStrict });
    setLimits(await getLimits());
    setLimApp("");
  }
  async function dropLimit(appId: number) {
    await removeLimit(appId);
    setLimits((l) => l.filter((x) => x.app_id !== appId));
  }

  async function addRule() {
    const p = rulePattern.trim();
    if (!p) return;
    await setBlockRule({
      id: null,
      kind: ruleKind,
      pattern: p,
      enabled: true,
      schedule_enabled: false,
      schedule_start: null,
      schedule_end: null,
    });
    setRules(await getBlockRules());
    setRulePattern("");
    setFocus(await getFocusState());
  }
  async function updateRule(rule: BlockRule, patch: Partial<BlockRule>) {
    const next: BlockRule = { ...rule, ...patch };
    await setBlockRule({
      id: next.id,
      kind: next.kind,
      pattern: next.pattern,
      enabled: next.enabled,
      schedule_enabled: next.schedule_enabled,
      schedule_start: next.schedule_start,
      schedule_end: next.schedule_end,
    });
    setRules((rs) => rs.map((r) => (r.id === rule.id ? next : r)));
    setFocus(await getFocusState());
  }
  async function toggleRule(rule: BlockRule, enabled: boolean) {
    await updateRule(rule, { enabled });
  }
  async function dropRule(id: number) {
    await removeBlockRule(id);
    setRules((rs) => rs.filter((r) => r.id !== id));
    setFocus(await getFocusState());
  }

  async function applyBlock() {
    try {
      const n = await applyWebsiteBlock();
      setBlockMsg(`Blocking ${n} site(s) system-wide.`);
    } catch (e) {
      setBlockMsg(String(e));
    }
  }
  async function clearBlock() {
    try {
      await clearWebsiteBlock();
      setBlockMsg("System block cleared.");
    } catch (e) {
      setBlockMsg(String(e));
    }
  }

  const limitedIds = new Set(limits.map((l) => l.app_id));
  const available = apps.filter((a) => !limitedIds.has(a.id));
  const remainingMs =
    focus?.active && focus.ends_at_ms ? Math.max(0, focus.ends_at_ms - now) : null;
  const countdown =
    remainingMs !== null
      ? `${Math.floor(remainingMs / 60000)
          .toString()
          .padStart(2, "0")}:${Math.floor((remainingMs % 60000) / 1000)
          .toString()
          .padStart(2, "0")}`
      : null;

  return (
    <div className="space-y-6">
      {/* Focus mode */}
      <Card className="p-5">
        <div className="flex items-center justify-between gap-4">
          <div className="flex items-start gap-3">
            <span
              className={cx(
                "flex h-10 w-10 items-center justify-center rounded-lg",
                focus?.active ? "bg-accent/15 text-accent" : "bg-bg text-text-muted",
              )}
              aria-hidden
            >
              <Target className="h-5 w-5" />
            </span>
            <div>
              <CardTitle>{t("focus.title", "Focus mode")}</CardTitle>
              <p className="text-body text-text-muted">
                {focus?.active
                  ? t("focus.active_desc", "Blocked apps will nudge you.")
                  : t("focus.off_desc", `Off. ${focus?.rules_count ?? 0} block rule(s) ready.`)}
              </p>
            </div>
          </div>

          {focus?.active ? (
            <div className="flex items-center gap-3">
              {countdown !== null ? (
                <span
                  className="rounded-md border border-border bg-bg px-3 py-1.5 font-mono text-stat tabular-nums text-text"
                  aria-label={t("focus.timer_aria", "Time remaining in focus session")}
                >
                  {countdown}
                </span>
              ) : null}
              <button
                type="button"
                onClick={stopSession}
                className="flex items-center gap-2 rounded-md border border-border bg-surface px-3 py-2 text-body-strong text-text hover:bg-surface-2"
              >
                <Square className="h-4 w-4" aria-hidden /> {t("focus.stop", "Stop")}
              </button>
            </div>
          ) : (
            <div className="flex items-center gap-2">
              <div className="inline-flex rounded-md border border-border bg-bg p-0.5">
                {SESSION_OPTIONS.map((m) => (
                  <button
                    key={m}
                    type="button"
                    onClick={() => setSessionMins(m)}
                    className={cx(
                      "rounded px-2.5 py-1 text-label",
                      sessionMins === m ? "bg-surface text-text" : "text-text-muted hover:text-text",
                    )}
                  >
                    {m}m
                  </button>
                ))}
              </div>
              <button
                type="button"
                onClick={startSession}
                className="flex items-center gap-2 rounded-md bg-accent px-3 py-2 text-body-strong text-white"
              >
                <Play className="h-4 w-4" aria-hidden /> {t("focus.start", "Start focus")}
              </button>
            </div>
          )}
        </div>

        {focus?.active ? (
          <div className="mt-4 border-t border-border pt-4">
            <label htmlFor="session-note" className="text-label text-text-muted">
              {t("focus.note_label", "What are you working on? (saved when the session ends)")}
               </label>
            <input
              id="session-note"
              value={sessionNote}
              onChange={(e) => setSessionNote(e.target.value)}
              placeholder={t("focus.note_placeholder", "e.g. deep work on the auth refactor")}
              className="mt-1.5 w-full rounded-md border border-border bg-bg px-3 py-2 text-body text-text placeholder:text-text-muted"
            />
          </div>
        ) : null}
      </Card>

      {/* Recent sessions */}
      {recentSessions.length > 0 ? (
        <div className="space-y-2">
          <CardTitle>{t("focus.recent_title", "Recent sessions")}</CardTitle>
          <Card className="p-5">
            <ul className="divide-y divide-border">
              {recentSessions.map((sess) => {
                const mins = Math.max(1, Math.round((sess.end_ms - sess.start_ms) / 60000));
                return (
                  <li key={sess.id} className="flex items-center justify-between gap-3 py-2.5">
                    <span className="min-w-0">
                      <span className="block truncate text-body-strong text-text">
                        {sess.note ?? t("focus.session_default", "Focus session")}
                      </span>
                      <span className="block text-label text-text-muted">
                        {new Date(sess.start_ms).toLocaleString(undefined, {
                          month: "short",
                          day: "numeric",
                          hour: "numeric",
                          minute: "2-digit",
                        })}
                      </span>
                    </span>
                    <span className="shrink-0 font-medium text-text">{`${mins}m`}</span>
                  </li>
                );
              })}
            </ul>
          </Card>
        </div>
      ) : null}

      {/* Daily limits */}
      <div className="space-y-2">
        <CardTitle>{t("focus.limits_title", "Daily app limits")}</CardTitle>
        <Card className="p-5">
          <div className="mb-4 flex flex-wrap items-end gap-2">
            <select
              value={limApp}
              onChange={(e) => setLimApp(e.target.value)}
              className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
            >
              <option value="">{t("focus.limit_choose", "Choose an app")}</option>
              {available.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.display_name}
                </option>
              ))}
            </select>
            <div className="flex items-center gap-1.5">
              <input
                type="number"
                min={5}
                max={1440}
                value={limMins}
                onChange={(e) => setLimMins(Number(e.target.value))}
                className="w-20 rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
              />
              <span className="text-body text-text-muted">{t("focus.limit_min_day", "min/day")}</span>
            </div>
            <select
              value={limStrict}
              onChange={(e) => setLimStrict(e.target.value as LimitStrictness)}
              className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
            >
              <option value="soft">{t("focus.limit_strict_soft", "Soft (track only)")} (track only)</option>
              <option value="medium">{t("focus.limit_strict_med", "Medium (nudge)")} (nudge)</option>
              <option value="strict">{t("focus.limit_strict_str", "Strict (strong nudge)")}</option>
            </select>
            <button
              type="button"
              onClick={addLimit}
              className="flex items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-body-strong text-white"
            >
              <Plus className="h-4 w-4" aria-hidden /> Add limit
            </button>
          </div>

          {limits.length === 0 ? (
            <EmptyState
              icon={<Hourglass className="h-7 w-7" />}
              title={t("focus.no_limits_title", "No limits yet")}
              description={t("focus.no_limits_desc","Set a daily cap on an app to get a gentle nudge when you reach it.")}
            />
          ) : (
            <ul className="space-y-4">
              {limits.map((l) => (
                <li key={l.app_id}>
                  <div className="mb-1 flex items-center justify-between gap-3 text-body">
                    <span className="flex items-center gap-2">
                      <Timer className="h-4 w-4 text-text-muted" aria-hidden />
                      <span className="font-medium text-text">{l.display_name}</span>
                      <span className="text-label text-text-muted">{l.strictness}</span>
                    </span>
                    <span className="flex items-center gap-3">
                      <span className={cx("font-medium", l.exceeded ? "text-negative" : "text-text")}>
                        {formatDuration(l.used_ms)} / {formatDuration(l.daily_ms)}
                      </span>
                      <button
                        type="button"
                        onClick={() => dropLimit(l.app_id)}
                        className="text-text-muted hover:text-negative"
                        aria-label={`Remove limit for ${l.display_name}`}
                      >
                        <Trash2 className="h-4 w-4" aria-hidden />
                      </button>
                    </span>
                  </div>
                  <LimitBar limit={l} />
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>

      {/* Block rules */}
      <div className="space-y-2">
        <CardTitle>{t("focus.block_list_title", "Block list (focus mode)")} (focus mode)</CardTitle>
        <Card className="p-5">
          <div className="mb-4 flex flex-wrap items-center gap-2">
            <select
              value={ruleKind}
              onChange={(e) => setRuleKind(e.target.value as BlockKind)}
              className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
            >
              <option value="app">{t("focus.rule_kind_app", "App")}</option>
              <option value={t("focus.rule_kind_web", "Website")}>Website</option>
            </select>
            <input
              value={rulePattern}
              onChange={(e) => setRulePattern(e.target.value)}
              placeholder={
                 ruleKind === "app" 
                   ? t("focus.rule_pattern_placeholder", "e.g. game.exe") 
                   : t("focus.rule_pattern_placeholder", "e.g. reddit.com")
  }
              className="flex-1 rounded-md border border-border bg-bg px-3 py-1.5 text-body text-text placeholder:text-text-muted"
            />
            <button
              type="button"
              onClick={addRule}
              className="flex items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-body-strong text-white"
            >
              <Plus className="h-4 w-4" aria-hidden /> {t("focus.rule_add", "Add rule")}
            </button>
          </div>

          {rules.length === 0 ? (
            <EmptyState
              icon={<ShieldBan className="h-7 w-7" />}
              title={t("focus.no_rules_title", "No block rules")}
              description={t("focus.no_rules_desc","Add apps or websites to block while focus mode is on.")}
            />
          ) : (
            <ul className="space-y-1.5">
              {rules.map((r) => (
                <li
                  key={r.id}
                  className="rounded-md border border-border bg-bg px-3 py-2 text-body"
                >
                  <div className="flex items-center justify-between gap-3">
                    <span className="flex min-w-0 items-center gap-2">
                      <span className="rounded bg-surface px-1.5 py-0.5 text-label text-text-muted">
                        {r.kind}
                      </span>
                      <span className="truncate text-text">{r.pattern}</span>
                      {r.schedule_enabled ? (
                        <span className="rounded bg-accent/15 px-1.5 py-0.5 text-label text-accent">
                          {minsToHHMM(r.schedule_start)}-{minsToHHMM(r.schedule_end)}
                        </span>
                      ) : null}
                    </span>
                    <span className="flex items-center gap-3">
                      <Toggle checked={r.enabled} onChange={(v) => toggleRule(r, v)} />
                      <button
                        type="button"
                        onClick={() => dropRule(r.id)}
                        className="text-text-muted hover:text-negative"
                        aria-label={t("focus.rule_remove_aria", `Remove rule ${r.pattern}`)}
                      >
                        <Trash2 className="h-4 w-4" aria-hidden />
                      </button>
                    </span>
                  </div>
                  <div className="mt-2 flex flex-wrap items-center gap-2 text-label text-text-muted">
                    <label className="flex items-center gap-1.5">
                      <input
                        type="checkbox"
                        checked={r.schedule_enabled}
                        onChange={(e) =>
                          updateRule(r, {
                            schedule_enabled: e.target.checked,
                            schedule_start: r.schedule_start ?? 9 * 60,
                            schedule_end: r.schedule_end ?? 17 * 60,
                          })
                        }
                      />
                     {t("focus.rule_active_between", "Active only between")} 
                    </label>
                    <input
                      type="time"
                      disabled={!r.schedule_enabled}
                      value={minsToHHMM(r.schedule_start ?? 9 * 60)}
                      onChange={(e) =>
                        updateRule(r, { schedule_start: hhmmToMins(e.target.value) })
                      }
                      className="rounded-md border border-border bg-bg px-2 py-1 text-body text-text disabled:opacity-50"
                    />
                    <span>{t("focus.rule_and", "and")}</span>
                    <input
                      type="time"
                      disabled={!r.schedule_enabled}
                      value={minsToHHMM(r.schedule_end ?? 17 * 60)}
                      onChange={(e) =>
                        updateRule(r, { schedule_end: hhmmToMins(e.target.value) })
                      }
                      className="rounded-md border border-border bg-bg px-2 py-1 text-body text-text disabled:opacity-50"
                    />
                  </div>
                </li>
              ))}
            </ul>
          )}
          <div className="mt-4 flex flex-wrap items-center gap-2">
            <button
              type="button"
              onClick={applyBlock}
              className="flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2"
            >
              <Globe className="h-4 w-4" aria-hidden /> {t("focus.apply_now", "Apply now")}
            </button>
            <button
              type="button"
              onClick={clearBlock}
              className="rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2"
            >
              {t("focus.clear_now", "Clear now")}
            </button>
            {blockMsg && <span className="text-label text-text-muted">{blockMsg}</span>}
          </div>
          <p className="mt-3 text-label text-text-muted">
            {t("focus.block_desc", "App rules nudge you when a blocked app is in front during focus mode. System-wide website blocking edits the hosts file (requires running System Trace as administrator) and now follows each rule's schedule automatically - blocks apply when a window opens and clear when it ends. The buttons above just force an immediate sync; a \"Clear now\" will re-apply within seconds if a rule is still enabled and in its active window.")}
          </p>
        </Card>
      </div>
    </div>
  );
}
