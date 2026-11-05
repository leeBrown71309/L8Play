import { useLingui } from "@lingui/react/macro";
import { useMemo, useState } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { ProgressBar } from "@/components/ds/progress-bar";
import { PlatformIcon, usePlatformNames } from "@/components/game/PlatformTag";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { Platform } from "@/lib/bindings/Platform";
import type { PlatformScan } from "@/lib/bindings/PlatformScan";

const WALL_SIZE = 15;
const LAUNCHERS: readonly Platform[] = ["steam", "epic", "xbox", "gog"];

/** Offline fallback: original abstract covers shipped with the app. */
const FALLBACK_COVERS = Object.values(
  import.meta.glob<string>("@/assets/onboarding/cover-*.svg", { eager: true, query: "?url", import: "default" }),
);

/**
 * Steam app ids of well-known games whose store cover decorates the first-launch wall. They are linked from
 * Steam's public CDN (never copied into the app or the repository) and have nothing to do with the user's library.
 */
const SHOWCASE_APP_IDS = [
  814380, 1245620, 1145360, 413150, 367520, 1091500, 1086940, 504230, 1174180, 582010, 1190460, 753640, 632470, 870780,
  2622380, 1623730, 1145350, 2358720, 553850, 2379780, 1030300, 1627720,
] as const;

const coverUrl = (appId: number) => `https://cdn.cloudflare.steamstatic.com/steam/apps/${appId}/library_600x900.jpg`;

/** Fisher-Yates shuffle on a copy; the order is picked once per mount so the wall does not flicker. */
function shuffled<T>(items: readonly T[]): T[] {
  const copy = [...items];
  for (let index = copy.length - 1; index > 0; index -= 1) {
    const other = Math.floor(Math.random() * (index + 1));
    [copy[index], copy[other]] = [copy[other] as T, copy[index] as T];
  }
  return copy;
}

/** One wall tile: the real cover fades in once loaded; an abstract cover stands in if it cannot load. */
function WallTile({ appId, fallback, dimmed }: { appId: number; fallback: string; dimmed: boolean }) {
  const [state, setState] = useState<"loading" | "ready" | "failed">("loading");
  return (
    <div style={dimmed ? { opacity: 0.35 } : undefined}>
      <img
        src={state === "failed" ? fallback : coverUrl(appId)}
        alt=""
        draggable={false}
        onLoad={() => setState((current) => (current === "failed" ? current : "ready"))}
        onError={() => setState("failed")}
        className="size-full object-cover transition-opacity duration-500"
        style={{ opacity: state === "loading" ? 0 : 1 }}
      />
    </div>
  );
}

/** The rotated cover wall behind the right panel: random well-known games, independent of the user's library. */
export function CoverWall({ dimmed }: { dimmed: boolean }) {
  const tiles = useMemo(() => {
    const apps = shuffled(SHOWCASE_APP_IDS).slice(0, WALL_SIZE + 5);
    const fallbacks = shuffled(FALLBACK_COVERS);
    return apps.map((appId, index) => ({ appId, fallback: fallbacks[index % fallbacks.length] ?? "" }));
  }, []);
  return (
    <div className="ob-wall" aria-hidden="true">
      {tiles.map((tile, index) => (
        <WallTile key={tile.appId} appId={tile.appId} fallback={tile.fallback} dimmed={dimmed && index > 12} />
      ))}
    </div>
  );
}

interface ScanGlassProps {
  games: number;
  saveFolders: number;
  artworkSets: number;
  scanning: boolean;
}

/** The translucent progress card shown over the wall while the library is scanned. */
export function ScanGlass({ games, saveFolders, artworkSets, scanning }: ScanGlassProps) {
  const { t } = useLingui();
  return (
    <div className="ob-glass">
      <b>{scanning ? t`Looking for games…` : t`Found ${games} games`}</b>
      <span>{t`Matching artwork from SteamGridDB…`}</span>
      <div className="ob-stats">
        <div>
          <b>{games}</b>
          <br />
          {t`games`}
        </div>
        <div>
          <b>{saveFolders}</b>
          <br />
          {t`save folders`}
        </div>
        <div>
          <b>{artworkSets}</b>
          <br />
          {t`artwork sets`}
        </div>
      </div>
      <ProgressBar tone="cream" size="sm" {...(scanning ? { indeterminate: true } : { value: 100 })} />
    </div>
  );
}

interface PlatformListProps {
  games: readonly GameSummary[];
  scan: readonly PlatformScan[] | null;
}

/** One row per game launcher: found (with a game count) or not installed on this PC. */
export function PlatformList({ games, scan }: PlatformListProps) {
  const { t } = useLingui();
  const names = usePlatformNames();
  const platforms: Platform[] = [...LAUNCHERS];
  // Anything else the backend reports (or the library already holds) is listed too.
  for (const entry of scan ?? []) if (!platforms.includes(entry.platform)) platforms.push(entry.platform);
  for (const game of games)
    if (game.platform !== "standalone" && !platforms.includes(game.platform)) platforms.push(game.platform);
  const standalone = games.filter((game) => game.platform === "standalone").length;

  return (
    <div className="ob-list" role="list">
      {platforms.map((platform) => {
        const inLibrary = games.filter((game) => game.platform === platform).length;
        const reported = scan?.find((entry) => entry.platform === platform);
        // `found: null` means the launcher is not installed; before any scan the library count speaks.
        const present = reported ? reported.found !== null : inLibrary > 0;
        const count = reported?.found ?? inLibrary;
        return (
          <div key={platform} role="listitem" className={present ? undefined : "off"}>
            <PlatformIcon platform={platform} size={20} />
            <span className="grow">{names[platform] ?? platform}</span>
            <span className="count">
              {present ? (count === 1 ? t`1 game` : t`${count} games`) : scan ? t`Not found` : t`Not scanned yet`}
            </span>
          </div>
        );
      })}
      <div role="listitem">
        <PlatformIcon platform="standalone" size={20} />
        <span className="grow">{t`Added by hand`}</span>
        <span className="count">{standalone === 1 ? t`1 game` : t`${standalone} games`}</span>
      </div>
    </div>
  );
}

/** What is protected, in the design system's "done" list. */
export function DoneList({ items }: { items: readonly string[] }) {
  return (
    <div className="ob-list">
      {items.map((item) => (
        <div key={item}>
          <DsIcon name="checkCircle" size={20} className="ok" />
          {item}
        </div>
      ))}
    </div>
  );
}
