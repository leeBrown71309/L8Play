import { useLingui } from "@lingui/react/macro";
import { useEffect, useMemo } from "react";

import { Coverflow } from "@/features/console/components/Coverflow";
import { FilterChips } from "@/features/console/components/FilterChips";
import { GameInfo } from "@/features/console/components/GameInfo";
import { CONSOLE_FILTERS, filterConsoleGames, stepFilter } from "@/features/console/core/console-filter";
import { useConsoleLibrary } from "@/features/console/core/library-store";
import { useConsoleNav, useZoneFocus } from "@/features/console/core/nav-store";
import { useConsoleGameActions } from "@/features/console/core/use-console-play";
import { useViewInput } from "@/features/console/core/view-input";
import { consoleSound } from "@/features/console/sound";
import { useGames } from "@/features/library/use-library";

/** Coverflow of the games, the facts about the selected one, and the filter chips above. */
export function LibraryView() {
  const { t } = useLingui();
  const games = useGames().data;
  const gameOpen = useConsoleNav((state) => state.gameId !== null);
  const dispatch = useConsoleNav((state) => state.dispatch);
  const { filter, letter, selected, setFilter, setLetter, select } = useConsoleLibrary();
  const zone = useZoneFocus("library.zone", 2);
  const actions = useConsoleGameActions();

  const list = useMemo(() => filterConsoleGames(games ?? [], filter, letter), [games, filter, letter]);
  const index = Math.min(selected, Math.max(0, list.length - 1));
  const game = list[index] ?? null;

  // Keep the stored selection valid (the list shrinks when a filter changes) and tell the backdrop which game it is.
  useEffect(() => {
    select(index, game?.id ?? null);
  }, [index, game?.id, select]);

  const move = (delta: number) => {
    const next = Math.min(list.length - 1, Math.max(0, index + delta));
    if (next === index) return;
    consoleSound.play("move");
    select(next, list[next]?.id ?? null);
  };
  const openGame = () => {
    if (!game) return;
    consoleSound.play("open");
    // The panel always opens on its first action (Play).
    dispatch({ type: "setFocus", zone: "game.actions", index: 0, count: 1 });
    dispatch({ type: "openGame", gameId: game.id });
  };
  const filterIndex = CONSOLE_FILTERS.indexOf(filter);

  useViewInput((action) => {
    if (zone.index === 1) {
      if (action === "left" || action === "right") {
        const next = stepFilter(filter, action === "right" ? 1 : -1);
        if (next !== filter) {
          consoleSound.play("move");
          setFilter(next);
        }
      } else if (action === "down" || action === "confirm") {
        consoleSound.play("move");
        zone.set(0);
      }
      return;
    }
    switch (action) {
      case "left":
        return move(-1);
      case "right":
        return move(1);
      case "up":
        consoleSound.play("move");
        return zone.set(1);
      case "down":
      case "confirm":
        return openGame();
      case "secondary":
        return game ? actions.play(game) : undefined;
      default:
    }
  });

  return (
    <section className={gameOpen ? "cs-library is-dimmed" : "cs-library"} aria-label={t`Library`}>
      <FilterChips
        filter={filter}
        letter={letter}
        focused={zone.index === 1 ? filterIndex : null}
        onPick={(next) => {
          consoleSound.play("select");
          setFilter(next);
        }}
        onClearLetter={() => setLetter(null)}
      />
      {game ? (
        <GameInfo game={game} showAction={!gameOpen} onPlay={() => actions.play(game)} />
      ) : (
        <div className="cs-info">
          <h1 className="cs-title">{games ? t`No game here` : t`Loading…`}</h1>
          <p className="cs-protection">{games ? t`Nothing matches this filter. Press back to change it.` : ""}</p>
        </div>
      )}
      <Coverflow
        games={list}
        selected={index}
        onSelect={(next) => {
          consoleSound.play("move");
          zone.set(0);
          select(next, list[next]?.id ?? null);
        }}
        onActivate={openGame}
        onStep={move}
      />
    </section>
  );
}
