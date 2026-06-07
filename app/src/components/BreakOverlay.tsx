import { useEffect, useState } from "react";
import { Coffee, X } from "lucide-react";
import { onBreakDue } from "../lib/api";
import type { BreakDue } from "../lib/types";

/**
 * Full-window break overlay. Shown when the collector emits `break_due`, or when
 * the Wellbeing page dispatches a "preview-break" window event. A dedicated
 * always-on-top OS window is a later enhancement; this covers the app window.
 */
export function BreakOverlay() {
  const [active, setActive] = useState<BreakDue | null>(null);
  const [remaining, setRemaining] = useState(0);

  useEffect(() => {
    let un = () => {};
    onBreakDue((b) => {
      setActive(b);
      setRemaining(b.duration_secs);
    }).then((u) => (un = u));

    const onPreview = (e: Event) => {
      const d = (e as CustomEvent<BreakDue>).detail ?? { duration_secs: 20, strict: false };
      setActive(d);
      setRemaining(d.duration_secs);
    };
    window.addEventListener("preview-break", onPreview as EventListener);
    return () => {
      un();
      window.removeEventListener("preview-break", onPreview as EventListener);
    };
  }, []);

  useEffect(() => {
    if (!active) return;
    const id = window.setInterval(() => setRemaining((r) => Math.max(0, r - 1)), 1000);
    return () => window.clearInterval(id);
  }, [active]);

  useEffect(() => {
    if (active && remaining <= 0) setActive(null);
  }, [active, remaining]);

  if (!active) return null;

  return (
    <div className="fixed inset-0 z-50 flex flex-col items-center justify-center gap-6 bg-bg/95 backdrop-blur">
      <span
        className="flex h-16 w-16 items-center justify-center rounded-full bg-accent/15 text-accent"
        aria-hidden
      >
        <Coffee className="h-8 w-8" />
      </span>
      <div className="text-center">
        <h2 className="text-h1 text-text">Time for a short break</h2>
        <p className="mt-1 text-body text-text-muted">
          Look away from the screen and rest your eyes.
        </p>
      </div>
      <div className="text-display tabular-nums text-accent">{remaining}s</div>
      <div className="flex items-center gap-3">
        {!active.strict && (
          <button
            type="button"
            onClick={() => setActive(null)}
            className="flex items-center gap-2 rounded-md border border-border bg-surface px-4 py-2 text-body-strong text-text hover:bg-surface-2"
          >
            <X className="h-4 w-4" aria-hidden /> Skip
          </button>
        )}
        <button
          type="button"
          onClick={() => setActive(null)}
          className="rounded-md bg-accent px-4 py-2 text-body-strong text-white"
        >
          Done
        </button>
      </div>
    </div>
  );
}
