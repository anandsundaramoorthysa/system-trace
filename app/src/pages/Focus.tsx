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
import {
  applyWebsiteBlock,
  clearWebsiteBlock,
  getApps,
  getBlockRules,
  getFocusState,
  getLimits,
  removeBlockRule,
  removeLimit,
  setBlockRule,
  setLimit,
  startFocusSession,
  stopFocusSession,
} from "../lib/api";
import type {
  AppInfo,
  BlockKind,
  BlockRule,
  FocusState,
  LimitStrictness,
  LimitView,
} from "../lib/types";
import { Card, CardTitle, EmptyState, Toggle, cx } from "../components/ui";
import { formatDuration } from "../lib/format";

const SESSION_OPTIONS = [25, 50, 90];

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

  const [sessionMins, setSessionMins] = useState(25);
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
  }, []);

  async function startSession() {
    setFocus(await startFocusSession(sessionMins));
  }
  async function stopSession() {
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
    await setBlockRule({ id: null, kind: ruleKind, pattern: p, enabled: true });
    setRules(await getBlockRules());
    setRulePattern("");
    setFocus(await getFocusState());
  }
  async function toggleRule(rule: BlockRule, enabled: boolean) {
    await setBlockRule({ id: rule.id, kind: rule.kind, pattern: rule.pattern, enabled });
    setRules((rs) => rs.map((r) => (r.id === rule.id ? { ...r, enabled } : r)));
    setFocus(await getFocusState());
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
  const remainingMins =
    focus?.active && focus.ends_at_ms
      ? Math.max(0, Math.round((focus.ends_at_ms - Date.now()) / 60000))
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
              <CardTitle>Focus mode</CardTitle>
              <p className="text-body text-text-muted">
                {focus?.active
                  ? remainingMins !== null
                    ? `On - about ${remainingMins} min left. Blocked apps will nudge you.`
                    : "On. Blocked apps will nudge you."
                  : `Off. ${focus?.rules_count ?? 0} block rule(s) ready.`}
              </p>
            </div>
          </div>

          {focus?.active ? (
            <button
              type="button"
              onClick={stopSession}
              className="flex items-center gap-2 rounded-md border border-border bg-surface px-3 py-2 text-body-strong text-text hover:bg-surface-2"
            >
              <Square className="h-4 w-4" aria-hidden /> Stop
            </button>
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
                <Play className="h-4 w-4" aria-hidden /> Start focus
              </button>
            </div>
          )}
        </div>
      </Card>

      {/* Daily limits */}
      <div className="space-y-2">
        <CardTitle>Daily app limits</CardTitle>
        <Card className="p-5">
          <div className="mb-4 flex flex-wrap items-end gap-2">
            <select
              value={limApp}
              onChange={(e) => setLimApp(e.target.value)}
              className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
            >
              <option value="">Choose an app</option>
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
              <span className="text-body text-text-muted">min/day</span>
            </div>
            <select
              value={limStrict}
              onChange={(e) => setLimStrict(e.target.value as LimitStrictness)}
              className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
            >
              <option value="soft">Soft (track only)</option>
              <option value="medium">Medium (nudge)</option>
              <option value="strict">Strict (strong nudge)</option>
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
              title="No limits yet"
              description="Set a daily cap on an app to get a gentle nudge when you reach it."
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
        <CardTitle>Block list (focus mode)</CardTitle>
        <Card className="p-5">
          <div className="mb-4 flex flex-wrap items-center gap-2">
            <select
              value={ruleKind}
              onChange={(e) => setRuleKind(e.target.value as BlockKind)}
              className="rounded-md border border-border bg-bg px-2 py-1.5 text-body text-text"
            >
              <option value="app">App</option>
              <option value="website">Website</option>
            </select>
            <input
              value={rulePattern}
              onChange={(e) => setRulePattern(e.target.value)}
              placeholder={ruleKind === "app" ? "e.g. game.exe" : "e.g. reddit.com"}
              className="flex-1 rounded-md border border-border bg-bg px-3 py-1.5 text-body text-text placeholder:text-text-muted"
            />
            <button
              type="button"
              onClick={addRule}
              className="flex items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-body-strong text-white"
            >
              <Plus className="h-4 w-4" aria-hidden /> Add rule
            </button>
          </div>

          {rules.length === 0 ? (
            <EmptyState
              icon={<ShieldBan className="h-7 w-7" />}
              title="No block rules"
              description="Add apps or websites to block while focus mode is on."
            />
          ) : (
            <ul className="space-y-1.5">
              {rules.map((r) => (
                <li
                  key={r.id}
                  className="flex items-center justify-between gap-3 rounded-md border border-border bg-bg px-3 py-2 text-body"
                >
                  <span className="flex min-w-0 items-center gap-2">
                    <span className="rounded bg-surface px-1.5 py-0.5 text-label text-text-muted">
                      {r.kind}
                    </span>
                    <span className="truncate text-text">{r.pattern}</span>
                  </span>
                  <span className="flex items-center gap-3">
                    <Toggle checked={r.enabled} onChange={(v) => toggleRule(r, v)} />
                    <button
                      type="button"
                      onClick={() => dropRule(r.id)}
                      className="text-text-muted hover:text-negative"
                      aria-label={`Remove rule ${r.pattern}`}
                    >
                      <Trash2 className="h-4 w-4" aria-hidden />
                    </button>
                  </span>
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
              <Globe className="h-4 w-4" aria-hidden /> Apply website block
            </button>
            <button
              type="button"
              onClick={clearBlock}
              className="rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text hover:bg-surface-2"
            >
              Clear
            </button>
            {blockMsg && <span className="text-label text-text-muted">{blockMsg}</span>}
          </div>
          <p className="mt-3 text-label text-text-muted">
            App rules nudge you when a blocked app is in front during focus mode. System-wide
            website blocking edits the hosts file and requires running System Trace as
            administrator.
          </p>
        </Card>
      </div>
    </div>
  );
}
