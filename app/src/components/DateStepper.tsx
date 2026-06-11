import { ChevronLeft, ChevronRight, Calendar } from "lucide-react";

interface Props {
  label: string;
  onPrev: () => void;
  onNext: () => void;
  onReset: () => void;
  /** Disable the next button when the anchor is already at the present. */
  atPresent: boolean;
  /** Reset-to-present button label, e.g. "Today" / "This week" / "This month". */
  resetLabel: string;
}

/**
 * A compact "< [label] >" stepper with a reset-to-present chip. Used by the
 * Reports view to walk backwards and forwards through days, weeks, and months.
 */
export function DateStepper({ label, onPrev, onNext, onReset, atPresent, resetLabel }: Props) {
  return (
    <div className="inline-flex items-center gap-1 rounded-md border border-border bg-surface px-1 py-1 text-body">
      <button
        type="button"
        onClick={onPrev}
        className="flex h-7 w-7 items-center justify-center rounded text-text-muted transition-colors hover:bg-bg hover:text-text"
        aria-label="Previous"
      >
        <ChevronLeft className="h-4 w-4" aria-hidden />
      </button>
      <span className="flex min-w-[10rem] items-center justify-center gap-2 px-2 text-body-strong text-text">
        <Calendar className="h-3.5 w-3.5 text-text-muted" aria-hidden />
        {label}
      </span>
      <button
        type="button"
        onClick={onNext}
        disabled={atPresent}
        className="flex h-7 w-7 items-center justify-center rounded text-text-muted transition-colors hover:bg-bg hover:text-text disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-text-muted"
        aria-label="Next"
      >
        <ChevronRight className="h-4 w-4" aria-hidden />
      </button>
      <button
        type="button"
        onClick={onReset}
        disabled={atPresent}
        className="ml-1 rounded px-2 py-1 text-label text-text-muted transition-colors hover:bg-bg hover:text-text disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-text-muted"
      >
        {resetLabel}
      </button>
    </div>
  );
}
