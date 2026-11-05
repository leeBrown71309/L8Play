import { placeholderGradient } from "@/components/game/GameArt";
import { useAmbient } from "@/components/shell/ambient-store";
import { useCrossfade } from "@/components/shell/use-crossfade";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { toImageSrc } from "@/lib/tauri/image-src";

/**
 * Backdrop image for a game: its cover (the dashboard card already shows the hero), then the hero
 * or header, then a gradient from the title so there is always something to blend.
 */
function backdropStyle(game: GameSummary) {
  const source = game.artwork.cover ?? game.artwork.hero ?? game.artwork.header;
  return source
    ? { backgroundImage: `url("${toImageSrc(source)}")` }
    : { backgroundImage: placeholderGradient(game.title) };
}

/** One blurred, scrimmed layer behind the page; it cross-fades when the featured game changes. */
export function AmbientBackdrop() {
  const game = useAmbient((state) => state.game);
  const { current, leaving } = useCrossfade<GameSummary>(game, (value) => value.id);
  if (!current && !leaving) return null;
  return (
    <div className="l8-ambient" aria-hidden="true">
      {leaving ? (
        <div
          key={leaving.id}
          className={current ? "l8-ambient-layer" : "l8-ambient-layer is-fading"}
          style={backdropStyle(leaving)}
        />
      ) : null}
      {current ? (
        <div key={current.id} className="l8-ambient-layer is-entering" style={backdropStyle(current)} />
      ) : null}
    </div>
  );
}
