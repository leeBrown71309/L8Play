import { useEffect, useMemo, useState } from "react";

import { placeholderGradient } from "@/components/game/GameArt";
import { useConsoleLibrary } from "@/features/console/core/library-store";
import { useGames } from "@/features/library/use-library";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { toImageSrc } from "@/lib/tauri/image-src";

/** How long the newest layer takes to fade in; older layers are dropped once it is fully opaque. */
const SETTLE_MS = 760;
/** At most this many layers stay stacked while the selection moves quickly. */
const MAX_LAYERS = 3;

interface Layer {
  /** Never reused, so React keeps each DOM node (and its running fade) for as long as the layer exists. */
  seq: number;
  gameId: number;
}

/** The game's hero (then store header, then cover) as a full-screen picture, or a gradient from its title. */
function layerStyle(game: GameSummary) {
  const source = game.artwork.hero ?? game.artwork.header ?? game.artwork.cover;
  return source
    ? { backgroundImage: `url("${toImageSrc(source)}")` }
    : { backgroundImage: placeholderGradient(game.title) };
}

/**
 * Full-screen ambience: the selected game's artwork, blurred and darkened. Every selection stacks a new layer that
 * fades in over the previous ones, which are only removed once the newest is opaque: nothing ever pops, even when
 * the selection moves faster than a fade. The slow drift lives on the one container, so layers never jump.
 */
export function ConsoleBackdrop() {
  const games = useGames().data;
  const selectedId = useConsoleLibrary((state) => state.selectedGameId);
  const byId = useMemo(() => new Map((games ?? []).map((game) => [game.id, game])), [games]);
  const [layers, setLayers] = useState<readonly Layer[]>([]);
  const [lastId, setLastId] = useState<number | null>(null);

  // A new selection stacks a layer (derived during render: the swap and its first frame are one commit).
  if (selectedId !== null && selectedId !== lastId) {
    setLastId(selectedId);
    setLayers((current) => [
      ...current.slice(1 - MAX_LAYERS),
      { seq: (current.at(-1)?.seq ?? 0) + 1, gameId: selectedId },
    ]);
  }

  useEffect(() => {
    if (layers.length < 2) return undefined;
    const timer = window.setTimeout(() => setLayers((current) => current.slice(-1)), SETTLE_MS);
    return () => window.clearTimeout(timer);
  }, [layers]);

  return (
    <div className="cs-backdrop" aria-hidden="true">
      <div className="cs-backdrop-stage">
        {layers.map((layer) => {
          const game = byId.get(layer.gameId);
          return game ? <div key={layer.seq} className="cs-backdrop-layer" style={layerStyle(game)} /> : null;
        })}
      </div>
      <div className="cs-backdrop-scrim" />
    </div>
  );
}
