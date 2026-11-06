import { useLingui } from "@lingui/react/macro";

import { CONSOLE_FILTERS, type ConsoleFilter, type Letter } from "@/features/console/core/console-filter";
import { cn } from "@/lib/cn";

interface FilterChipsProps {
  filter: ConsoleFilter;
  letter: Letter | null;
  /** Index of the chip the focus is on, or null while the coverflow has it. */
  focused: number | null;
  onPick: (filter: ConsoleFilter) => void;
  onClearLetter: () => void;
}

/** The chip row above the coverflow. The focus ring follows the pad; the chosen chip stays filled. */
export function FilterChips({ filter, letter, focused, onPick, onClearLetter }: FilterChipsProps) {
  const { t } = useLingui();
  const labels: Record<ConsoleFilter, string> = {
    all: t`All games`,
    recent: t`Recent`,
    favourites: t`Favourites`,
    installed: t`Installed`,
    "not-installed": t`Not installed`,
    "saves-only": t`Saves only`,
  };
  return (
    <div className="cs-chips" role="radiogroup" aria-label={t`Filter games`}>
      {CONSOLE_FILTERS.map((id, index) => (
        <button
          key={id}
          type="button"
          role="radio"
          aria-checked={id === filter}
          tabIndex={-1}
          className={cn("cs-chip", id === filter && "is-on", focused === index && "is-focused")}
          onClick={() => onPick(id)}
        >
          {labels[id]}
        </button>
      ))}
      {letter ? (
        <button type="button" tabIndex={-1} className="cs-chip is-letter" onClick={onClearLetter}>
          {t`Starts with ${letter}`}
          <span aria-hidden="true"> ✕</span>
        </button>
      ) : null}
    </div>
  );
}
