/**
 * Theme provider. Resolves the user's preference (system | dark | light) into a
 * concrete dark/light class on <html>, persists it, and reconciles with the
 * authoritative `theme` setting from the Rust core. The pre-React script in
 * index.html sets the initial class to avoid a flash.
 */

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import type { ThemePreference } from "../lib/types";
import { getSettings, setSetting } from "../lib/api";

type Resolved = "dark" | "light";

interface ThemeCtx {
  theme: ThemePreference;
  resolved: Resolved;
  setTheme: (t: ThemePreference) => void;
}

const Ctx = createContext<ThemeCtx | null>(null);
const STORAGE_KEY = "system-trace.theme";

function systemPrefersDark(): boolean {
  return (
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
  );
}

function applyTheme(pref: ThemePreference): Resolved {
  const dark = pref === "dark" || (pref === "system" && systemPrefersDark());
  document.documentElement.classList.toggle("dark", dark);
  return dark ? "dark" : "light";
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<ThemePreference>(() => {
    const stored = localStorage.getItem(STORAGE_KEY) as ThemePreference | null;
    return stored ?? "system";
  });
  const [resolved, setResolved] = useState<Resolved>(() => applyTheme(theme));

  // Load the authoritative preference from the core once.
  useEffect(() => {
    getSettings()
      .then((s) => setThemeState(s.theme))
      .catch(() => {});
  }, []);

  // Apply + persist whenever the preference changes.
  useEffect(() => {
    setResolved(applyTheme(theme));
    localStorage.setItem(STORAGE_KEY, theme);
  }, [theme]);

  // Track OS theme changes while in "system" mode.
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => {
      if (theme === "system") setResolved(applyTheme("system"));
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [theme]);

  const setTheme = useCallback((t: ThemePreference) => {
    setThemeState(t);
    setSetting("theme", t).catch(() => {});
  }, []);

  return <Ctx.Provider value={{ theme, resolved, setTheme }}>{children}</Ctx.Provider>;
}

export function useTheme(): ThemeCtx {
  const c = useContext(Ctx);
  if (!c) throw new Error("useTheme must be used within ThemeProvider");
  return c;
}
