import { useRef, useState, type WheelEvent } from "react";

import { GameArt } from "@/components/game/GameArt";
import { PlatformIcon } from "@/components/game/PlatformTag";
import { coverStyleVars, coverTransform, coverWindow, type CoverWindow } from "@/features/console/core/coverflow-math";
import { useArtworkLoading } from "@/features/library/artwork-loading-store";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { cn } from "@/lib/cn";

interface CoverflowProps {
  games: readonly GameSummary[];
  selected: number;
  /** A side cover was clicked: move the selection there. */
  onSelect: (index: number) => void;
  /** The centre cover was clicked: open it. */
  onActivate: (index: number) => void;
  /** A wheel step in one direction. */
  onStep: (delta: number) => void;
}

/** Minimum time between two wheel steps, so a smooth trackpad does not skip dozens of covers. */
const WHEEL_STEP_MS = 140;

/**
 * Coverflow: the selected cover large and in front, neighbours turned towards it, smaller and darker.
 * Only the covers around the selection are rendered; each one gets its geometry as CSS custom properties
 * once per selection change, and the CSS does the animating (transform and opacity only).
 */
export function Coverflow({ games, selected, onSelect, onActivate, onStep }: CoverflowProps) {
  const loadingArt = useArtworkLoading((state) => state.pending);
  const lastWheel = useRef(0);
  const [window_, setWindow] = useState<CoverWindow | null>(null);
  const wanted = coverWindow(selected, games.length, window_);
  if (wanted !== window_) setWindow(wanted);

  const onWheel = (event: WheelEvent) => {
    const now = performance.now();
    if (now - lastWheel.current < WHEEL_STEP_MS) return;
    const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
    if (Math.abs(delta) < 4) return;
    lastWheel.current = now;
    onStep(delta > 0 ? 1 : -1);
  };

  return (
    <div className="cs-flow" onWheel={onWheel} role="listbox" aria-label="Games" aria-orientation="horizontal">
      {Array.from({ length: Math.max(0, wanted.last - wanted.first + 1) }, (_, step) => wanted.first + step).map(
        (index) => {
          const game = games[index];
          if (!game) return null;
          const offset = index - selected;
          const centre = offset === 0;
          return (
            <button
              key={game.id}
              type="button"
              role="option"
              aria-selected={centre}
              aria-label={game.title}
              tabIndex={-1}
              className={cn("cs-card", centre && "is-centre", !game.installed && "is-uninstalled")}
              style={coverStyleVars(coverTransform(offset)) as React.CSSProperties}
              onClick={() => (centre ? onActivate(index) : onSelect(index))}
            >
              <GameArt game={game} kind="cover" loading={loadingArt.has(game.id)} />
              <span className="cs-card-dim" aria-hidden="true" />
              <span className="cs-card-platform" aria-hidden="true">
                <PlatformIcon platform={game.platform} size={20} />
              </span>
            </button>
          );
        },
      )}
    </div>
  );
}
