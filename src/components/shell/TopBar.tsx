import { useLingui } from "@lingui/react/macro";
import { useEffect, useRef, type ReactNode } from "react";
import { Link, useLocation, useNavigate } from "react-router";

import { ROUTES } from "@/app/routes";
import { DsIcon } from "@/components/ds/ds-icon";
import { SearchField } from "@/components/ds/form-fields";
import { useCloudSummary } from "@/components/shell/use-cloud-summary";
import { Tooltip } from "@/components/ui/tooltip";
import { useLibraryView } from "@/features/library/library-view-store";
import { cn } from "@/lib/cn";

/** Page heading (or greeting), global search with Ctrl K, sync state and notifications. */
export function TopBar({ heading }: { heading?: ReactNode }) {
  const { t } = useLingui();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const search = useLibraryView((state) => state.search);
  const setFilters = useLibraryView((state) => state.setFilters);
  const cloud = useCloudSummary();
  const root = useRef<HTMLElement>(null);

  // Ctrl+K focuses the search from anywhere.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.key.toLowerCase() === "k") {
        event.preventDefault();
        root.current?.querySelector("input")?.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const onSearch = (value: string) => {
    setFilters({ search: value });
    if (pathname !== ROUTES.games) void navigate(ROUTES.games);
  };

  return (
    <header ref={root} className="l8-topbar">
      <div className="l8-topbar-head">{heading}</div>
      <SearchField
        className="l8-topbar-search"
        placeholder={t`Search your games…`}
        shortcut={["Ctrl", "K"]}
        value={search}
        onChange={(event) => onSearch(event.target.value)}
      />
      <div className="l8-topbar-actions">
        <Tooltip content={cloud.label} side="bottom">
          <Link to={ROUTES.cloud} aria-label={cloud.label} className={cn("l8-roundbtn", `tone-${cloud.tone}`)}>
            <DsIcon name={cloud.icon} size={18} className={cloud.state === "syncing" ? "l8-pulse" : undefined} />
            {cloud.state !== "none" ? <i className="l8-roundbtn-dot" /> : null}
          </Link>
        </Tooltip>
        <Tooltip content={t`Notifications`} side="bottom">
          <Link
            to={cloud.attention ? ROUTES.cloud : ROUTES.dashboard}
            aria-label={t`Notifications`}
            className="l8-roundbtn"
          >
            <DsIcon name="bell" size={18} />
            {cloud.attention ? <i className="l8-roundbtn-badge" /> : null}
          </Link>
        </Tooltip>
      </div>
    </header>
  );
}
