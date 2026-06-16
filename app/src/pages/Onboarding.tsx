import { useState } from "react";
import { ArrowRight, ShieldCheck, Activity, Sparkles, PowerCircle } from "lucide-react";
import { t } from "../lib/i18n";
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
    title: t("onboarding.step1_title", "Welcome to System Trace"),
    body: t("onboarding.step1_body",
      "A calm screen-time tracker for your desktop. It records the app and window you are using, detects idle time, and turns it into clear dashboards and reports - so you can understand where your time goes without guesswork."),
  },
  {
    icon: ShieldCheck,
    title: t("onboarding.step2_title", "Private by default"), 
    body: t("onboarding.step2_body",
      "Everything stays on this device. There is no cloud, no account, and no telemetry. Your data lives in a local SQLite database you can export, wipe, or exclude apps from at any time. Window-title capture is off by default - you can turn it on in Settings if you want."),
  },
  {
    icon: Sparkles,
    title: t("onboarding.step3_title", "Made for grown-ups"), 
    body: t("onboarding.step3_body",
      "Limits, focus mode, and break reminders are here when you want them - quiet when you don't. Categories are neutral by default; turn on productivity scoring in Settings if you want a Focus Score. You can pause tracking at any moment from the top bar."),
  },
  {
    icon: PowerCircle,
    title: t("onboarding.step4_title", "Always on, quietly"), 
    body: t("onboarding.step4_body",
      "System Trace works best when it runs in the background. With your permission, it will start when you sign in to your computer and live in the system tray. Closing the window keeps it tracking; only quitting from the tray menu stops it. You can change this any time in Settings."),
  },
];

export function Onboarding({ onDone }: Props) {
  const [stepIndex, setStepIndex] = useState(0);
  const [finishing, setFinishing] = useState(false);
  const [runOnStartup, setRunOnStartup] = useState(true);
  const step = STEPS[stepIndex];
  const Icon = step.icon;
  const isLast = stepIndex === STEPS.length - 1;

  async function finish() {
    setFinishing(true);
    try {
      // Apply the user's autostart choice. When on, also start minimized to
      // tray on boot so the window doesn't pop up on every sign-in.
      const flag = runOnStartup ? "true" : "false";
      await setSetting("launch_at_login", flag);
      await setSetting("start_minimized", flag);
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
            {t("onboarding.step_indicator", `Step ${stepIndex + 1} of ${STEPS.length}`)}
          </span>
        </div>

        <h1 className="mt-6 text-h1 text-text">{step.title}</h1>
        <p className="mt-4 text-body text-text-muted">{step.body}</p>

        {isLast ? (
          <label className="mt-5 flex cursor-pointer items-center gap-3 rounded-md border border-border bg-bg px-4 py-3">
            <input
              type="checkbox"
              checked={runOnStartup}
              onChange={(e) => setRunOnStartup(e.target.checked)}
              className="h-4 w-4 accent-accent"
            />
            <span className="text-body text-text">
              {t("onboarding.run_at_login", "Run System Trace when I sign in to my computer")}
              <span className="ml-1 text-label text-text-muted">{t("onboarding.recommended", "(recommended)")}</span>
            </span>
          </label>
        ) : null}

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
               {t("onboarding.skip", "Skip")}
              </button>
            ) : null}
            <button
              type="button"
              onClick={isLast ? finish : () => setStepIndex((i) => i + 1)}
              disabled={finishing}
              className="inline-flex items-center gap-2 rounded-md bg-accent px-4 py-2 text-body-strong text-white transition-colors hover:opacity-90 disabled:opacity-60"
            >
              {isLast ? t("onboarding.get_started", "Get started") : t("onboarding.next", "Next")}
              <ArrowRight className="h-4 w-4" aria-hidden />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
