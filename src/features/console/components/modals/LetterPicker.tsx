import { useLingui } from "@lingui/react/macro";
import { useMemo } from "react";

import { ModalFrame } from "@/features/console/components/modals/ModalFrame";
import { LETTERS, lettersInUse, type Letter } from "@/features/console/core/console-filter";
import { pointerMoved } from "@/features/console/core/pointer-move";
import { useConsoleLibrary } from "@/features/console/core/library-store";
import { useConsoleNav } from "@/features/console/core/nav-store";
import { useModalList } from "@/features/console/core/use-modal-list";
import { consoleSound } from "@/features/console/sound";
import { useGames } from "@/features/library/use-library";
import { cn } from "@/lib/cn";

const COLUMNS = 7;

/** Pick a first letter with the pad: a grid of A–Z and #, with an "All" cell that clears the filter. */
export function LetterPicker() {
  const { t } = useLingui();
  const dispatch = useConsoleNav((state) => state.dispatch);
  const setLetter = useConsoleLibrary((state) => state.setLetter);
  const current = useConsoleLibrary((state) => state.letter);
  const games = useGames().data;
  const used = useMemo(() => lettersInUse(games ?? []), [games]);
  const cells: (Letter | null)[] = [null, ...LETTERS];

  const close = () => dispatch({ type: "popModal" });
  const apply = (letter: Letter | null) => {
    setLetter(letter);
    close();
  };
  const list = useModalList({
    count: cells.length,
    columns: COLUMNS,
    onConfirm: (index) => apply(cells[index] ?? null),
    onBack: close,
    onSecondary: () => {
      consoleSound.play("back");
      apply(null);
    },
  });

  return (
    <ModalFrame title={t`Find a game`} subtitle={t`Jump to the games starting with a letter`} icon="search" wide>
      <div className="cs-letters" role="listbox" aria-label={t`Letters`}>
        {cells.map((letter, index) => (
          <button
            key={letter ?? "all"}
            type="button"
            role="option"
            aria-selected={current === letter}
            tabIndex={-1}
            className={cn(
              "cs-letter",
              list.index === index && "is-focused",
              current === letter && "is-on",
              letter !== null && !used.has(letter) && "is-empty",
            )}
            onPointerMove={(event) => pointerMoved(event) && list.setIndex(index)}
            onClick={() => apply(letter)}
          >
            {letter ?? t`All`}
          </button>
        ))}
      </div>
    </ModalFrame>
  );
}
