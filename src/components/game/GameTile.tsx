import { useLingui } from "@lingui/react/macro";
import { Link } from "react-router";

import { DsIcon } from "@/components/ds/ds-icon";
import { Protection } from "@/components/ds/storage-tags";
import { GameArt } from "@/components/game/GameArt";
import { PlatformTag } from "@/components/game/PlatformTag";
import type { GameProtection } from "@/features/library/game-protection";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { cn } from "@/lib/cn";
import { formatPlaytime, formatRelative } from "@/lib/format";

interface GameTileProps {
  game: GameSummary;
  running: boolean;
  loading?: boolean;
  protection: GameProtection;
  /** Forces the tilted, ringed look (the design's selected tile). */
  selected?: boolean;
  onPlay: (game: GameSummary) => void;
}

/** Dashboard tile (`l8-tile`): hero art, title over it, round Play, protection corner. */
export function GameTile({ game, running, loading = false, protection, selected = false, onPlay }: GameTileProps) {
  const { t, i18n } = useLingui();
  const sub = running
    ? t`Playing now`
    : `${game.lastPlayedAt ? formatRelative(game.lastPlayedAt, i18n.locale) : t`Never played`} · ${formatPlaytime(game.playtimeSeconds)}`;
  return (
    <article
      className={cn("l8-tile", selected && "is-selected", !game.installed && "is-uninstalled")}
      aria-busy={loading}
    >
      <div className="l8-tile-art">
        <GameArt game={game} kind={game.artwork.hero ? "hero" : "cover"} showTitle={false} loading={loading} />
        <Link to={`/games/${game.id}`} aria-label={game.title} className="absolute inset-0" />
        <div className="l8-tile-shade pointer-events-none" />
        <button
          type="button"
          className="l8-tile-play"
          aria-label={game.installed ? t`Play ${game.title}` : t`Install ${game.title}`}
          onClick={() => onPlay(game)}
        >
          <DsIcon name={game.installed ? "play" : "download"} size={16} />
        </button>
        <span className="l8-tile-corner pointer-events-none flex items-center gap-2">
          <PlatformTag platform={game.platform} tone="stage" />
          <Protection local={protection.local} cloud={protection.cloud} tone="stage" />
        </span>
        <div className="l8-tile-text pointer-events-none">
          <b className="line-clamp-2">{game.title}</b>
          <span className="truncate">{sub}</span>
        </div>
      </div>
    </article>
  );
}
