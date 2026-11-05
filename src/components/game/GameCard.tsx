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

interface GameCardProps {
  game: GameSummary;
  running: boolean;
  /** Artwork is being fetched: the cover shimmers instead of looking empty. */
  loading?: boolean;
  protection: GameProtection;
  onPlay: (game: GameSummary) => void;
  onToggleFavorite: (game: GameSummary) => void;
}

/**
 * Library card (`l8-card`): vertical cover, title, one line of meta. Hover lifts and tilts the card
 * inside a dashed ring (the design system's signature interaction) and reveals Play, favourite,
 * platform and protection.
 */
export function GameCard({ game, running, loading = false, protection, onPlay, onToggleFavorite }: GameCardProps) {
  const { t, i18n } = useLingui();
  const meta = running
    ? t`Playing now`
    : !game.installed
      ? t`Not installed`
      : game.lastPlayedAt
        ? `${formatRelative(game.lastPlayedAt, i18n.locale)} · ${formatPlaytime(game.playtimeSeconds)}`
        : t`Never played`;

  return (
    <article className={cn("l8-card", !game.installed && "is-uninstalled")} aria-label={game.title} aria-busy={loading}>
      <div className="l8-card-frame">
        <div className="l8-card-art">
          <GameArt game={game} kind="cover" loading={loading} />
          <Link to={`/games/${game.id}`} aria-label={game.title} className="absolute inset-0" />
          <div className="l8-card-shade pointer-events-none" />
          <div className="l8-card-top pointer-events-none">
            <button
              type="button"
              className={cn("l8-card-play pointer-events-auto", !game.installed && "is-install")}
              aria-label={game.installed ? t`Play ${game.title}` : t`Install ${game.title}`}
              onClick={() => onPlay(game)}
            >
              <DsIcon name={game.installed ? "play" : "download"} size={16} />
            </button>
            <button
              type="button"
              className={cn("l8-card-fav pointer-events-auto", game.favorite && "is-on")}
              aria-pressed={game.favorite}
              aria-label={game.favorite ? t`Remove ${game.title} from favourites` : t`Add ${game.title} to favourites`}
              onClick={() => onToggleFavorite(game)}
            >
              <DsIcon name={game.favorite ? "starFill" : "star"} size={15} />
            </button>
          </div>
          {/* The store mark is always visible; protection joins it on hover, as in the design. */}
          <PlatformTag
            platform={game.platform}
            tone="stage"
            className="pointer-events-auto absolute bottom-2.5 left-2.5"
          />
          <div className="l8-card-bottom pointer-events-none justify-end">
            <Protection local={protection.local} cloud={protection.cloud} tone="stage" />
          </div>
          {!game.installed ? <span className="l8-card-flag left-[54px] h-[28px]">{t`Not installed`}</span> : null}
          {running ? (
            <span className="absolute bottom-2.5 left-[54px] inline-flex h-[28px] items-center gap-1.5 rounded-pill bg-flare px-2.5 text-[11px] font-semibold text-on-flare">
              <i className="size-1.5 animate-pulse rounded-full bg-on-flare" />
              {t`Running`}
            </span>
          ) : null}
        </div>
      </div>
      <div className="l8-card-meta">
        {loading ? (
          <>
            <div className="l8-skel l8-skel-line" />
            <div className="l8-skel l8-skel-line is-short" />
          </>
        ) : (
          <>
            <Link to={`/games/${game.id}`} className="l8-card-title block no-underline" title={game.title}>
              {game.title}
            </Link>
            <div className={cn("l8-card-sub", running && "font-semibold text-flare-text")}>{meta}</div>
          </>
        )}
      </div>
    </article>
  );
}
