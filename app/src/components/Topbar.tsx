import { Pause, Play, Sun, Moon, Monitor } from "lucide-react";
import type { CollectorState, ThemePreference } from "../lib/types";
import { useTheme } from "../theme/ThemeProvider";
import { cx } from "./ui";

const STATE_LABEL: Record<CollectorState, string> = {
  active: "Active",
  idle: "Idle",
  locked: "Locked",
  paused: "Paused",
};

const STATE_DOT: Record<CollectorState, string> = {
  active: "bg-positive",
  idle: "bg-warning",
  locked: "bg-text-muted",
  paused: "bg-text-muted",
};

const NEXT_THEME: Record<ThemePreference, ThemePreference> = {
  system: "light",
  light: "dark",
  dark: "system",
};

export function Topbar({
  title,
  state,
  activeApp,
  paused,
  onTogglePause,
}: {
  title: string;
  state: CollectorState;
  activeApp: string | null;
  paused: boolean;
  onTogglePause: () => void;
}) {
  const { theme, setTheme } = useTheme();
  const ThemeIcon = theme === "dark" ? Moon : theme === "light" ? Sun : Monitor;

  return (
    <header className="flex items-center justify-between border-b border-border bg-bg px-6 py-3.5">
      <h1 className="text-h2 text-text">{title}</h1>

      <div className="flex items-center gap-3">
        <span className="flex items-center gap-2 rounded-full border border-border bg-surface px-3 py-1.5 text-label text-text-muted">
          <span className={cx("h-2 w-2 rounded-full", STATE_DOT[state])} aria-hidden />
          {STATE_LABEL[state]}
          {state === "active" && activeApp ? (
            <span className="text-text">- {activeApp}</span>
          ) : null}
        </span>

        <button
          type="button"
          onClick={onTogglePause}
          className="flex items-center gap-2 rounded-md border border-border bg-surface px-3 py-1.5 text-body-strong text-text transition-colors duration-hover hover:bg-surface-2"
          aria-pressed={paused}
        >
          {paused ? <Play className="h-4 w-4" aria-hidden /> : <Pause className="h-4 w-4" aria-hidden />}
          {paused ? "Resume" : "Pause"}
        </button>

        <button
          type="button"
          onClick={() => setTheme(NEXT_THEME[theme])}
          className="rounded-md border border-border bg-surface p-2 text-text transition-colors duration-hover hover:bg-surface-2"
          aria-label={`Theme: ${theme}. Switch to ${NEXT_THEME[theme]}.`}
          title={`Theme: ${theme}`}
        >
          <ThemeIcon className="h-4 w-4" aria-hidden />
        </button>
      </div>
    </header>
  );
}
