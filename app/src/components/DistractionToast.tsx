import { useEffect, useState } from "react";
import { AlertCircle, X, Pause } from "lucide-react";
import { onDistractionNudge, setTrackingPaused } from "../lib/api";
import type { DistractionNudge } from "../lib/types";

interface Toast {
  id: number;
  payload: DistractionNudge;
}

let nextId = 1;
const TOAST_TTL_MS = 12_000;

/**
 * Listens for distraction_nudge events from the collector and renders a calm,
 * dismissible toast. Multiple stacked nudges are shown in order, but each one
 * auto-dismisses after a short timeout so they don't pile up.
 */
export function DistractionToast() {
  const [toasts, setToasts] = useState<Toast[]>([]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onDistractionNudge((payload) => {
      const id = nextId++;
      setToasts((t) => [...t, { id, payload }]);
      window.setTimeout(
        () => setToasts((t) => t.filter((x) => x.id !== id)),
        TOAST_TTL_MS,
      );
    }).then((u) => {
      unlisten = u;
    });
    return () => unlisten?.();
  }, []);

  function dismiss(id: number) {
    setToasts((t) => t.filter((x) => x.id !== id));
  }

  async function pauseTracking(id: number) {
    try {
      await setTrackingPaused(true);
    } finally {
      dismiss(id);
    }
  }

  if (toasts.length === 0) return null;

  return (
    <div className="pointer-events-none fixed bottom-6 right-6 z-40 flex flex-col gap-2">
      {toasts.map((t) => (
        <div
          key={t.id}
          className="pointer-events-auto flex w-80 items-start gap-3 rounded-lg border border-border bg-surface px-4 py-3 shadow-e2 dark:shadow-e2-dark"
          role="status"
          aria-live="polite"
        >
          <span
            className="mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-warning/15 text-warning"
            aria-hidden
          >
            <AlertCircle className="h-4 w-4" />
          </span>
          <div className="min-w-0 flex-1">
            <p className="text-body-strong text-text">
              {t.payload.mins} min on {t.payload.app_name}
            </p>
            <p className="mt-0.5 text-label text-text-muted">
              Worth a quick break?
            </p>
            <div className="mt-2 flex gap-2">
              <button
                type="button"
                onClick={() => pauseTracking(t.id)}
                className="inline-flex items-center gap-1 rounded-md border border-border bg-bg px-2 py-1 text-label text-text-muted hover:text-text"
              >
                <Pause className="h-3 w-3" aria-hidden /> Pause
              </button>
              <button
                type="button"
                onClick={() => dismiss(t.id)}
                className="rounded-md px-2 py-1 text-label text-text-muted hover:text-text"
              >
                Dismiss
              </button>
            </div>
          </div>
          <button
            type="button"
            onClick={() => dismiss(t.id)}
            className="text-text-muted hover:text-text"
            aria-label="Close"
          >
            <X className="h-4 w-4" aria-hidden />
          </button>
        </div>
      ))}
    </div>
  );
}
