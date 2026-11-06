import { useLingui } from "@lingui/react/macro";
import { useEffect, useRef, type ReactNode } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { Dropdown, SearchField } from "@/components/ds/form-fields";
import { usePlatformNames } from "@/components/game/PlatformTag";
import { Chip } from "@/components/ui/chip";
import { Segmented } from "@/components/ui/segmented";
import { countByInstall } from "@/features/library/filter-games";
import { useLibraryView, type InstallFilter, type LibrarySort } from "@/features/library/library-view-store";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { Platform } from "@/lib/bindings/Platform";

/** Chips follow the store order of the design; platforms with no game get no chip. */
const PLATFORM_ORDER: readonly Platform[] = ["steam", "epic", "xbox", "gog", "standalone"];

/** Search, platform chips, installed / favourites filters, sort and grid/list switch. */
export function LibraryToolbar({ games }: { games: readonly GameSummary[] }) {
  const { t } = useLingui();
  const names = usePlatformNames();
  const view = useLibraryView();
  const root = useRef<HTMLDivElement>(null);
  const setView = view.setView;

  // "/" jumps to the filter; Ctrl+G and Ctrl+L switch between grid and list.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const typing = event.target instanceof HTMLElement && /^(INPUT|TEXTAREA|SELECT)$/.test(event.target.tagName);
      if (event.key === "/" && !typing && !event.ctrlKey && !event.metaKey) {
        event.preventDefault();
        root.current?.querySelector("input")?.focus();
      } else if (event.ctrlKey && event.key.toLowerCase() === "g") {
        event.preventDefault();
        setView("grid");
      } else if (event.ctrlKey && event.key.toLowerCase() === "l") {
        event.preventDefault();
        setView("list");
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [setView]);

  const platforms: { value: Platform | "all"; label: string; count: number }[] = [
    { value: "all", label: t`All`, count: games.length },
    ...PLATFORM_ORDER.map((platform) => ({
      value: platform,
      label: platform === "epic" ? "Epic" : (names[platform] ?? platform),
      count: games.filter((game) => game.platform === platform).length,
    })),
  ];
  const installCounts = countByInstall(games, view);
  const installStates: { value: InstallFilter; label: string; icon?: ReactNode }[] = [
    { value: "all", label: t`All games` },
    { value: "installed", label: t`Installed`, icon: <DsIcon name="drive" size={16} /> },
    { value: "not-installed", label: t`Not installed`, icon: <DsIcon name="download" size={16} /> },
    // Only offered once some saves have been imported (or while it is the active filter).
    ...(installCounts["saves-only"] > 0 || view.install === "saves-only"
      ? [{ value: "saves-only" as const, label: t`Saves only`, icon: <DsIcon name="snapshot" size={16} /> }]
      : []),
  ];
  const sorts: { id: LibrarySort; label: string }[] = [
    { id: "recent", label: t`Recently played` },
    { id: "title", label: t`Title A–Z` },
    { id: "playtime", label: t`Playtime` },
    { id: "size", label: t`Install size` },
    { id: "added", label: t`Date added` },
  ];

  return (
    <div ref={root} className="mb-[22px] flex flex-wrap items-center gap-2">
      <SearchField
        placeholder={t`Filter by title`}
        width={240}
        shortcut={["/"]}
        value={view.search}
        onChange={(event) => view.setFilters({ search: event.target.value })}
      />
      <span className="mx-1.5 h-[22px] w-px bg-line-strong" />
      {platforms
        .filter((platform) => platform.value === "all" || platform.count > 0)
        .map((platform) => (
          <Chip
            key={platform.value}
            selected={view.platform === platform.value}
            count={platform.count}
            onClick={() => view.setFilters({ platform: platform.value })}
          >
            {platform.label}
          </Chip>
        ))}
      <span className="mx-1.5 h-[22px] w-px bg-line-strong" />
      <div role="group" aria-label={t`Installation`} className="flex items-center gap-2">
        {installStates.map((state) => (
          <Chip
            key={state.value}
            selected={view.install === state.value}
            count={installCounts[state.value]}
            icon={state.icon}
            onClick={() => view.setFilters({ install: state.value })}
          >
            {state.label}
          </Chip>
        ))}
      </div>
      <Chip
        selected={view.favoritesOnly}
        icon={<DsIcon name="star" size={16} />}
        onClick={() => view.setFilters({ favoritesOnly: !view.favoritesOnly })}
      >
        {t`Favourites`}
      </Chip>
      <div className="ml-auto flex items-center gap-2">
        <Dropdown
          prefix={t`Sort`}
          label={t`Sort`}
          value={view.sort}
          options={sorts}
          onChange={(sort) => view.setFilters({ sort })}
        />
        <Segmented<"grid" | "list">
          label={t`View`}
          value={view.view}
          onChange={view.setView}
          options={[
            { value: "grid", label: t`Grid view (Ctrl+G)`, icon: <DsIcon name="grid" size={18} /> },
            { value: "list", label: t`List view (Ctrl+L)`, icon: <DsIcon name="list" size={18} /> },
          ]}
        />
      </div>
    </div>
  );
}
