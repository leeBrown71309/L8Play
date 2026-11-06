import { useLingui } from "@lingui/react/macro";
import { useState } from "react";
import type { ReactNode } from "react";
import { Link } from "react-router";

import { ROUTES } from "@/app/routes";
import { DsIcon } from "@/components/ds/ds-icon";
import { Protection } from "@/components/ds/storage-tags";
import { GameArt } from "@/components/game/GameArt";
import { PlatformTag } from "@/components/game/PlatformTag";
import { PlayIcon } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { IconButton } from "@/components/ui/icon-button";
import { useGameActions } from "@/features/library/use-game-actions";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { cn } from "@/lib/cn";
import { formatBytes, formatDateTime, formatPlaytime } from "@/lib/format";
import { toImageSrc } from "@/lib/tauri/image-src";

interface GameHeroProps {
  game: GameSummary;
  running: boolean;
  /** Where this game's snapshots live, for the protection indicator. */
  protection: { local: boolean; cloud: boolean };
  canSnapshot: boolean;
  onCreateSnapshot: () => void;
  tabs: ReactNode;
}

/** Logo over the hero when it loads; the title in type otherwise. */
function HeroIdentity({ game }: { game: GameSummary }) {
  const [failedLogo, setFailedLogo] = useState<string | null>(null);
  const { logo } = game.artwork;
  if (logo && failedLogo !== logo) {
    return (
      <>
        <img
          src={toImageSrc(logo)}
          alt=""
          onError={() => setFailedLogo(logo)}
          className="gd-hero-logo"
          style={{ width: 400, height: 120 }}
        />
        <h1 className="l8-sr">{game.title}</h1>
      </>
    );
  }
  return <h1 className="l8-hero-title">{game.title}</h1>;
}

/**
 * Immersive game hero (design system GameHero): hero art under a maroon scrim, the game's icon,
 * logo or title, a meta row, the Play row and the protection indicator. Same size on every tab.
 */
export function GameHero({ game, running, protection, canSnapshot, onCreateSnapshot, tabs }: GameHeroProps) {
  const { t, i18n } = useLingui();
  const actions = useGameActions();
  const hasHero = Boolean(game.artwork.hero ?? game.artwork.header);
  const fallbackCover = !hasHero && Boolean(game.artwork.cover);

  return (
    <section className={cn("l8-hero", !hasHero && "is-fallback")}>
      <div className="l8-hero-art">
        {fallbackCover ? (
          <GameArt game={game} kind="cover" showTitle={false} className="l8-hero-blur" />
        ) : (
          <GameArt game={game} kind="hero" showTitle={false} className="object-top" />
        )}
      </div>
      <div className="l8-hero-scrim" />
      <div className="l8-hero-body">
        <Link to={ROUTES.games} className="l8-hero-crumb no-underline">
          <DsIcon name="chevronLeft" size={16} />
          {t`My Games`}
        </Link>
        <div className="l8-hero-id">
          <div className="gd-hero-idrow">
            <span className="gd-hero-icon">
              <GameArt game={game} kind="icon" showTitle={false} />
            </span>
            <HeroIdentity game={game} />
          </div>
          <div className="l8-hero-meta">
            <PlatformTag platform={game.platform} tone="stage" />
            <span>
              {game.archived ? (
                <>
                  <DsIcon name="snapshot" size={14} />
                  {t`Saves only · not installed`}
                </>
              ) : game.installed ? (
                <>
                  <DsIcon name="drive" size={14} />
                  {t`Installed · ${formatBytes(game.sizeBytes, i18n.locale)}`}
                </>
              ) : (
                <>
                  <DsIcon name="cloudDown" size={14} />
                  {t`Not installed`}
                </>
              )}
            </span>
            {game.lastPlayedAt ? (
              <span>
                <DsIcon name="clock" size={14} />
                {t`Last played ${formatDateTime(game.lastPlayedAt, i18n.locale)}`}
              </span>
            ) : null}
            <span>{t`${formatPlaytime(game.playtimeSeconds)} played`}</span>
          </div>
        </div>
        <div className="l8-hero-actions">
          {running ? (
            <span className="l8-btn l8-btn-xl l8-btn-primary gd-hero-playing">
              <i />
              {t`Playing now`}
            </span>
          ) : game.archived ? null : game.installed ? (
            <Button
              variant="play"
              size="xl"
              className="l8-play"
              icon={
                <span className="grid place-items-center">
                  <PlayIcon size={18} className="l8-play-glyph" fill="currentColor" />
                </span>
              }
              onClick={() => actions.play(game)}
              disabled={actions.isLaunching(game.id)}
            >
              {t`Play`}
            </Button>
          ) : (
            <Button variant="primary" size="lg" icon="download" onClick={() => actions.play(game)}>
              {t`Install`}
            </Button>
          )}
          <Button
            variant="stage"
            size="lg"
            icon="snapshot"
            onClick={onCreateSnapshot}
            disabled={!canSnapshot || running}
            title={running ? t`Close the game first` : undefined}
          >
            {t`Create snapshot`}
          </Button>
          <IconButton
            icon={game.favorite ? "starFill" : "star"}
            label={game.favorite ? t`Remove from favourites` : t`Add to favourites`}
            variant="stage"
            size="lg"
            active={game.favorite}
            onClick={() => actions.toggleFavorite(game)}
          />
          {game.installed ? (
            <IconButton
              icon="folder"
              label={t`Open install folder`}
              variant="stage"
              size="lg"
              onClick={() => actions.openFolder(game)}
            />
          ) : null}
          <div className="l8-hero-prot">
            <Protection local={protection.local} cloud={protection.cloud} tone="stage" showLabel />
          </div>
        </div>
      </div>
      <div className="l8-hero-tabs">{tabs}</div>
    </section>
  );
}
