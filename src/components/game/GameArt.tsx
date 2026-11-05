import { useState } from "react";

import type { GameSummary } from "@/lib/bindings/GameSummary";
import { toImageSrc } from "@/lib/tauri/image-src";
import { cn } from "@/lib/cn";

export type ArtKind = "cover" | "hero" | "logo" | "icon";

/** Fallback chain per kind: the first URL that loads wins; covers and heroes end on generated art. */
function candidates(game: GameSummary, kind: ArtKind): string[] {
  const { cover, hero, logo, header, icon } = game.artwork;
  // An icon falls back to the cover (cropped square) so rails never show a blank circle.
  const chain = { cover: [cover], hero: [hero, header], logo: [logo], icon: [icon, cover] }[kind];
  return chain.filter((url): url is string => Boolean(url));
}

/** Stable hue pair from the title, kept inside the Ember palette's warm range. */
export function placeholderGradient(title: string): string {
  let hash = 0;
  for (const char of title) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  const hue = 345 + (hash % 50);
  return `linear-gradient(150deg, hsl(${hue % 360} 55% 42%) 0%, hsl(${(hue + 20) % 360} 45% 22%) 70%, hsl(${hue % 360} 40% 14%) 100%)`;
}

interface GameArtProps {
  game: GameSummary;
  kind: ArtKind;
  className?: string;
  /** Show the title on generated covers (off for heroes, which show the title elsewhere). */
  showTitle?: boolean;
  /** Artwork is being fetched: shimmer instead of a bare placeholder. */
  loading?: boolean | undefined;
}

/**
 * Game artwork, always cropped to fill (`object-cover`), never stretched. Missing or broken
 * images fall back to the next source, then to generated art; a missing logo renders nothing.
 */
export function GameArt({ game, kind, className, showTitle = kind === "cover", loading = false }: GameArtProps) {
  const sources = candidates(game, kind);
  const [failed, setFailed] = useState(0);
  const source = sources[failed];

  if (loading && !source) return <div aria-hidden="true" className={cn("l8-art l8-skel", className)} />;
  if (source) {
    return (
      <img
        src={toImageSrc(source)}
        alt=""
        draggable={false}
        loading="lazy"
        decoding="async"
        onError={() => setFailed((count) => count + 1)}
        className={cn(
          "l8-art",
          kind === "logo" ? "object-contain object-left-bottom" : "object-cover",
          "size-full",
          className,
        )}
      />
    );
  }
  if (kind === "logo") return null;
  return (
    <div
      aria-hidden="true"
      className={cn("l8-art relative size-full overflow-hidden", className)}
      style={{ background: placeholderGradient(game.title) }}
    >
      <div className="absolute -top-1/4 -right-1/4 size-3/4 rounded-full bg-[radial-gradient(circle,rgba(255,244,241,0.18),transparent_70%)]" />
      {showTitle && (
        <span className="absolute inset-x-3 bottom-3 line-clamp-3 text-[17px] leading-[21px] font-bold tracking-[-0.01em] text-on-stage">
          {game.title}
        </span>
      )}
    </div>
  );
}
