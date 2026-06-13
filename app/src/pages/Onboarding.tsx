import { useState } from "react";
import { ArrowRight, ShieldCheck, Activity, Sparkles } from "lucide-react";
import { setSetting } from "../lib/api";

interface Props {
  onDone: () => void;
}

interface Step {
  icon: typeof Activity;
  title: string;
  body: string;
}

const STEPS: Step[] = [
  {
    icon: Activity,
    title: "Welcome to System Trace",
    body:
      "A calm screen-time tracker for your desktop. It records the app and window you are using, detects idle time, and turns it into clear dashboards and reports - so you can understand where your time goes without guesswork.",
  },
  {
    icon: ShieldCheck,
    title: "Private by default",
    body:
      "Everything stays on this device. There is no cloud, no account, and no telemetry. Your data lives in a local SQLite database you can export, wipe, or exclude apps from at any time. Window-title capture is off by default - you can turn it on in Settings if you want.",
  },
  {
    icon: Sparkles,
    title: "Made for grown-ups",
    body:
      "Limits, focus mode, and break reminders are here when you want them - quiet when you don't. Categories are neutral by default; turn on productivity scoring in Settings if you want a Focus Score. You can pause tracking at any moment from the top bar.",
  },
];

export function Onboarding({ onDone }: Props) {
  const [stepIndex, setStepIndex] = useState(0);
  const [finishing, setFinishing] = useState(false);
  const step = STEPS[stepIndex];
  const Icon = step.icon;
  const isLast = stepIndex === STEPS.length - 1;

  async function finish() {
    setFinishing(true);
    try {
      await setSetting("onboarding_complete", "true");
    } catch {
      // Even if the save fails the user has seen the flow; let them in.
    }
    onDone();
  }

  return (
    <div className="flex h-screen items-center justify-center bg-bg px-6 text-text">
      <div className="w-full max-w-xl rounded-lg border border-border bg-surface p-8 shadow-e2 dark:shadow-e2-dark">
        <div className="flex items-center gap-3">
          <span
            className="flex h-11 w-11 items-center justify-center rounded-lg bg-accent/15 text-accent"
            aria-hidden
          >
            <Icon className="h-5 w-5" />
          </span>
          <span className="text-label uppercase tracking-widest text-text-muted">
            Step {stepIndex + 1} of {STEPS.length}
          </span>
        </div>

        <h1 className="mt-6 text-h1 text-text">{step.title}</h1>
        <p className="mt-4 text-body text-text-muted">{step.body}</p>

        <div className="mt-8 flex items-center justify-between">
          <div className="flex gap-1.5" aria-hidden>
            {STEPS.map((_, i) => (
              <span
                key={i}
                className={
                  i === stepIndex
                    ? "h-1.5 w-6 rounded-full bg-accent"
                    : "h-1.5 w-1.5 rounded-full bg-border"
                }
              />
            ))}
          </div>

          <div className="flex items-center gap-3">
            {!isLast ? (
              <button
                type="button"
                onClick={finish}
                className="rounded-md px-3 py-2 text-body text-text-muted hover:text-text"
              >
                Skip
              </button>
            ) : null}
            <button
              type="button"
              onClick={isLast ? finish : () => setStepIndex((i) => i + 1)}
              disabled={finishing}
              className="inline-flex items-center gap-2 rounded-md bg-accent px-4 py-2 text-body-strong text-white transition-colors hover:opacity-90 disabled:opacity-60"
            >
              {isLast ? "Get started" : "Next"}
              <ArrowRight className="h-4 w-4" aria-hidden />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
