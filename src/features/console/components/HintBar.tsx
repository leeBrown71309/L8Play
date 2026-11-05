import { useLingui } from "@lingui/react/macro";

import { hintsFor, type HintContext, type HintIntent } from "@/features/console/core/hint-model";
import { useConsoleNav } from "@/features/console/core/nav-store";
import { actionLabel, useGamepadStatus, useInputSource } from "@/features/console/input";

/** Which hint set matches what is on screen. */
export function useHintContext(): HintContext {
  const modals = useConsoleNav((state) => state.modals);
  const gameId = useConsoleNav((state) => state.gameId);
  const section = useConsoleNav((state) => state.section);
  const filtersFocused = useConsoleNav((state) => (state.focus["library.zone"] ?? 0) === 1);
  const top = modals.at(-1);
  if (top) return top.kind === "letters" ? "letters" : "dialog";
  if (gameId !== null) return "game";
  if (section === "library") return filtersFocused ? "library-filters" : "library";
  return section === "settings" ? "settings" : "list";
}

/** The button hints at the bottom of the screen; glyphs follow the device in use (keyboard, Xbox, PlayStation). */
export function HintBar() {
  const { t } = useLingui();
  const source = useInputSource();
  const { family } = useGamepadStatus();
  const hints = hintsFor(useHintContext());

  const words: Record<HintIntent, string> = {
    select: t`Select`,
    open: t`Open`,
    play: t`Play`,
    back: t`Back`,
    exit: t`Desktop`,
    tabs: t`Sections`,
    menu: t`Menu`,
    search: t`Find`,
    adjust: t`Adjust`,
    clear: t`Clear`,
    filters: t`Filters`,
  };

  return (
    <footer className="cs-hints" aria-label={t`Controls`}>
      {hints.map((hint) => (
        <span key={`${hint.intent}-${hint.actions.join("-")}`} className="cs-hint">
          {hint.actions.map((action) => (
            <kbd key={action}>{actionLabel(action, source, family)}</kbd>
          ))}
          {words[hint.intent]}
        </span>
      ))}
    </footer>
  );
}
