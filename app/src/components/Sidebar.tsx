import { LayoutDashboard, AppWindow, BarChart3, Target, HeartPulse, Settings } from "lucide-react";
import type { Page } from "../lib/nav";
import { cx } from "./ui";

const ITEMS: Array<{ id: Page; label: string; icon: typeof LayoutDashboard }> = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "apps", label: "Apps", icon: AppWindow },
  { id: "reports", label: "Reports", icon: BarChart3 },
  { id: "focus", label: "Focus", icon: Target },
  { id: "wellbeing", label: "Wellbeing", icon: HeartPulse },
  { id: "settings", label: "Settings", icon: Settings },
];

/** The "Pulse Scope" brand mark, inline so it is theme-aware (accent var). */
function BrandMark() {
  return (
    <svg viewBox="0 0 40 40" className="h-9 w-9" aria-hidden>
      <rect
        x="0.5"
        y="0.5"
        width="39"
        height="39"
        rx="10"
        fill="var(--surface-2)"
        stroke="var(--border)"
      />
      <g clipPath="url(#sb-clip)">
        <circle cx="20" cy="20" r="8" fill="none" stroke="var(--accent)" strokeOpacity="0.3" />
        <path
          d="M6 20 H13 L15 20 L17 15 L20 26 L23 13 L26 27 L28 20 L30 20 H34"
          fill="none"
          stroke="var(--accent)"
          strokeWidth="1.6"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </g>
      <circle cx="20" cy="20" r="12.5" fill="none" stroke="var(--accent)" strokeOpacity="0.45" />
      <circle cx="23" cy="13" r="2" fill="var(--positive)" />
      <defs>
        <clipPath id="sb-clip">
          <circle cx="20" cy="20" r="12.5" />
        </clipPath>
      </defs>
    </svg>
  );
}

export function Sidebar({
  active,
  onNavigate,
}: {
  active: Page;
  onNavigate: (page: Page) => void;
}) {
  return (
    <aside className="flex w-60 shrink-0 flex-col border-r border-border bg-surface">
      <div className="flex items-center gap-2.5 px-5 py-5">
        <BrandMark />
        <div className="leading-tight">
          <div className="text-h3 font-semibold text-text">
            System <span className="text-accent">Trace</span>
          </div>
          <div className="text-label text-text-muted">Screen-time tracker</div>
        </div>
      </div>

      <nav className="flex flex-1 flex-col gap-1 px-3 py-2">
        {ITEMS.map(({ id, label, icon: Icon }) => {
          const isActive = active === id;
          return (
            <button
              key={id}
              type="button"
              onClick={() => onNavigate(id)}
              aria-current={isActive ? "page" : undefined}
              className={cx(
                "flex items-center gap-3 rounded-md px-3 py-2 text-body-strong transition-colors duration-hover",
                isActive
                  ? "bg-bg text-text"
                  : "text-text-muted hover:bg-bg hover:text-text",
              )}
            >
              <Icon
                className={cx("h-[18px] w-[18px]", isActive && "text-accent")}
                aria-hidden
              />
              {label}
            </button>
          );
        })}
      </nav>

      <div className="px-5 py-4 text-label text-text-muted">
        Local-first. Your data stays on this device.
      </div>
    </aside>
  );
}
