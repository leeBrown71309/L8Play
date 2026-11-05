import { useLingui } from "@lingui/react/macro";
import { useId } from "react";

import { StatusBadge } from "@/components/ds/status-badge";
import { PlatformIcon, usePlatformNames } from "@/components/game/PlatformTag";
import { Button } from "@/components/ui/button";
import { SettingGroup } from "@/features/settings/setting-blocks";
import type { Platform } from "@/lib/bindings/Platform";

type StoreId = Extract<Platform, "steam" | "epic" | "xbox" | "gog">;

interface StoreCardProps {
  store: StoreId;
  name: string;
  description: string;
  onSetUpSteam: () => void;
}

/** One store: mark and name, its status, what connecting will do, and an inert Connect button. */
function StoreCard({ store, name, description, onSetUpSteam }: StoreCardProps) {
  const { t } = useLingui();
  const soonId = useId();
  return (
    <div className="st-row st-store" role="group" aria-label={name}>
      <span className="st-store-mark">
        <PlatformIcon platform={store} size={28} />
      </span>
      <div className="grow">
        <div className="t st-store-title">
          {name}
          {store === "steam" ? (
            <StatusBadge status="info" size="sm">{t`Available with an API key`}</StatusBadge>
          ) : (
            <StatusBadge status="offline" size="sm">{t`Not connected`}</StatusBadge>
          )}
        </div>
        <div className="d">{description}</div>
      </div>
      <div className="controls">
        {store === "steam" ? (
          <Button size="sm" onClick={onSetUpSteam}>
            {t`Set up`}
          </Button>
        ) : (
          <>
            <StatusBadge status="info" size="sm" className="st-soon">
              <span id={soonId}>{t`Coming soon`}</span>
            </StatusBadge>
            <Button size="sm" variant="primary" disabled aria-describedby={soonId}>
              {t`Connect`}
            </Button>
          </>
        )}
      </div>
    </div>
  );
}

/** Interface only: connecting a store account is not available yet, so nothing here talks to a network. */
export function StoreAccountsBlock({ onSetUpSteam }: { onSetUpSteam: () => void }) {
  const { t } = useLingui();
  const names = usePlatformNames();
  return (
    <SettingGroup title={t`Store accounts`}>
      <p className="st-store-intro">
        {t`Connect a store to list every game you own, installed or not, and open its install page from L8Play. Sign-in always happens on the store’s own page: L8Play never sees or stores your password.`}
      </p>
      <StoreCard
        store="steam"
        name={names.steam ?? "Steam"}
        description={t`Lists your whole Steam library. Works today with your Steam Web API key or public profile (see above).`}
        onSetUpSteam={onSetUpSteam}
      />
      <StoreCard
        store="epic"
        name={names.epic ?? "Epic Games"}
        description={t`Lists the games in your Epic library and opens the Epic Games Launcher to install them.`}
        onSetUpSteam={onSetUpSteam}
      />
      <StoreCard
        store="xbox"
        name={t`Xbox (Microsoft account)`}
        description={t`Lists your Xbox and Microsoft Store games and opens the Xbox app to install them.`}
        onSetUpSteam={onSetUpSteam}
      />
      <StoreCard
        store="gog"
        name={names.gog ?? "GOG"}
        description={t`Lists your GOG library and opens GOG Galaxy to install games.`}
        onSetUpSteam={onSetUpSteam}
      />
      <p className="st-store-foot">{t`Free: no paid service is involved. Not available yet.`}</p>
    </SettingGroup>
  );
}
