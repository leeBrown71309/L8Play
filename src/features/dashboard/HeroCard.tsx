import { useLingui } from "@lingui/react/macro";
import { useEffect, useState } from "react";
import { Link } from "react-router";

import { DsIcon } from "@/components/ds/ds-icon";
import { GameArt } from "@/components/game/GameArt";
import { PlatformIcon, usePlatformNames } from "@/components/game/PlatformTag";
import { useCrossfade } from "@/components/shell/use-crossfade";
import { Button } from "@/components/ui/button";
import type { GameProtection } from "@/features/library/game-protection";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { cn } from "@/lib/cn";
import { formatPlaytime, formatRelative } from "@/lib/format";

import "@/features/dashboard/dashboard-motion.css";

interface HeroCardProps {
  game: GameSummary;
  protection: GameProtection;
  running: boolean;
  loading?: boolean;
  /** Number of games the pager cycles through; hidden when 1. */
  pages: number;
  page: number;
  onPage: (page: number) => void;
  onPlay: () => void;
  /** Reports whether the pointer or keyboard focus is on the card, so autoplay can wait. */
  onPauseChange?: (paused: boolean) => void;
}

/** Coral "Continue playing" card (`l8-herocard`): chips, title, text, snapshot pill and Play over the hero art. */
export function HeroCard({
  game,
  protection,
  running,
  loading = false,
  pages,
  page,
  onPage,
  onPlay,
  onPauseChange,
}: HeroCardProps) {
  const { t, i18n } = useLingui();
  const names = usePlatformNames();
  const { leaving } = useCrossfade<GameSummary>(game, (value) => value.id);
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  useEffect(() => {
    onPauseChange?.(hovered || focused);
  }, [hovered, focused, onPauseChange]);
  const chip = running
    ? t`Playing now`
    : game.lastPlayedAt
      ? t`Played ${formatRelative(game.lastPlayedAt, i18n.locale)}`
      : t`Continue playing`;
  const where =
    protection.local && protection.cloud === true
      ? t`Your latest save is backed up on this PC and in Google Drive.`
      : protection.local
        ? t`Your latest save is backed up on this PC.`
        : t`No snapshot yet. Your save is copied each time you stop playing.`;
  const pill =
    protection.snapshotCount === 0
      ? t`No snapshot yet`
      : protection.snapshotCount === 1
        ? t`1 snapshot · protected`
        : t`${protection.snapshotCount} snapshots · protected`;

  return (
    <section
      className="l8-herocard h-[262px] min-h-0 min-[1700px]:h-[360px]"
      aria-roledescription="carousel"
      aria-label={t`Recently played`}
      onPointerEnter={() => setHovered(true)}
      onPointerLeave={() => setHovered(false)}
      onFocusCapture={() => setFocused(true)}
      onBlurCapture={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setFocused(false);
      }}
    >
      <div className="l8-herocard-art">
        {leaving ? (
          <div key={leaving.id} className="l8-hero-layer">
            <GameArt game={leaving} kind="hero" showTitle={false} />
          </div>
        ) : null}
        <div key={game.id} className={cn("l8-hero-layer", leaving && "is-entering")}>
          <GameArt game={game} kind="hero" showTitle={false} loading={loading} />
        </div>
      </div>
      <div key={game.id} className="l8-herocard-body l8-hero-body-in">
        <div className="l8-herocard-chips">
          <span className="l8-herocard-chip">
            <DsIcon name="clock" size={14} />
            {chip}
          </span>
          <span className="l8-herocard-sq" title={names[game.platform] ?? game.platform}>
            <PlatformIcon platform={game.platform} size={20} />
          </span>
          <span
            className="l8-herocard-sq"
            title={protection.snapshotCount > 0 ? t`Protected by snapshots` : t`Not protected yet`}
          >
            <DsIcon name={protection.snapshotCount > 0 ? "shieldCheck" : "alert"} size={16} />
          </span>
        </div>
        <h2 className="l8-herocard-title line-clamp-2 min-[1700px]:text-[44px] min-[1700px]:leading-[48px]">
          <Link to={`/games/${game.id}`} className="text-inherit no-underline">
            {game.title}
          </Link>
        </h2>
        <p className="l8-herocard-text">
          {t`${formatPlaytime(game.playtimeSeconds)} played so far.`} {where}
        </p>
        <div className="l8-herocard-foot">
          <Button variant="light" size="lg" icon="play" onClick={onPlay} disabled={running}>
            {t`Play`}
          </Button>
          <Link to={`/games/${game.id}`} className="l8-herocard-pill text-inherit no-underline">
            <DsIcon name="snapshot" size={15} />
            {pill}
          </Link>
        </div>
      </div>
      {pages > 1 ? (
        <div className="l8-herocard-pages">
          {Array.from({ length: pages }, (_, index) => (
            <button
              key={index}
              type="button"
              aria-label={t`Show game ${index + 1} of ${pages}`}
              aria-current={index === page}
              className="flex cursor-pointer items-center border-0 bg-transparent p-0"
              onClick={() => onPage(index)}
            >
              <i className={cn(index === page && "is-on")} />
            </button>
          ))}
        </div>
      ) : null}
    </section>
  );
}
