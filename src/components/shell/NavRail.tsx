import { useLingui } from "@lingui/react/macro";
import { useEffect, useRef, useState } from "react";
import { NavLink } from "react-router";

import { ROUTES } from "@/app/routes";
import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { Mark } from "@/components/shell/Mark";
import { useCloudSummary } from "@/components/shell/use-cloud-summary";
import { useAddGameFlow } from "@/features/library/use-library-flows";
import { useGames } from "@/features/library/use-library";
import { useSaveStatuses } from "@/features/saves/use-saves";
import { cn } from "@/lib/cn";

interface NavItem {
  to: string;
  label: string;
  shortcut: string;
  icon: DsIconName;
  count?: number | undefined;
  alert?: boolean;
}

function RailLink({ item }: { item: NavItem }) {
  return (
    <NavLink
      to={item.to}
      end={item.to === ROUTES.dashboard}
      aria-label={item.label}
      data-tip={`${item.label}  ${item.shortcut}`}
      className={({ isActive }) => cn("l8-railbtn", isActive && "is-on")}
    >
      <span className="l8-railbtn-ic">
        <DsIcon name={item.icon} size={20} />
      </span>
      <span className="l8-railbtn-label" aria-hidden="true">
        {item.label}
      </span>
      {item.count !== undefined ? (
        <span className="l8-railbtn-count" aria-hidden="true">
          {item.count}
        </span>
      ) : null}
      {item.alert ? <i className="l8-railbtn-dot" /> : null}
    </NavLink>
  );
}

interface NavRailProps {
  expanded: boolean;
  onToggle: () => void;
}

/** Floating navigation rail (`l8-rail`): mark, five sections, open/close toggle, "Add a game". */
export function NavRail({ expanded, onToggle }: NavRailProps) {
  const { t } = useLingui();
  const gameCount = useGames().data?.length;
  const snapshotCount = useSaveStatuses().data?.reduce((sum, status) => sum + status.snapshotCount, 0);
  const cloud = useCloudSummary();
  const addGame = useAddGameFlow();
  const addLabel = t`Add a game`;
  const toggleLabel = expanded ? t`Collapse the menu` : t`Expand the menu`;
  // Labels clip against the rail only while it is moving (see shell-motion.css).
  const [animating, setAnimating] = useState(false);
  const settleTimer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(settleTimer.current), []);
  const toggle = () => {
    setAnimating(true);
    window.clearTimeout(settleTimer.current);
    settleTimer.current = window.setTimeout(() => setAnimating(false), 300);
    onToggle();
  };

  const items: NavItem[] = [
    { to: ROUTES.dashboard, label: t`Dashboard`, shortcut: "Ctrl+1", icon: "home" },
    { to: ROUTES.games, label: t`My Games`, shortcut: "Ctrl+2", icon: "gamepad", count: gameCount },
    { to: ROUTES.saves, label: t`Save Manager`, shortcut: "Ctrl+3", icon: "snapshot", count: snapshotCount },
    { to: ROUTES.cloud, label: t`Cloud Storage`, shortcut: "Ctrl+4", icon: "cloud", alert: cloud.attention },
    { to: ROUTES.settings, label: t`Settings`, shortcut: "Ctrl+5", icon: "settings" },
  ];

  return (
    <nav aria-label={t`Main`} className={cn("l8-rail", expanded && "is-expanded", animating && "is-animating")}>
      <div className="l8-rail-brand">
        <Mark size={32} />
        <span className="l8-wordmark" aria-hidden="true">
          L8<b>PLAY</b>
        </span>
      </div>
      <div className="l8-rail-nav">
        {items.map((item) => (
          <RailLink key={item.to} item={item} />
        ))}
      </div>
      <div className="l8-rail-foot flex flex-col gap-4">
        <button
          type="button"
          className="l8-railbtn"
          aria-label={toggleLabel}
          aria-expanded={expanded}
          data-tip={toggleLabel}
          onClick={toggle}
        >
          <span className="l8-railbtn-ic">
            <DsIcon name="chevronRight" size={18} className={cn("transition-transform", expanded && "rotate-180")} />
          </span>
          <span className="l8-railbtn-label" aria-hidden="true">
            {toggleLabel}
          </span>
        </button>
        <button
          type="button"
          className="l8-railbtn l8-railbtn-add"
          aria-label={addLabel}
          data-tip={addLabel}
          disabled={addGame.isAdding}
          onClick={() => void addGame.start()}
        >
          <span className="l8-railbtn-ic">
            <DsIcon name="plus" size={20} />
          </span>
          <span className="l8-railbtn-label" aria-hidden="true">
            {addLabel}
          </span>
        </button>
      </div>
    </nav>
  );
}
