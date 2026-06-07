import { useEffect, useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { Topbar } from "./components/Topbar";
import { Dashboard } from "./pages/Dashboard";
import { Apps } from "./pages/Apps";
import { Reports } from "./pages/Reports";
import { Focus } from "./pages/Focus";
import { Wellbeing } from "./pages/Wellbeing";
import { Settings } from "./pages/Settings";
import { BreakOverlay } from "./components/BreakOverlay";
import type { Page } from "./lib/nav";
import type { CollectorState } from "./lib/types";
import { getCollectorState, onUsageTick, setTrackingPaused } from "./lib/api";

const TITLES: Record<Page, string> = {
  dashboard: "Dashboard",
  apps: "Apps",
  reports: "Reports",
  focus: "Focus",
  wellbeing: "Wellbeing",
  settings: "Settings",
};

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [state, setState] = useState<CollectorState>("idle");
  const [activeApp, setActiveApp] = useState<string | null>(null);
  const [liveTotal, setLiveTotal] = useState<number | null>(null);

  useEffect(() => {
    getCollectorState().then(setState).catch(() => {});
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onUsageTick((t) => {
      setState(t.state);
      setActiveApp(t.active_app);
      setLiveTotal(t.total_ms);
    }).then((u) => {
      unlisten = u;
    });
    return () => unlisten?.();
  }, []);

  const paused = state === "paused";

  async function togglePause() {
    const next = await setTrackingPaused(!paused);
    setState(next);
    if (next === "paused") setActiveApp(null);
  }

  return (
    <div className="flex h-screen overflow-hidden bg-bg text-text">
      <Sidebar active={page} onNavigate={setPage} />
      <div className="flex min-w-0 flex-1 flex-col">
        <Topbar
          title={TITLES[page]}
          state={state}
          activeApp={activeApp}
          paused={paused}
          onTogglePause={togglePause}
        />
        <main className="flex-1 overflow-y-auto px-6 py-6">
          {page === "dashboard" && <Dashboard liveTotalMs={liveTotal} />}
          {page === "apps" && <Apps />}
          {page === "reports" && <Reports />}
          {page === "focus" && <Focus />}
          {page === "wellbeing" && <Wellbeing />}
          {page === "settings" && <Settings />}
        </main>
      </div>
      <BreakOverlay />
    </div>
  );
}
